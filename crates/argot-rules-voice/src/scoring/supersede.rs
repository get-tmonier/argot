//! Supersession mining — learn "this repo replaces X with Y" from accepted
//! history at fit time.
//!
//! A repo mid-migration has two voices; the corpus snapshot only hears the
//! loud (old) one. This module walks the accepted first-parent history and
//! mines **replacement pairs**: an import or callee X removed while Y is
//! added in the same file of the same commit, repeatedly, across files, in
//! one direction, with X declining and Y rising since the pair first
//! appeared. Survivors become [`Supersession`] facts stored in the voice
//! artifact: the rising side stops reading as foreign, and new code written
//! against the old side raises the `superseded` rule — with the mining
//! evidence (commits, files, dates, an example sha) rendered verbatim.
//!
//! Guards (each kills a validated noise class — see
//! `docs/research/evidence/supersession-mining-probe.md`):
//! - support: ≥ `MIN_COMMITS` distinct commits and ≥ `MIN_FILES` files;
//! - asymmetry: the reverse pair must be ≪ the forward pair;
//! - trend since the pair's first commit: X net-declining AND Y net-rising —
//!   a pattern the repo still deliberately adds (a dual-era convention) is
//!   not superseded;
//! - replacement-sink: a Y absorbing several distinct X is a systematic
//!   refactor's landing zone, not a replacement; an X paired with several Y
//!   keeps only a clearly dominant one;
//! - churn caps: files rewriting many specifiers per side and commits
//!   touching many files are bulk edits, skipped for pairing (their deltas
//!   still feed the trend);
//! - leftovers: a pair whose X is gone from the corpus is a *completed*
//!   migration — nothing left to guard, dropped at attach time; callee pairs
//!   additionally require a distinctive, non-ubiquitous X (generic names
//!   like a bare `stop` would fire on unrelated code).
//!
//! No usable history (no git, shallow below the support bar) mines nothing
//! and stays silent — the guardrail is unchanged, not degraded.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use argot_lang::adapters::{adapter_for, Language, LanguageAdapter};
use argot_lang::callees::non_none_callees;
use argot_lang::ext::{ext_to_lang, extension};

/// First-parent accepted commits the miner replays. Migrations span months,
/// so this window is far wider than the integrity mini-replay's (and matches
/// audit's history cap); per-commit work is bounded by the churn caps and
/// the unique-blob extraction cache. A migration whose last activity is
/// older than the window has aged out of the repo's living voice.
pub const SUPERSEDE_WINDOW: usize = 1000;
/// Distinct commits a pair needs before it is a repo decision, not an edit.
const MIN_COMMITS: usize = 3;
/// Distinct files a pair needs — one file's churn is not a convention.
const MIN_FILES: usize = 3;
/// The forward pair must outnumber the reverse by this factor.
const ASYMMETRY: usize = 2;
/// A Y side pairing (with ≥ 2 commits) against this many distinct X is a
/// refactor sink, not a replacement.
const REPLACEMENT_SINK_CAP: usize = 3;
/// Per-file per-side specifier churn above this is a bulk rewrite — skipped
/// for pairing (cross-product noise), still counted for the trend.
const CHURN_CAP: usize = 6;
/// Commits touching more corpus files than this are mass churn — skipped
/// entirely.
const MASS_COMMIT_FILE_CAP: usize = 50;
/// Blobs above this size are skipped (same cap as the integrity replay).
const MAX_BLOB: usize = 400_000;
/// Callee names shorter than this are too generic to guard.
const CALLEE_MIN_LEN: usize = 4;
/// A callee-kind X still present in more than this fraction of corpus files
/// is polysemous or ubiquitous, not a migration leftover set.
const CALLEE_UBIQUITY_FRACTION: f64 = 0.2;
/// Absolute leftover-file cap for callee-kind pairs (same hazard).
const CALLEE_LEFTOVER_CAP: usize = 24;
/// Leftover paths listed in the artifact (the count is always exact).
pub const LEFTOVER_LIST_CAP: usize = 20;

/// Which vocabulary a supersession lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SupersessionKind {
    Import,
    Callee,
}

/// One learned (or declared) replacement fact, persisted per language in the
/// voice artifact. Evidence fields are the mining observations themselves —
/// what the finding renders, verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Supersession {
    pub old: String,
    pub new: String,
    pub kind: SupersessionKind,
    /// Distinct accepted commits that replaced `old` with `new`.
    pub commits: usize,
    /// Distinct files those commits touched.
    pub files: usize,
    /// First and last replacement dates (ISO `YYYY-MM-DD`).
    pub first: String,
    pub last: String,
    /// One replacing commit's short sha, for the evidence line.
    pub example_commit: String,
    /// Corpus files still using `old` (the migration's remaining debt).
    #[serde(default)]
    pub leftover_count: usize,
    /// Up to [`LEFTOVER_LIST_CAP`] of those paths, repo-relative.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leftovers: Vec<String>,
}

/// One changed file's specifier delta inside one commit (already reduced to
/// truly-removed / truly-added: a specifier on both sides moved, and pairs
/// with nothing).
#[derive(Debug, Clone)]
pub struct FileSpecDelta {
    pub path: String,
    pub removed: Vec<String>,
    pub added: Vec<String>,
}

/// One accepted commit's specifier deltas, one vocabulary at a time.
#[derive(Debug, Clone)]
pub struct CommitDelta {
    pub sha: String,
    /// ISO date (`YYYY-MM-DD`) of the commit.
    pub date: String,
    pub files: Vec<FileSpecDelta>,
}

/// A pair that survived every history-side guard. Leftover attachment (and
/// the corpus-side callee guards) happens against the fit corpus in
/// [`attach_leftovers`].
#[derive(Debug, Clone, PartialEq)]
pub struct MinedPair {
    pub old: String,
    pub new: String,
    pub commits: usize,
    pub files: usize,
    pub first: String,
    pub last: String,
    pub example_commit: String,
}

/// Mine replacement pairs from one vocabulary's per-commit deltas
/// (`commits` ordered oldest → newest). Pure — the git driver and the tests
/// feed it the same shape.
pub fn mine_pairs(commits: &[CommitDelta]) -> Vec<MinedPair> {
    struct PairStat {
        commits: HashSet<usize>,
        files: HashSet<String>,
        first_idx: usize,
        last_idx: usize,
        example: String,
    }
    let mut pairs: HashMap<(String, String), PairStat> = HashMap::new();
    let mut deltas: HashMap<String, Vec<(usize, i64)>> = HashMap::new();

    for (idx, commit) in commits.iter().enumerate() {
        if commit.files.len() > MASS_COMMIT_FILE_CAP {
            continue;
        }
        for file in &commit.files {
            let removed: HashSet<&String> = file.removed.iter().collect();
            let added: HashSet<&String> = file.added.iter().collect();
            let truly_removed: Vec<&String> = removed.difference(&added).copied().collect();
            let truly_added: Vec<&String> = added.difference(&removed).copied().collect();
            for spec in &truly_removed {
                deltas.entry((*spec).clone()).or_default().push((idx, -1));
            }
            for spec in &truly_added {
                deltas.entry((*spec).clone()).or_default().push((idx, 1));
            }
            if truly_removed.len() > CHURN_CAP || truly_added.len() > CHURN_CAP {
                continue;
            }
            for old in &truly_removed {
                for new in &truly_added {
                    let stat = pairs
                        .entry(((*old).clone(), (*new).clone()))
                        .or_insert_with(|| PairStat {
                            commits: HashSet::new(),
                            files: HashSet::new(),
                            first_idx: idx,
                            last_idx: idx,
                            example: commit.sha.clone(),
                        });
                    stat.commits.insert(idx);
                    stat.files.insert(file.path.clone());
                    stat.last_idx = idx;
                }
            }
        }
    }

    let net_since = |spec: &str, start: usize| -> i64 {
        deltas
            .get(spec)
            .map(|d| d.iter().filter(|(i, _)| *i >= start).map(|(_, v)| v).sum())
            .unwrap_or(0)
    };

    let supported = |old: &str, new: &str| -> usize {
        pairs
            .get(&(old.to_string(), new.to_string()))
            .map(|s| s.commits.len())
            .unwrap_or(0)
    };

    let mut survivors: Vec<(&(String, String), &PairStat)> = Vec::new();
    for (key, stat) in &pairs {
        let (old, new) = key;
        if stat.commits.len() < MIN_COMMITS || stat.files.len() < MIN_FILES {
            continue;
        }
        let reverse = supported(new, old);
        if stat.commits.len() < ASYMMETRY * reverse.max(1) {
            continue;
        }
        if net_since(old, stat.first_idx) >= 0 || net_since(new, stat.first_idx) <= 0 {
            continue;
        }
        survivors.push((key, stat));
    }

    let mut x_partners: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut y_fan_in: HashMap<&str, usize> = HashMap::new();
    for ((old, new), stat) in &pairs {
        if stat.commits.len() >= 2 {
            x_partners.entry(old).or_default().push(new);
            *y_fan_in.entry(new).or_default() += 1;
        }
    }

    let mut mined: Vec<MinedPair> = Vec::new();
    for ((old, new), stat) in survivors {
        if y_fan_in.get(new.as_str()).copied().unwrap_or(0) >= REPLACEMENT_SINK_CAP {
            continue;
        }
        let partners = x_partners
            .get(old.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if partners.len() > 1 {
            let mut rivals: Vec<(usize, &str)> =
                partners.iter().map(|y| (supported(old, y), *y)).collect();
            rivals.sort_by(|a, b| b.cmp(a));
            let dominant = rivals[0].1 == new && rivals[0].0 >= ASYMMETRY * rivals[1].0;
            if !dominant {
                continue;
            }
        }
        mined.push(MinedPair {
            old: old.clone(),
            new: new.clone(),
            commits: stat.commits.len(),
            files: stat.files.len(),
            first: commits[stat.first_idx].date.clone(),
            last: commits[stat.last_idx].date.clone(),
            example_commit: stat.example.clone(),
        });
    }
    mined.sort_by(|a, b| {
        b.commits
            .cmp(&a.commits)
            .then_with(|| b.files.cmp(&a.files))
            .then_with(|| a.old.cmp(&b.old))
    });
    mined
}

/// Per-language mining input assembled from the history walk.
#[derive(Debug, Default)]
pub struct LanguageDeltas {
    pub imports: Vec<CommitDelta>,
    pub callees: Vec<CommitDelta>,
}

/// Walk ≤ [`SUPERSEDE_WINDOW`] accepted first-parent commits and assemble
/// per-language specifier deltas. `keep` scopes paths exactly like the
/// corpus walk (the excluded dirs that never shape the voice never shape
/// supersessions either). Returns an empty map when the repo has no usable
/// history.
pub fn history_deltas(
    repo_dir: &Path,
    keep: &(dyn Fn(&str) -> bool + Sync),
) -> BTreeMap<String, LanguageDeltas> {
    let Ok(repo) = git2::Repository::discover(repo_dir) else {
        return BTreeMap::new();
    };
    let Some(head) = repo.head().ok().and_then(|h| h.peel_to_commit().ok()) else {
        return BTreeMap::new();
    };
    let Ok(mut revwalk) = repo.revwalk() else {
        return BTreeMap::new();
    };
    if revwalk.set_sorting(git2::Sort::TOPOLOGICAL).is_err()
        || revwalk.push(head.id()).is_err()
        || revwalk.simplify_first_parent().is_err()
    {
        return BTreeMap::new();
    }

    let mut oids: Vec<git2::Oid> = Vec::new();
    for oid in revwalk {
        if oids.len() >= SUPERSEDE_WINDOW {
            break;
        }
        let Ok(oid) = oid else { break };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        if commit.parent_count() == 1 {
            oids.push(oid);
        }
    }
    if oids.len() < MIN_COMMITS {
        return BTreeMap::new();
    }

    struct CommitFiles {
        sha: String,
        date: String,
        time: i64,
        /// (language, path, old blob oid, new blob oid)
        files: Vec<(&'static str, String, git2::Oid, git2::Oid)>,
    }

    // Phase 1 — parallel per-commit tree diffs: collect both-sided blob ids
    // per changed corpus-language file. libgit2 is thread-safe across
    // separate repository handles; each worker opens its own.
    let per_commit: Vec<Option<CommitFiles>> =
        argot_engine::par::par_map_indexed(oids.len(), |i| {
            let repo = git2::Repository::discover(repo_dir).ok()?;
            let commit = repo.find_commit(oids[i]).ok()?;
            let parent = commit.parent(0).ok()?;
            let (old_tree, new_tree) = (parent.tree().ok()?, commit.tree().ok()?);
            let mut diff = repo
                .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
                .ok()?;
            let _ = diff.find_similar(Some(&mut git2::DiffFindOptions::new()));

            let mut files = Vec::new();
            for d in diff.deltas() {
                if d.status() == git2::Delta::Added || d.status() == git2::Delta::Deleted {
                    continue;
                }
                let Some(path) = d.new_file().path().map(|p| p.to_string_lossy().to_string())
                else {
                    continue;
                };
                let Some(lang) = ext_to_lang(&extension(&path)) else {
                    continue;
                };
                if !keep(&path) {
                    continue;
                }
                files.push((lang, path, d.old_file().id(), d.new_file().id()));
            }
            let when = commit.author().when().seconds();
            Some(CommitFiles {
                sha: short_sha(&oids[i]),
                date: iso_date(when),
                time: when,
                files,
            })
        });
    let mut commits: Vec<CommitFiles> = per_commit.into_iter().flatten().collect();
    commits.sort_by_key(|c| c.time);

    // Phase 2 — parallel per-unique-blob extraction. The old side of one
    // commit is the new side of the next, so deduping by blob id roughly
    // halves the parse work; unchanged renames dedupe to a single parse.
    let mut unique: Vec<(git2::Oid, &'static str)> = Vec::new();
    let mut seen: HashSet<git2::Oid> = HashSet::new();
    for c in &commits {
        for (lang, _, old_id, new_id) in &c.files {
            for id in [old_id, new_id] {
                if !id.is_zero() && seen.insert(*id) {
                    unique.push((*id, lang));
                }
            }
        }
    }
    struct BlobSpecs {
        imports: HashSet<String>,
        callees: HashSet<String>,
    }
    let extracted: Vec<Option<(git2::Oid, BlobSpecs)>> =
        argot_engine::par::par_map_indexed(unique.len(), |i| {
            let (id, lang) = unique[i];
            let repo = git2::Repository::discover(repo_dir).ok()?;
            let blob = repo.find_blob(id).ok()?;
            if blob.size() > MAX_BLOB {
                return None;
            }
            let text = String::from_utf8_lossy(blob.content()).to_string();
            let adapter = adapter_for(lang)?;
            let language = Language::from_scoring_name(lang)?;
            Some((
                id,
                BlobSpecs {
                    imports: adapter.extract_imports(&text),
                    callees: non_none_callees(&text, language).into_iter().collect(),
                },
            ))
        });
    let by_blob: HashMap<git2::Oid, BlobSpecs> = extracted.into_iter().flatten().collect();

    // Phase 3 — assemble per-language, per-vocabulary commit deltas.
    let mut out: BTreeMap<String, LanguageDeltas> = BTreeMap::new();
    for c in &commits {
        let mut per_lang: BTreeMap<&'static str, (Vec<FileSpecDelta>, Vec<FileSpecDelta>)> =
            BTreeMap::new();
        for (lang, path, old_id, new_id) in &c.files {
            let (Some(old), Some(new)) = (by_blob.get(old_id), by_blob.get(new_id)) else {
                continue;
            };
            let entry = per_lang.entry(lang).or_default();
            let set_delta = |old: &HashSet<String>, new: &HashSet<String>| {
                let removed: Vec<String> = old.difference(new).cloned().collect();
                let added: Vec<String> = new.difference(old).cloned().collect();
                (removed, added)
            };
            let (removed, added) = set_delta(&old.imports, &new.imports);
            if !removed.is_empty() || !added.is_empty() {
                entry.0.push(FileSpecDelta {
                    path: path.clone(),
                    removed,
                    added,
                });
            }
            let (removed, added) = set_delta(&old.callees, &new.callees);
            if !removed.is_empty() || !added.is_empty() {
                entry.1.push(FileSpecDelta {
                    path: path.clone(),
                    removed,
                    added,
                });
            }
        }
        for (lang, (imports, callees)) in per_lang {
            let deltas = out.entry(lang.to_string()).or_default();
            if !imports.is_empty() {
                deltas.imports.push(CommitDelta {
                    sha: c.sha.clone(),
                    date: c.date.clone(),
                    files: imports,
                });
            }
            if !callees.is_empty() {
                deltas.callees.push(CommitDelta {
                    sha: c.sha.clone(),
                    date: c.date.clone(),
                    files: callees,
                });
            }
        }
    }
    out
}

/// Mine one language's supersessions from its history deltas. Callee names
/// below the distinctiveness bar never become pairs.
pub fn mine_language(deltas: &LanguageDeltas) -> Vec<(SupersessionKind, MinedPair)> {
    let mut out: Vec<(SupersessionKind, MinedPair)> = Vec::new();
    for pair in mine_pairs(&deltas.imports) {
        out.push((SupersessionKind::Import, pair));
    }
    for pair in mine_pairs(&deltas.callees) {
        if pair.old.len() >= CALLEE_MIN_LEN {
            out.push((SupersessionKind::Callee, pair));
        }
    }
    out
}

/// Attach corpus leftovers and apply the corpus-side guards: a pair with no
/// leftovers is a completed migration (dropped); a callee-kind pair whose
/// `old` is still ubiquitous is a polysemy hazard (dropped). `corpus` paths
/// are repo-relative.
pub fn attach_leftovers(
    pairs: Vec<(SupersessionKind, MinedPair)>,
    corpus: &[(String, String)],
    adapter: &dyn LanguageAdapter,
    language: Language,
) -> Vec<Supersession> {
    let mut out = Vec::new();
    for (kind, pair) in pairs {
        let mut leftovers: Vec<String> = Vec::new();
        for (path, source) in corpus {
            let present = match kind {
                SupersessionKind::Import => {
                    source.contains(pair.old.as_str())
                        && adapter.extract_imports(source).contains(&pair.old)
                }
                SupersessionKind::Callee => {
                    source.contains(pair.old.as_str())
                        && non_none_callees(source, language)
                            .iter()
                            .any(|c| c == &pair.old)
                }
            };
            if present {
                leftovers.push(path.clone());
            }
        }
        if leftovers.is_empty() {
            continue;
        }
        if kind == SupersessionKind::Callee {
            let fraction = leftovers.len() as f64 / corpus.len().max(1) as f64;
            if leftovers.len() > CALLEE_LEFTOVER_CAP || fraction > CALLEE_UBIQUITY_FRACTION {
                continue;
            }
        }
        leftovers.sort();
        let leftover_count = leftovers.len();
        leftovers.truncate(LEFTOVER_LIST_CAP);
        out.push(Supersession {
            old: pair.old,
            new: pair.new,
            kind,
            commits: pair.commits,
            files: pair.files,
            first: pair.first,
            last: pair.last,
            example_commit: pair.example_commit,
            leftover_count,
            leftovers,
        });
    }
    out
}

fn short_sha(oid: &git2::Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

/// Unix seconds → ISO `YYYY-MM-DD` (days-from-civil inverse, dependency-free).
fn iso_date(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests;
