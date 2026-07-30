//! Adaptive freshness for committed fit snapshots.
//!
//! The decision is based on the final accepted tree, not elapsed time or the
//! number of commits used to reach it. Callers get one serializable assessment
//! and never need to reproduce freshness policy themselves.

use crate::config::ArgotConfig;
use crate::health::FitHealth;
use argot_lang::adapters::adapter_for;
use argot_lang::ext::{ext_to_lang_ctx, extension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PROFILE_SCHEMA: u32 = 1;
const WATCH_RATIO: f64 = 0.15;
const RECOMMENDED_RATIO: f64 = 0.35;
const STRONG_RATIO: f64 = 0.65;
const MATERIAL_SLICE_LINES: u64 = 1_000;
const MATERIAL_SLICE_SHARE: f64 = 0.05;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FitProfile {
    pub schema: u32,
    pub source: SourceProfile,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceProfile {
    pub files: u64,
    pub lines: u64,
    #[serde(default)]
    pub functions: u64,
    #[serde(default)]
    pub languages: BTreeMap<String, SliceProfile>,
    #[serde(default)]
    pub areas: BTreeMap<String, SliceProfile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SliceProfile {
    pub files: u64,
    pub lines: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    Ready,
    ConfigChanged,
    ProfileMissing,
    HistoryUnavailable,
    LineageDiverged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    Fresh,
    Watch,
    Recommended,
    StronglyRecommended,
}

impl Recommendation {
    pub fn notifies_check(self) -> bool {
        self >= Self::Recommended
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshReason {
    pub kind: String,
    pub level: Recommendation,
    pub changed: u64,
    pub baseline: u64,
    pub current: u64,
    pub ratio: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshAnalysis {
    pub complete: bool,
    pub changed_files: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefreshAssessment {
    pub compatibility: Compatibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<Recommendation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
    pub algorithm: String,
    pub fit_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_source_commits: Option<usize>,
    pub accepted_source_commits_at_least: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub reasons: Vec<RefreshReason>,
    pub analysis: RefreshAnalysis,
}

impl RefreshAssessment {
    fn unavailable(compatibility: Compatibility, fit_sha: &str) -> Self {
        Self {
            compatibility,
            recommendation: None,
            score: None,
            algorithm: "adaptive-v1".to_string(),
            fit_sha: fit_sha.to_string(),
            accepted_sha: None,
            accepted_source_commits: None,
            accepted_source_commits_at_least: false,
            summary: Some(match compatibility {
                Compatibility::ConfigChanged => "fit-relevant configuration changed".to_string(),
                Compatibility::ProfileMissing => {
                    "adaptive freshness profile is missing".to_string()
                }
                Compatibility::HistoryUnavailable => {
                    "fit history is unavailable in this clone".to_string()
                }
                Compatibility::LineageDiverged => {
                    "snapshot belongs to a different accepted history".to_string()
                }
                Compatibility::Ready => "refresh assessment unavailable".to_string(),
            }),
            reasons: Vec::new(),
            analysis: RefreshAnalysis {
                complete: false,
                changed_files: 0,
            },
        }
    }

    pub fn primary_reason(&self) -> Option<&RefreshReason> {
        self.reasons.first()
    }
}

impl RefreshReason {
    pub fn human_summary(&self) -> String {
        let pct = self.ratio * 100.0;
        match (self.kind.as_str(), self.scope.as_deref()) {
            ("source_turnover", _) => format!("{pct:.1}% of the fitted source surface changed"),
            ("layout_turnover", _) => format!("{pct:.1}% of the fitted file layout changed"),
            ("function_surface_turnover", _) => {
                format!("{pct:.1}% of the fitted function surface changed")
            }
            ("language_turnover", Some(scope)) => {
                format!("{pct:.1}% of the fitted {scope} source changed")
            }
            ("area_turnover", Some(scope)) => {
                format!("{pct:.1}% of the fitted {scope} area changed")
            }
            ("explicit_commit_backstop", _) => format!(
                "the explicit refresh-after backstop was reached ({} accepted source commits)",
                self.changed
            ),
            _ => format!("{pct:.1}% learned-surface drift"),
        }
    }
}

fn line_count(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        0
    } else {
        bytes.iter().filter(|b| **b == b'\n').count() as u64
            + u64::from(bytes.last() != Some(&b'\n'))
    }
}

fn area_key(rel: &str) -> String {
    let dirs: Vec<&str> = rel
        .split('/')
        .rev()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    match dirs.as_slice() {
        [] => ".".to_string(),
        [one] => (*one).to_string(),
        [one, two, ..] => format!("{one}/{two}"),
    }
}

/// Build the small denominator profile persisted by `fit`. The paths are the
/// exact files that shaped the voice; no extra repository walk is introduced.
pub fn build_fit_profile(repo: &Path, corpus_files: &[PathBuf]) -> FitProfile {
    let repo_langs = crate::corpus::repo_langs(repo);
    let mut source = SourceProfile::default();
    for path in corpus_files {
        let actual = if path.is_absolute() {
            path.clone()
        } else {
            repo.join(path)
        };
        let Ok(bytes) = std::fs::read(&actual) else {
            continue;
        };
        let rel = actual
            .strip_prefix(repo)
            .unwrap_or(&actual)
            .to_string_lossy()
            .replace('\\', "/");
        let lines = line_count(&bytes);
        source.files += 1;
        source.lines += lines;
        if let Some(language) = ext_to_lang_ctx(&extension(&rel), repo_langs) {
            let slice = source.languages.entry(language.to_string()).or_default();
            slice.files += 1;
            slice.lines += lines;
            source.functions +=
                function_bodies(&String::from_utf8_lossy(&bytes), language).len() as u64;
        }
        let slice = source.areas.entry(area_key(&rel)).or_default();
        slice.files += 1;
        slice.lines += lines;
    }
    FitProfile {
        schema: PROFILE_SCHEMA,
        source,
    }
}

#[derive(Default)]
struct DeltaSlice {
    changed: u64,
    additions: u64,
    deletions: u64,
}

#[derive(Default)]
struct FunctionDelta {
    changed: u64,
    additions: u64,
    deletions: u64,
}

fn function_bodies(source: &str, language: &str) -> BTreeMap<(String, String), usize> {
    let Some(adapter) = adapter_for(language) else {
        return BTreeMap::new();
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut bodies = BTreeMap::new();
    for callable in adapter.callable_bodies(source) {
        if callable.end_line.saturating_sub(callable.start_line) + 1 < 3 {
            continue;
        }
        let start = callable.start_line.saturating_sub(1).min(lines.len());
        let end = callable.end_line.min(lines.len());
        let body = lines[start..end].join("\n");
        *bodies.entry((callable.symbol, body)).or_default() += 1;
    }
    bodies
}

fn function_delta(old: &str, new: &str, old_language: &str, new_language: &str) -> FunctionDelta {
    let old_bodies = function_bodies(old, old_language);
    let new_bodies = function_bodies(new, new_language);
    let old_count: usize = old_bodies.values().sum();
    let new_count: usize = new_bodies.values().sum();
    let unchanged: usize = old_bodies
        .iter()
        .map(|(body, old_n)| old_n.min(new_bodies.get(body).unwrap_or(&0)))
        .sum();
    FunctionDelta {
        changed: old_count.max(new_count).saturating_sub(unchanged) as u64,
        additions: new_count.saturating_sub(unchanged) as u64,
        deletions: old_count.saturating_sub(unchanged) as u64,
    }
}

fn blob_text(repo: &git2::Repository, oid: git2::Oid) -> String {
    if oid.is_zero() {
        return String::new();
    }
    repo.find_blob(oid)
        .map(|blob| String::from_utf8_lossy(blob.content()).into_owned())
        .unwrap_or_default()
}

fn ratio(changed: u64, baseline: u64, current: u64) -> f64 {
    let denominator = baseline.max(current);
    if denominator == 0 {
        0.0
    } else {
        (changed as f64 / denominator as f64).min(1.0)
    }
}

fn level_for_ratio(value: f64) -> Recommendation {
    if value >= STRONG_RATIO {
        Recommendation::StronglyRecommended
    } else if value >= RECOMMENDED_RATIO {
        Recommendation::Recommended
    } else if value >= WATCH_RATIO {
        Recommendation::Watch
    } else {
        Recommendation::Fresh
    }
}

fn current_count(baseline: u64, additions: u64, deletions: u64) -> u64 {
    baseline.saturating_add(additions).saturating_sub(deletions)
}

fn material_slice(baseline: u64, current: u64, total: u64) -> bool {
    let size = baseline.max(current);
    size >= MATERIAL_SLICE_LINES
        || (total > 0 && size as f64 / total as f64 >= MATERIAL_SLICE_SHARE)
}

fn push_reason(
    reasons: &mut Vec<RefreshReason>,
    kind: &str,
    scope: Option<String>,
    changed: u64,
    baseline: u64,
    current: u64,
) {
    let ratio = ratio(changed, baseline, current);
    if ratio == 0.0 {
        return;
    }
    reasons.push(RefreshReason {
        kind: kind.to_string(),
        level: level_for_ratio(ratio),
        changed,
        baseline,
        current,
        ratio,
        scope,
    });
}

/// Assess the committed snapshot against the accepted tree. This is the one
/// freshness interface used by status, check, MCP, and CI.
pub fn assess(repo_path: &Path, health: &FitHealth, config: &ArgotConfig) -> RefreshAssessment {
    let Some(profile) = health.refresh_profile.as_ref() else {
        return RefreshAssessment::unavailable(Compatibility::ProfileMissing, &health.fit_sha);
    };
    if profile.schema != PROFILE_SCHEMA || health.fit_sha.is_empty() {
        return RefreshAssessment::unavailable(Compatibility::ProfileMissing, &health.fit_sha);
    }
    if health.config_fingerprint != crate::health::config_fingerprint(config) {
        return RefreshAssessment::unavailable(Compatibility::ConfigChanged, &health.fit_sha);
    }
    let Some(accepted_sha) = crate::check::freshness_anchor(&repo_path.to_string_lossy(), config)
    else {
        return RefreshAssessment::unavailable(Compatibility::HistoryUnavailable, &health.fit_sha);
    };
    let Ok(repo) = git2::Repository::open(repo_path) else {
        return RefreshAssessment::unavailable(Compatibility::HistoryUnavailable, &health.fit_sha);
    };
    let (Ok(fit_oid), Ok(accepted_oid)) = (
        git2::Oid::from_str(&health.fit_sha),
        git2::Oid::from_str(&accepted_sha),
    ) else {
        return RefreshAssessment::unavailable(Compatibility::HistoryUnavailable, &health.fit_sha);
    };
    if fit_oid != accepted_oid {
        match repo.graph_descendant_of(accepted_oid, fit_oid) {
            Ok(true) => {}
            Ok(false) => {
                return RefreshAssessment::unavailable(
                    Compatibility::LineageDiverged,
                    &health.fit_sha,
                )
            }
            Err(_) => {
                return RefreshAssessment::unavailable(
                    Compatibility::HistoryUnavailable,
                    &health.fit_sha,
                )
            }
        }
    }
    let (Ok(fit_commit), Ok(accepted_commit)) =
        (repo.find_commit(fit_oid), repo.find_commit(accepted_oid))
    else {
        return RefreshAssessment::unavailable(Compatibility::HistoryUnavailable, &health.fit_sha);
    };
    let (Ok(fit_tree), Ok(accepted_tree)) = (fit_commit.tree(), accepted_commit.tree()) else {
        return RefreshAssessment::unavailable(Compatibility::HistoryUnavailable, &health.fit_sha);
    };
    let Ok(mut diff) = repo.diff_tree_to_tree(Some(&fit_tree), Some(&accepted_tree), None) else {
        return RefreshAssessment::unavailable(Compatibility::HistoryUnavailable, &health.fit_sha);
    };
    let mut find = git2::DiffFindOptions::new();
    find.renames(true);
    let _ = diff.find_similar(Some(&mut find));

    let suppressions = config.path_suppressions();
    let repo_langs = crate::corpus::repo_langs(repo_path);
    let mut total = DeltaSlice::default();
    let mut languages: BTreeMap<String, DeltaSlice> = BTreeMap::new();
    let mut areas: BTreeMap<String, DeltaSlice> = BTreeMap::new();
    let mut changed_files = 0u64;
    let mut functions = FunctionDelta::default();
    let mut structural_files = 0u64;
    let mut file_additions = 0u64;
    let mut file_deletions = 0u64;

    for index in 0..diff.deltas().len() {
        let Some(delta) = diff.get_delta(index) else {
            continue;
        };
        let old = delta.old_file().path().and_then(Path::to_str);
        let new = delta.new_file().path().and_then(Path::to_str);
        let old_source = old.is_some_and(|p| crate::corpus::is_corpus_source(p, &suppressions));
        let new_source = new.is_some_and(|p| crate::corpus::is_corpus_source(p, &suppressions));
        if !old_source && !new_source {
            continue;
        }
        let Ok(Some(patch)) = git2::Patch::from_diff(&diff, index) else {
            continue;
        };
        let Ok((_, additions, deletions)) = patch.line_stats() else {
            continue;
        };
        let additions = additions as u64;
        let deletions = deletions as u64;
        let changed = additions.max(deletions);
        changed_files += 1;
        total.changed += changed;
        total.additions += additions;
        total.deletions += deletions;

        let old_text = if old_source {
            blob_text(&repo, delta.old_file().id())
        } else {
            String::new()
        };
        let new_text = if new_source {
            blob_text(&repo, delta.new_file().id())
        } else {
            String::new()
        };
        let old_lines = line_count(old_text.as_bytes());
        let new_lines = line_count(new_text.as_bytes());
        let old_language = old
            .filter(|_| old_source)
            .and_then(|p| ext_to_lang_ctx(&extension(p), repo_langs));
        let new_language = new
            .filter(|_| new_source)
            .and_then(|p| ext_to_lang_ctx(&extension(p), repo_langs));
        match (old_language, new_language) {
            (Some(old_language), Some(new_language)) if old_language == new_language => {
                let slice = languages.entry(old_language.to_string()).or_default();
                slice.changed += changed;
                slice.additions += additions;
                slice.deletions += deletions;
            }
            (old_language, new_language) => {
                if let Some(old_language) = old_language {
                    let slice = languages.entry(old_language.to_string()).or_default();
                    slice.changed += old_lines;
                    slice.deletions += old_lines;
                }
                if let Some(new_language) = new_language {
                    let slice = languages.entry(new_language.to_string()).or_default();
                    slice.changed += new_lines;
                    slice.additions += new_lines;
                }
            }
        }
        if let (Some(old_language), Some(new_language)) =
            (old_language.or(new_language), new_language.or(old_language))
        {
            let delta_functions = function_delta(&old_text, &new_text, old_language, new_language);
            functions.changed += delta_functions.changed;
            functions.additions += delta_functions.additions;
            functions.deletions += delta_functions.deletions;
        }
        let old_area = old.filter(|_| old_source).map(area_key);
        let new_area = new.filter(|_| new_source).map(area_key);
        match (&old_area, &new_area) {
            (Some(old_area), Some(new_area)) if old_area == new_area => {
                let slice = areas.entry(old_area.clone()).or_default();
                slice.changed += changed;
                slice.additions += additions;
                slice.deletions += deletions;
            }
            _ => {
                if let Some(old_area) = &old_area {
                    let slice = areas.entry(old_area.clone()).or_default();
                    slice.changed += old_lines;
                    slice.deletions += old_lines;
                }
                if let Some(new_area) = &new_area {
                    let slice = areas.entry(new_area.clone()).or_default();
                    slice.changed += new_lines;
                    slice.additions += new_lines;
                }
            }
        }

        match delta.status() {
            git2::Delta::Added => {
                file_additions += 1;
                structural_files += 1;
            }
            git2::Delta::Deleted => {
                file_deletions += 1;
                structural_files += 1;
            }
            git2::Delta::Renamed if old_area != new_area => {
                structural_files += 1;
            }
            _ => {}
        }
    }

    let current_lines = current_count(profile.source.lines, total.additions, total.deletions);
    let current_files = current_count(profile.source.files, file_additions, file_deletions);
    let mut reasons = Vec::new();
    push_reason(
        &mut reasons,
        "source_turnover",
        None,
        total.changed,
        profile.source.lines,
        current_lines,
    );
    push_reason(
        &mut reasons,
        "layout_turnover",
        None,
        structural_files,
        profile.source.files,
        current_files,
    );
    let current_functions = current_count(
        profile.source.functions,
        functions.additions,
        functions.deletions,
    );
    push_reason(
        &mut reasons,
        "function_surface_turnover",
        None,
        functions.changed,
        profile.source.functions,
        current_functions,
    );

    for (language, delta) in languages {
        let baseline = profile
            .source
            .languages
            .get(&language)
            .cloned()
            .unwrap_or_default();
        let current = current_count(baseline.lines, delta.additions, delta.deletions);
        if material_slice(baseline.lines, current, current_lines) {
            push_reason(
                &mut reasons,
                "language_turnover",
                Some(language),
                delta.changed,
                baseline.lines,
                current,
            );
        }
    }
    for (area, delta) in areas {
        let baseline = profile.source.areas.get(&area).cloned().unwrap_or_default();
        let current = current_count(baseline.lines, delta.additions, delta.deletions);
        if material_slice(baseline.lines, current, current_lines) {
            push_reason(
                &mut reasons,
                "area_turnover",
                Some(area),
                delta.changed,
                baseline.lines,
                current,
            );
        }
    }

    reasons.sort_by(|a, b| b.ratio.total_cmp(&a.ratio));
    let score = reasons
        .first()
        .map(|r| (r.ratio * 100.0).round().clamp(0.0, 100.0) as u8)
        .unwrap_or(0);
    let mut recommendation = level_for_ratio(score as f64 / 100.0);
    let commit_scan_limit = config
        .fit_refresh_after
        .unwrap_or(crate::check::FRESHNESS_SCAN_CAP);
    let accepted_source_commits = crate::check::accepted_source_commits_behind(
        &repo_path.to_string_lossy(),
        &health.fit_sha,
        config,
        commit_scan_limit,
    );
    let accepted_source_commits_at_least =
        accepted_source_commits.is_some_and(|count| count >= commit_scan_limit);
    if let (Some(backstop), Some(commits)) = (config.fit_refresh_after, accepted_source_commits) {
        if commits >= backstop {
            recommendation = recommendation.max(Recommendation::Recommended);
            reasons.push(RefreshReason {
                kind: "explicit_commit_backstop".to_string(),
                level: Recommendation::Recommended,
                changed: commits as u64,
                baseline: backstop as u64,
                current: commits as u64,
                ratio: 1.0,
                scope: None,
            });
        }
    }
    reasons.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then_with(|| b.ratio.total_cmp(&a.ratio))
    });
    let summary = reasons
        .first()
        .map(RefreshReason::human_summary)
        .or_else(|| Some("no material learned-surface drift".to_string()));

    RefreshAssessment {
        compatibility: Compatibility::Ready,
        recommendation: Some(recommendation),
        score: Some(score),
        algorithm: "adaptive-v1".to_string(),
        fit_sha: health.fit_sha.clone(),
        accepted_sha: Some(accepted_sha),
        accepted_source_commits,
        accepted_source_commits_at_least,
        summary,
        reasons,
        analysis: RefreshAnalysis {
            complete: true,
            changed_files,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("argot_refresh_{name}_{}", std::process::id()))
    }

    fn commit_all(repo: &git2::Repository, message: &str) -> String {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.update_all(["*"].iter(), None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let parents = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect::<Vec<_>>();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents.iter().collect::<Vec<_>>(),
        )
        .unwrap()
        .to_string()
    }

    fn source(version: usize) -> String {
        (0..100)
            .map(|i| {
                if i < version {
                    format!("value_{i} = changed_{i}\n")
                } else {
                    format!("value_{i} = {i}\n")
                }
            })
            .collect()
    }

    #[test]
    fn ratio_levels_are_conservative() {
        assert_eq!(level_for_ratio(0.149), Recommendation::Fresh);
        assert_eq!(level_for_ratio(0.15), Recommendation::Watch);
        assert_eq!(level_for_ratio(0.35), Recommendation::Recommended);
        assert_eq!(level_for_ratio(0.65), Recommendation::StronglyRecommended);
    }

    #[test]
    fn area_keys_are_stable_and_monorepo_aware() {
        assert_eq!(area_key("lib.rs"), ".");
        assert_eq!(area_key("src/lib.rs"), "src");
        assert_eq!(area_key("packages/api/src/index.ts"), "packages/api");
    }

    #[test]
    fn function_turnover_ignores_unchanged_bodies() {
        let old = "def stable():\n    value = 1\n    return value\n\ndef evolving():\n    value = 1\n    return value\n";
        let new = "def stable():\n    value = 1\n    return value\n\ndef evolving():\n    value = 2\n    return value\n\ndef added():\n    value = 3\n    return value\n";
        let delta = function_delta(old, new, "python", "python");
        assert_eq!(delta.changed, 2);
        assert_eq!(delta.additions, 2);
        assert_eq!(delta.deletions, 1);
    }

    #[test]
    fn docs_churn_is_fresh_but_one_large_source_change_is_not() {
        let dir = scratch("tree_delta");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("src/app.py"), source(0)).unwrap();
        let fit_sha = commit_all(&repo, "fit point");
        let config = ArgotConfig::default();
        let health = FitHealth {
            fit_sha,
            config_fingerprint: crate::health::config_fingerprint(&config),
            drift_candidates: Vec::new(),
            refresh_profile: Some(build_fit_profile(&dir, &[PathBuf::from("src/app.py")])),
        };

        std::fs::write(dir.join("README.md"), "many docs\n").unwrap();
        commit_all(&repo, "docs only");
        let docs = assess(&dir, &health, &config);
        assert_eq!(docs.recommendation, Some(Recommendation::Fresh));
        assert_eq!(docs.score, Some(0));

        std::fs::write(dir.join("src/app.py"), source(40)).unwrap();
        commit_all(&repo, "large refactor");
        let changed = assess(&dir, &health, &config);
        assert_eq!(changed.compatibility, Compatibility::Ready);
        assert_eq!(changed.recommendation, Some(Recommendation::Recommended));
        assert_eq!(changed.score, Some(40));
        assert!(changed
            .reasons
            .iter()
            .any(|reason| reason.kind == "source_turnover"));

        // Intermediate churn that disappears from the accepted tree must not
        // create maintenance work.
        std::fs::write(dir.join("src/app.py"), source(0)).unwrap();
        commit_all(&repo, "revert refactor");
        let reverted = assess(&dir, &health, &config);
        assert_eq!(reverted.recommendation, Some(Recommendation::Fresh));
        assert_eq!(reverted.score, Some(0));

        let mut reconfigured = config.clone();
        reconfigured.exclude.paths.push("generated/".to_string());
        assert_eq!(
            assess(&dir, &health, &reconfigured).compatibility,
            Compatibility::ConfigChanged
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn commit_backstop_is_opt_in() {
        let dir = scratch("explicit_backstop");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("src/app.py"), source(0)).unwrap();
        let fit_sha = commit_all(&repo, "fit point");
        let config = ArgotConfig::default();
        let health = FitHealth {
            fit_sha,
            config_fingerprint: crate::health::config_fingerprint(&config),
            drift_candidates: Vec::new(),
            refresh_profile: Some(build_fit_profile(&dir, &[PathBuf::from("src/app.py")])),
        };
        std::fs::write(dir.join("src/app.py"), source(1)).unwrap();
        commit_all(&repo, "tiny source change");

        let adaptive = assess(&dir, &health, &config);
        assert_eq!(adaptive.recommendation, Some(Recommendation::Fresh));

        let mut explicit = config.clone();
        explicit.fit_refresh_after = Some(1);
        let with_backstop = assess(&dir, &health, &explicit);
        // `[fit]` is not fit-relevant, so the explicit backstop does not make
        // the snapshot configuration-incompatible.
        assert_eq!(with_backstop.compatibility, Compatibility::Ready);
        assert_eq!(
            with_backstop.recommendation,
            Some(Recommendation::Recommended)
        );
        assert!(with_backstop
            .reasons
            .iter()
            .any(|reason| reason.kind == "explicit_commit_backstop"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_monorepo_local_refactor_uses_the_area_denominator() {
        let dir = scratch("monorepo_area");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("packages/api/src")).unwrap();
        std::fs::create_dir_all(dir.join("packages/web/src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("packages/api/src/app.py"), source(0)).unwrap();
        std::fs::write(dir.join("packages/web/src/app.py"), source(0)).unwrap();
        let fit_sha = commit_all(&repo, "fit point");
        let config = ArgotConfig::default();
        let health = FitHealth {
            fit_sha,
            config_fingerprint: crate::health::config_fingerprint(&config),
            drift_candidates: Vec::new(),
            refresh_profile: Some(build_fit_profile(
                &dir,
                &[
                    PathBuf::from("packages/api/src/app.py"),
                    PathBuf::from("packages/web/src/app.py"),
                ],
            )),
        };

        std::fs::write(dir.join("packages/api/src/app.py"), source(40)).unwrap();
        commit_all(&repo, "refactor api package");
        let assessment = assess(&dir, &health, &config);
        assert_eq!(assessment.recommendation, Some(Recommendation::Recommended));
        assert!(assessment.reasons.iter().any(|reason| {
            reason.kind == "area_turnover"
                && reason.scope.as_deref() == Some("packages/api")
                && (reason.ratio - 0.4).abs() < f64::EPSILON
        }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pure_language_shift_is_visible_without_line_churn() {
        let dir = scratch("language_shift");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("src/app.py"), source(0)).unwrap();
        let fit_sha = commit_all(&repo, "fit point");
        let config = ArgotConfig::default();
        let health = FitHealth {
            fit_sha,
            config_fingerprint: crate::health::config_fingerprint(&config),
            drift_candidates: Vec::new(),
            refresh_profile: Some(build_fit_profile(&dir, &[PathBuf::from("src/app.py")])),
        };

        std::fs::rename(dir.join("src/app.py"), dir.join("src/app.js")).unwrap();
        commit_all(&repo, "move source to javascript");
        let assessment = assess(&dir, &health, &config);
        assert_eq!(
            assessment.recommendation,
            Some(Recommendation::StronglyRecommended)
        );
        assert!(assessment
            .reasons
            .iter()
            .any(|reason| reason.kind == "language_turnover"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshots_without_an_adaptive_profile_require_a_deliberate_refit() {
        let config = ArgotConfig::default();
        let health = FitHealth {
            fit_sha: "legacy".to_string(),
            config_fingerprint: crate::health::config_fingerprint(&config),
            drift_candidates: Vec::new(),
            refresh_profile: None,
        };
        let assessment = assess(Path::new("."), &health, &config);
        assert_eq!(assessment.compatibility, Compatibility::ProfileMissing);
        assert_eq!(assessment.recommendation, None);
    }
}
