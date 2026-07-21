//! Placement conventions — where a repo keeps a kind of code.
//!
//! Beyond *what* API a repo uses (`convention_catalog`), teams enforce *where*
//! code lives: DB access only in the migration layer, validation at the API
//! boundary, view hooks only in components, business logic never in components.
//! The signal is corpus-agnostic — a **feature** (a call or import) that
//! **concentrates in one location** (a directory, a filename role, an
//! extension) and is near-absent elsewhere, measured by lift and concentration.
//! No framework literals: features and locations both come from the path and
//! the language adapters.
//!
//! Evidence (feature×location lift generalizes across 4 corpora / 3 languages):
//! `docs/research/evidence/team-convention-placement-mining.md`.

use argot_lang::adapters::{adapter_for, Language, LanguageAdapter};
use argot_lang::callees::non_none_callees;
use argot_lang::ext::{ext_to_lang, extension};
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

/// A location needs at least this many files before it can carry a convention.
const MIN_GROUP: usize = 8;
/// A feature needs this many files repo-wide to be worth judging.
const MIN_SUPPORT: usize = 6;
/// …and this many inside the location (the convention's real support).
const MIN_LOCAL: usize = 4;
/// Concentration vs. base rate: `P(feature|location) / P(feature)`.
const MIN_LIFT: f64 = 2.0;
/// The feature must live at least this fraction of the time in the location —
/// a high bar so the "rule" (feature-outside-home is a violation) has few leaks.
const MIN_CONCENTRATION: f64 = 0.80;
/// Raw `(feature, location)` pairs are highly redundant (10 `queryInterface.*`
/// calls for one "migrations" convention) and explode on big monorepos. We
/// aggregate to at most this many places, each with at most this many signature
/// features — a usable list, not thousands of pairs.
const MAX_CONVENTIONS: usize = 24;
const MAX_SIGNATURE: usize = 6;
const MAX_BLOB: usize = 800_000;

/// Universal self-reference tokens — a call on `self`/`this` is a same-object
/// call, never a placement signal. These are language-structural (every OOP
/// language has one), not corpus or framework vocabulary. Everything else that
/// counts as noise (a language's builtins/globals) comes from the adapter's
/// `identifier_noise()`, so nothing framework-specific is hardcoded here — and
/// no "which directories/roles matter" list either: the lift filter drops a
/// universal directory (`src/`) on its own (its base rate ≈ 1), and file roles
/// are read straight off the path.
const SELF_REFS: &[&str] = &["self", "this", "super", "base", "cls", "me", "it", "_"];

/// A mined placement convention: a location and the signature features that
/// concentrate there — "this place is where `<features>` live."
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PlacementConvention {
    /// Where it is — `dir:migrations`, `role:service`, `ext:.tsx`.
    pub location: String,
    /// Files in the location.
    pub files: usize,
    /// Path globs matching the location — the *home*. A scripted rule enforcing
    /// this convention scopes to files *outside* these (`exclude = location_globs`)
    /// and reports the signature feature: "this belongs in the home, not here."
    pub location_globs: Vec<String>,
    /// The features that concentrate here, strongest first, receiver-deduped.
    pub signature: Vec<SignatureFeature>,
}

/// One feature in a place's signature.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SignatureFeature {
    /// A call (`queryInterface.addColumn`) or bare callee (`useState`).
    pub feature: String,
    /// Files inside the location that use it.
    pub home_files: usize,
    /// Files *outside* the location that use it — the would-be violations.
    pub out_files: usize,
    pub lift: f64,
    /// `home / (home + out)` — how confined the feature is.
    pub concentration: f64,
}

/// A raw `(feature, location)` association before aggregation.
#[derive(Clone, Debug)]
struct RawCandidate {
    feature: String,
    location: String,
    loc_files: usize,
    home_files: usize,
    out_files: usize,
    lift: f64,
    concentration: f64,
}

/// One file's placement signature: its location labels + the features it uses.
struct FileSig {
    locations: Vec<String>,
    features: HashSet<String>,
}

/// Location labels for a repo-relative path: each directory segment, a filename
/// role (`x.service.ts` → `service`, `capsule.ts` → `capsule`), and the
/// extension. All derived from the path — no framework knowledge.
fn location_labels(rel: &str, ext: &str) -> Vec<String> {
    let mut out = Vec::new();
    let parts: Vec<&str> = rel.split('/').collect();
    // Every directory segment is a candidate place. A segment shared by (nearly)
    // all files — `src/`, `app/` — carries no placement signal and self-filters:
    // its base rate ≈ 1, so every feature's lift there is ≈ 1 (< MIN_LIFT). No
    // hardcoded "generic directory" list needed.
    for seg in &parts[..parts.len().saturating_sub(1)] {
        if seg.len() > 1 {
            out.push(format!("dir:{seg}"));
        }
    }
    // A filename role is the last segment of the stem — `capsule.ts` → `capsule`,
    // `user.service.ts` → `service`, `Button.tsx` → `button`. Read straight off
    // the path, no vocabulary list. A role only becomes a place when the *name
    // recurs* across the repo (MIN_GROUP) — so `capsule` (20 files) is a role and
    // `button` (1 file) is not, learned from the data, not declared.
    if let Some(file) = parts.last() {
        let stem = file.strip_suffix(ext).unwrap_or(file);
        if let Some(role) = stem.split('.').next_back().filter(|r| r.len() > 1) {
            out.push(format!("role:{}", role.to_ascii_lowercase()));
        }
    }
    if !ext.is_empty() {
        out.push(format!("ext:{ext}"));
    }
    // A repeated path segment (`foo/foo/x.ts`) must not double-count the file.
    out.sort();
    out.dedup();
    out
}

/// Path globs matching a location's home files — the inverse of the labels a
/// file gets. `dir:migrations` → everything under a `migrations/` directory;
/// `ext:.tsx` → every `.tsx`; `role:capsule` → files whose last stem segment is
/// `capsule` (`capsule.ts` and `x.capsule.ts`). A scripted rule uses these as
/// its `exclude` so it only checks files *outside* the home.
fn location_globs(location: &str) -> Vec<String> {
    if let Some(dir) = location.strip_prefix("dir:") {
        vec![format!("**/{dir}/**")]
    } else if let Some(ext) = location.strip_prefix("ext:") {
        vec![format!("**/*{ext}")]
    } else if let Some(role) = location.strip_prefix("role:") {
        vec![format!("**/{role}.*"), format!("**/*.{role}.*")]
    } else {
        Vec::new()
    }
}

/// The features a file uses: bare callees + `receiver.method` + receiver lead.
/// `noise` is the language's own noise set (from the adapter's
/// `identifier_noise()`) — the only "what to ignore" input, and it is
/// language-provided, not hardcoded here. Self-reference leads (`self`/`this`)
/// and the extractor's `<call>` sentinel (a call on an unresolved receiver, e.g.
/// a fluent chain) are dropped too — both structural, not vocabulary.
fn features(src: &str, lang: Language, noise: &HashSet<String>) -> HashSet<String> {
    let drop = |lead: &str| {
        lead.is_empty()
            || lead.starts_with('<')
            || SELF_REFS.contains(&lead)
            || noise.contains(lead)
    };
    let mut f = HashSet::new();
    for c in non_none_callees(src, lang) {
        match c.rfind('.') {
            Some(i) => {
                let recv = &c[..i];
                let lead = recv.split(['.', ':']).next().unwrap_or(recv);
                if drop(lead) {
                    continue;
                }
                f.insert(lead.to_string());
                f.insert(format!("{lead}.{}", &c[i + 1..]));
            }
            None => {
                if !drop(&c) && c.len() > 1 {
                    f.insert(c);
                }
            }
        }
    }
    f
}

/// Walk a fitted repo's corpus and mine its placement conventions, ranked by
/// `lift × home_files` (strong and well-supported first).
pub fn mine_placement(repo_dir: &Path) -> Vec<PlacementConvention> {
    let mut sigs: Vec<FileSig> = Vec::new();
    let mut adapters: HashMap<&'static str, Box<dyn LanguageAdapter>> = HashMap::new();
    for path in argot_engine::corpus::collect_source_files(repo_dir) {
        let rel = argot_engine::corpus::rel_to_repo(&path, repo_dir);
        let ext = extension(&rel);
        let Some(lang_name) = ext_to_lang(&ext) else {
            continue;
        };
        let Some(lang) = Language::from_scoring_name(lang_name) else {
            continue;
        };
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        if src.len() > MAX_BLOB {
            continue;
        }
        let adapter = adapters
            .entry(lang_name)
            .or_insert_with(|| adapter_for(lang_name).unwrap());
        sigs.push(FileSig {
            locations: location_labels(&rel, &ext),
            features: features(&src, lang, adapter.identifier_noise()),
        });
    }
    aggregate(candidates_from(&sigs))
}

/// The pure mining core over pre-computed file signatures — the unit-testable
/// half of [`mine_placement`].
fn candidates_from(sigs: &[FileSig]) -> Vec<RawCandidate> {
    let total = sigs.len();
    if total == 0 {
        return Vec::new();
    }
    let mut feat_total: HashMap<&str, usize> = HashMap::new();
    let mut loc_files: HashMap<&str, usize> = HashMap::new();
    let mut loc_feat: HashMap<(&str, &str), usize> = HashMap::new();
    for sig in sigs {
        for f in &sig.features {
            *feat_total.entry(f.as_str()).or_default() += 1;
        }
        for l in &sig.locations {
            *loc_files.entry(l.as_str()).or_default() += 1;
            for f in &sig.features {
                *loc_feat.entry((l.as_str(), f.as_str())).or_default() += 1;
            }
        }
    }

    let mut out = Vec::new();
    for (&(loc, feat), &home) in &loc_feat {
        let loc_n = loc_files[loc];
        let gtotal = feat_total[feat];
        if loc_n < MIN_GROUP || gtotal < MIN_SUPPORT || home < MIN_LOCAL {
            continue;
        }
        let lift = (home as f64 / loc_n as f64) / (gtotal as f64 / total as f64);
        let concentration = home as f64 / gtotal as f64;
        if lift >= MIN_LIFT && concentration >= MIN_CONCENTRATION {
            out.push(RawCandidate {
                feature: feat.to_string(),
                location: loc.to_string(),
                loc_files: loc_n,
                home_files: home,
                out_files: gtotal.saturating_sub(home),
                lift,
                concentration,
            });
        }
    }
    out.sort_by(|a, b| {
        strength(b)
            .partial_cmp(&strength(a))
            .unwrap()
            .then_with(|| a.location.cmp(&b.location))
            .then_with(|| a.feature.cmp(&b.feature))
    });
    out
}

/// A raw candidate's weight: strong (high lift) and well-supported (many files).
fn strength(c: &RawCandidate) -> f64 {
    c.lift * c.home_files as f64
}

/// Collapse the flat `(feature, location)` candidates into a compact list of
/// places, each with its signature features. Receiver-deduped (`queryInterface`
/// present ⇒ drop `queryInterface.addColumn` etc.), capped to the strongest
/// places, ranked by their combined signal.
fn aggregate(raw: Vec<RawCandidate>) -> Vec<PlacementConvention> {
    let mut by_loc: HashMap<String, Vec<RawCandidate>> = HashMap::new();
    for c in raw {
        by_loc.entry(c.location.clone()).or_default().push(c);
    }

    let mut places: Vec<PlacementConvention> = by_loc
        .into_iter()
        .map(|(location, mut cands)| {
            // Receiver dedup: if a bare lead (`queryInterface`) is a signature
            // feature here, drop its `lead.method` variants — they're the same
            // convention. Keep dotted features whose lead isn't itself a signal.
            let leads: HashSet<String> = cands
                .iter()
                .filter(|c| !c.feature.contains('.'))
                .map(|c| c.feature.clone())
                .collect();
            cands.retain(|c| match c.feature.split_once('.') {
                Some((lead, _)) => !leads.contains(lead),
                None => true,
            });
            cands.sort_by(|a, b| {
                strength(b)
                    .partial_cmp(&strength(a))
                    .unwrap()
                    .then_with(|| a.feature.cmp(&b.feature))
            });
            let files = cands.first().map(|c| c.loc_files).unwrap_or(0);
            let signature = cands
                .into_iter()
                .take(MAX_SIGNATURE)
                .map(|c| SignatureFeature {
                    feature: c.feature,
                    home_files: c.home_files,
                    out_files: c.out_files,
                    lift: c.lift,
                    concentration: c.concentration,
                })
                .collect::<Vec<_>>();
            PlacementConvention {
                location_globs: location_globs(&location),
                location,
                files,
                signature,
            }
        })
        .filter(|p| !p.signature.is_empty())
        .collect();

    // Rank places by their combined signal (sum of signature strengths).
    places.sort_by(|a, b| {
        let sa: f64 = a
            .signature
            .iter()
            .map(|f| f.lift * f.home_files as f64)
            .sum();
        let sb: f64 = b
            .signature
            .iter()
            .map(|f| f.lift * f.home_files as f64)
            .sum();
        sb.partial_cmp(&sa)
            .unwrap()
            .then_with(|| a.location.cmp(&b.location))
    });
    places.truncate(MAX_CONVENTIONS);
    places
}

#[cfg(test)]
mod tests;
