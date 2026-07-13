//! The test-integrity sense (`--features integrity`) — catches an agent
//! gaming tests to green a failing suite: weakening, disabling, or deleting
//! them **alongside a production change**.
//!
//! A changeset (both sides of every changed file) is diffed into per-version
//! test inventories ([`super::test_inventory`]) and reduced to **events**:
//!
//! * `test-deleted` — a test (or test file) removed while the production
//!   symbols it exercised still exist and production code grows.
//! * `test-disabled` — a skip/ignore marker added, a test rewritten in a
//!   disabled form, or a body gutted to a vacuous pass.
//! * `test-weakened` — assertions excised, tautologized, widened to a weaker
//!   predicate, or (in repos whose history is quiet enough) an expected
//!   literal retargeted in isolation.
//!
//! **FP is the design gate** (evidence:
//! `docs/research/evidence/test-integrity-feasibility-scout.md`). Every event
//! carries the refinements that took natural base rates to ~0–1%: the
//! tests-only scope guard, changeset-wide move/replacement detection,
//! definition-survival, prod net-churn direction, pure excision, and
//! pure-literal subjects. Residual per-repo churn is handled at fit: a
//! mini-replay over the repo's accepted history measures each event's natural
//! rate and disables the noisy ones for that repo (stored in
//! `.argot/integrity.json`, a sibling artifact — `scorer-config.json` stays
//! byte-identical). All thresholds are internal; the only user surface is the
//! `[rules]` severity of the three rules.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use argot_lang::adapters::Language;

use super::test_inventory::{
    self, defined_symbols, is_test_path, language_for_path, tautology_capable, words_of,
    AssertionSite, Strength, TestCase, TestInventory,
};

/// The fit artifact, beside `scorer-config.json`.
pub const INTEGRITY_FILE: &str = "integrity.json";
const ARTIFACT_VERSION: u32 = 1;

/// Accepted-history mini-replay window (first-parent commits), the same order
/// of magnitude as the semantic layer's calibration window.
pub const HISTORY_WINDOW: usize = 150;
/// An event class whose accepted-history rate (per test-touching commit)
/// exceeds this is disabled for the repo.
const GATE_BAR: f64 = 0.02;
/// Retargets are routine in healthy TDD; the event only gates in repos whose
/// history shows essentially none of it.
const RETARGET_GATE_BAR: f64 = 0.005;
/// Below this many observed test-touching commits the measured rates carry no
/// signal; gates fall back to the scout-informed defaults.
const MIN_OBSERVED: usize = 10;
/// Body-word Jaccard at which a removed test counts as renamed/moved to an
/// added test elsewhere in the changeset.
const RENAME_JACCARD: f64 = 0.3;
/// Site-word Jaccard at which a weaker new assertion counts as a rewrite of a
/// vanished exact assertion (comparison widening).
const WIDEN_JACCARD: f64 = 0.4;
/// More same-kind events than this in ONE changeset is a migration/refactor
/// sweep, not gaming — gaming is surgical (flip the failing few, not forty).
const BULK_EVENT_CAP: usize = 3;

// ---------------------------------------------------------------- events

/// The observable gaming events, each mapping to one of the three rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventKind {
    TestDeleted,
    TestFileDeleted,
    SkipAdded,
    BodyGutted,
    AssertionsRemoved,
    Tautology,
    Widened,
    Retarget,
}

impl EventKind {
    pub const ALL: [EventKind; 8] = [
        EventKind::TestDeleted,
        EventKind::TestFileDeleted,
        EventKind::SkipAdded,
        EventKind::BodyGutted,
        EventKind::AssertionsRemoved,
        EventKind::Tautology,
        EventKind::Widened,
        EventKind::Retarget,
    ];

    /// The rule (reason code) this event fires.
    pub fn reason(self) -> &'static str {
        match self {
            EventKind::TestDeleted | EventKind::TestFileDeleted => "test_deleted",
            EventKind::SkipAdded | EventKind::BodyGutted => "test_disabled",
            EventKind::AssertionsRemoved
            | EventKind::Tautology
            | EventKind::Widened
            | EventKind::Retarget => "test_weakened",
        }
    }

    /// Stable artifact key.
    pub fn key(self) -> &'static str {
        match self {
            EventKind::TestDeleted => "test_deleted",
            EventKind::TestFileDeleted => "test_file_deleted",
            EventKind::SkipAdded => "skip_added",
            EventKind::BodyGutted => "body_gutted",
            EventKind::AssertionsRemoved => "assertions_removed",
            EventKind::Tautology => "tautology",
            EventKind::Widened => "comparison_widened",
            EventKind::Retarget => "expected_retarget",
        }
    }

    /// Default gate when the repo's history is too short to measure: the
    /// scout showed retargets are never quiet enough to assume.
    fn default_enabled(self) -> bool {
        !matches!(self, EventKind::Retarget)
    }
}

/// One side-pair of a changed file — the engine's two-sided collection
/// shape, re-exported at this slice's historical path.
pub use argot_engine::check::two_sided::FileChange;

/// One detected gaming event, ready to become a finding.
#[derive(Debug, Clone)]
pub struct IntegrityEvent {
    pub kind: EventKind,
    /// The test file the event happened in.
    pub file: String,
    /// The affected test's name (empty for whole-file deletion).
    pub test_name: String,
    /// Best-effort 1-indexed line (new side where it exists, old side for
    /// deletions).
    pub line: usize,
    /// Event-specific fragment (counts, the weakened site, the marker).
    pub detail: String,
    /// One production file this changeset also modifies — the co-change the
    /// evidence line names.
    pub prod_file: String,
}

impl IntegrityEvent {
    /// The rendered `↳` evidence line: name the weakened/deleted test AND the
    /// co-changed production source.
    pub fn evidence(&self) -> String {
        let subject = if self.test_name.is_empty() {
            format!("`{}`", self.file)
        } else {
            format!("`{}`", self.test_name)
        };
        let what = match self.kind {
            EventKind::TestDeleted => format!("test {subject} deleted — {}", self.detail),
            EventKind::TestFileDeleted => {
                format!("test file {subject} deleted — {}", self.detail)
            }
            EventKind::SkipAdded => format!("test {subject} disabled — {}", self.detail),
            EventKind::BodyGutted => format!("test {subject} gutted — {}", self.detail),
            EventKind::AssertionsRemoved => {
                format!("test {subject} weakened — {}", self.detail)
            }
            EventKind::Tautology => format!(
                "test {subject} weakened — new assertion can never fail: {}",
                self.detail
            ),
            EventKind::Widened => format!("test {subject} weakened — {}", self.detail),
            EventKind::Retarget => format!(
                "test {subject} weakened — expected value retargeted: {}",
                self.detail
            ),
        };
        format!("{what}; this change also modifies {}", self.prod_file)
    }

    /// Deterministic content the hit hash is computed over — stable across
    /// re-renders of the evidence line.
    pub fn hash_content(&self) -> String {
        format!("{}:{}:{}", self.kind.key(), self.test_name, self.detail)
    }
}

// ---------------------------------------------------------------- artifact

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventGate {
    /// Measured accepted-history rate (per test-touching commit).
    pub rate: f64,
    /// Whether the event fires in this repo.
    pub enabled: bool,
}

/// The learned per-repo model: which events are quiet enough to gate here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityModel {
    version: u32,
    pub repo_sha: String,
    /// Test-touching accepted commits observed in the fit window.
    pub observed_commits: usize,
    /// Event key → learned gate.
    pub gates: BTreeMap<String, EventGate>,
}

impl IntegrityModel {
    /// All events enabled at their defaults — used when no artifact exists
    /// (and by the fit mini-replay itself).
    pub fn permissive() -> Self {
        IntegrityModel {
            version: ARTIFACT_VERSION,
            repo_sha: String::new(),
            observed_commits: 0,
            gates: BTreeMap::new(),
        }
    }

    pub fn enabled(&self, kind: EventKind) -> bool {
        self.gates
            .get(kind.key())
            .map(|g| g.enabled)
            .unwrap_or_else(|| kind.default_enabled())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// `None` on parse failure or version mismatch — the caller degrades to
    /// a loud skip, never a silent misread.
    pub fn from_json(s: &str) -> Option<Self> {
        let m: IntegrityModel = serde_json::from_str(s).ok()?;
        (m.version == ARTIFACT_VERSION).then_some(m)
    }
}

// ---------------------------------------------------------------- changeset evaluation

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    inter / union
}

/// A file version's role in the changeset.
fn carries_tests(path: &str, lang: Language, inv: &TestInventory) -> bool {
    is_test_path(path, lang) || !inv.is_empty()
}

/// Source text with test-case line spans REMOVED — what's left is the file's
/// production side (Rust keeps unit tests inside prod files). Removal, not
/// blanking: a line inserted inside a test span (a marker, an assertion)
/// must not shift the remainder and read as a production change.
fn strip_tests(source: &str, inv: &TestInventory) -> String {
    if inv.is_empty() {
        return source.to_string();
    }
    let lines: Vec<&str> = source.lines().collect();
    let mut drop = vec![false; lines.len()];
    for t in &inv.tests {
        let start = t.line.saturating_sub(1);
        let end = (start + t.body_lines).min(lines.len());
        for d in drop.iter_mut().take(end).skip(start) {
            *d = true;
        }
    }
    lines
        .iter()
        .zip(&drop)
        .filter(|(_, d)| !**d)
        .map(|(l, _)| *l)
        .collect::<Vec<_>>()
        .join("\n")
}

struct TestFileDiff {
    path: String,
    old_inv: TestInventory,
    /// `None` = the file was deleted.
    new_inv: Option<TestInventory>,
}

/// Reduce one changeset (both sides of every changed source file) to its
/// gaming events. Pure — used by check (findings) and by fit (base rates).
pub fn changeset_events(files: &[FileChange]) -> Vec<IntegrityEvent> {
    let mut events = Vec::new();

    // ---- split test carriers from production, gather prod context
    let mut test_diffs: Vec<TestFileDiff> = Vec::new();
    let mut prod_touched = false;
    let mut prod_deleted_any = false;
    let mut prod_old_defs: HashSet<String> = HashSet::new();
    let mut prod_new_defs: HashSet<String> = HashSet::new();
    let mut prod_old_lines = 0usize;
    let mut prod_new_lines = 0usize;
    let mut prod_example = String::new();

    for fc in files {
        let Some(lang) = language_for_path(&fc.path) else {
            continue;
        };
        let old_inv = fc
            .old
            .as_deref()
            .map(|s| test_inventory::extract(s, lang))
            .unwrap_or_default();
        let new_inv = fc
            .new
            .as_deref()
            .map(|s| test_inventory::extract(s, lang))
            .unwrap_or_default();

        let is_test_carrier =
            carries_tests(&fc.path, lang, &old_inv) || carries_tests(&fc.path, lang, &new_inv);

        // The file's production side (test spans stripped). A path-conventional
        // test file has no production side at all (its helpers are test infra).
        if !is_test_path(&fc.path, lang) {
            let old_prod = fc.old.as_deref().map(|s| strip_tests(s, &old_inv));
            let new_prod = fc.new.as_deref().map(|s| strip_tests(s, &new_inv));
            let changed = match (&old_prod, &new_prod) {
                (Some(o), Some(n)) => o != n,
                (None, Some(_)) | (Some(_), None) => true,
                (None, None) => false,
            };
            if changed {
                prod_touched = true;
                if prod_example.is_empty() {
                    prod_example = fc.path.clone();
                }
                if fc.new.is_none() {
                    prod_deleted_any = true;
                }
                if let Some(o) = &old_prod {
                    prod_old_defs.extend(defined_symbols(o, lang));
                    prod_old_lines += o.lines().count();
                }
                if let Some(n) = &new_prod {
                    prod_new_defs.extend(defined_symbols(n, lang));
                    prod_new_lines += n.lines().count();
                }
            }
        }

        if is_test_carrier {
            test_diffs.push(TestFileDiff {
                path: fc.path.clone(),
                old_inv,
                new_inv: fc.new.as_ref().map(|_| new_inv),
            });
        }
    }

    // Scope guard: a tests-only changeset is suite curation, never gaming.
    if !prod_touched || test_diffs.is_empty() {
        return events;
    }
    // A net-shrinking production change is removal/externalization — the
    // opposite shape of a bolted-on broken change whose tests got gamed.
    let prod_net_growing = prod_new_lines >= prod_old_lines;

    // ---- changeset-wide site accounting (multiset: structurally identical
    // assertions collide on site_key, so identity is key + occurrence count).
    // A key whose changeset-wide count net-drops was genuinely removed; a
    // moved assertion nets to zero.
    let mut old_site_counts: HashMap<&str, usize> = HashMap::new();
    let mut new_site_counts: HashMap<&str, usize> = HashMap::new();
    let mut changeset_new_test_names: HashSet<String> = HashSet::new();
    let mut changeset_added_test_bodies: Vec<HashSet<String>> = Vec::new();
    let mut changeset_added_tests = 0usize;
    for td in &test_diffs {
        for t in &td.old_inv.tests {
            for a in &t.assertions {
                *old_site_counts.entry(a.site_key.as_str()).or_default() += 1;
            }
        }
        let Some(ni) = &td.new_inv else { continue };
        let old_names: HashSet<&str> = td.old_inv.tests.iter().map(|t| t.name.as_str()).collect();
        for t in &ni.tests {
            changeset_new_test_names.insert(t.name.clone());
            if !old_names.contains(t.name.as_str()) {
                changeset_added_tests += 1;
                changeset_added_test_bodies.push(t.body_words.clone());
            }
            for a in &t.assertions {
                *new_site_counts.entry(a.site_key.as_str()).or_default() += 1;
            }
        }
    }
    // Whitespace-collapsed texts of both sides — the "did this exact
    // assertion text survive somewhere?" check that excuses extract-to-helper
    // refactors (the assertion moves into a plain function the inventory
    // doesn't walk). Count-aware: a surviving duplicate of an excised
    // assertion must not excuse the excision.
    let collapse = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    let changeset_new_texts: Vec<String> = files
        .iter()
        .filter_map(|f| f.new.as_deref())
        .map(collapse)
        .collect();
    let changeset_old_texts: Vec<String> = files
        .iter()
        .filter_map(|f| f.old.as_deref())
        .map(collapse)
        .collect();
    let text_count = |texts: &[String], needle: &str| -> usize {
        texts.iter().map(|t| t.matches(needle).count()).sum()
    };
    // Words of the changeset's ADDED lines in TEST-CARRYING files (line-level
    // multiset diff) — an assertion moved into a new helper and adapted to
    // its signature lives in added test text; a pure deletion adds nothing.
    // Production-side additions never excuse an excision (they routinely
    // share the test's domain vocabulary).
    let added_line_words: HashSet<String> = {
        let mut words = HashSet::new();
        for f in files {
            let (Some(o), Some(n)) = (f.old.as_deref(), f.new.as_deref()) else {
                continue;
            };
            let carries = language_for_path(&f.path).is_some_and(|lang| {
                is_test_path(&f.path, lang) || !test_inventory::extract(n, lang).is_empty()
            });
            if !carries {
                continue;
            }
            let mut old_lines: HashMap<&str, usize> = HashMap::new();
            for l in o.lines() {
                *old_lines.entry(l.trim()).or_default() += 1;
            }
            for l in n.lines() {
                let t = l.trim();
                match old_lines.get_mut(t) {
                    Some(c) if *c > 0 => *c -= 1,
                    _ => words.extend(words_of(t)),
                }
            }
        }
        words
    };
    let changeset_added_assertions: usize = new_site_counts
        .iter()
        .map(|(k, n)| n.saturating_sub(old_site_counts.get(k).copied().unwrap_or(0)))
        .sum();
    // Was this site occurrence genuinely lost from the changeset (not moved)?
    let net_lost = |key: &str| -> bool {
        new_site_counts.get(key).copied().unwrap_or(0)
            < old_site_counts.get(key).copied().unwrap_or(0)
    };

    // A removed test that reappears (same name anywhere, or an added test with
    // substantially overlapping body) was moved/rewritten, not deleted.
    let replaced_somewhere = |t: &TestCase| -> bool {
        changeset_new_test_names.contains(&t.name)
            || changeset_added_test_bodies
                .iter()
                .any(|w| jaccard(&t.body_words, w) >= RENAME_JACCARD)
    };
    // Do the production symbols this test exercised still exist after the
    // change? (No tie to the changed prod files → cannot claim survival.)
    let subject_survives = |t: &TestCase| -> bool {
        let subject: Vec<&String> = t
            .body_words
            .iter()
            .filter(|w| prod_old_defs.contains(*w))
            .collect();
        !subject.is_empty() && subject.iter().any(|w| prod_new_defs.contains(*w))
    };

    // ---- per-file inventory diffs
    let mut retarget_sites = 0usize;
    let mut retarget_sample: Option<IntegrityEvent> = None;

    for td in &test_diffs {
        let file = &td.path;
        if td.old_inv.is_empty() {
            continue; // added file — context only
        }

        let Some(ni) = &td.new_inv else {
            // Whole test file deleted.
            let n = td.old_inv.tests.len();
            let reappeared = td
                .old_inv
                .tests
                .iter()
                .filter(|t| replaced_somewhere(t))
                .count();
            let moved = reappeared * 2 >= n;
            if !prod_deleted_any
                && prod_net_growing
                && !moved
                && td.old_inv.tests.iter().any(&subject_survives)
            {
                events.push(IntegrityEvent {
                    kind: EventKind::TestFileDeleted,
                    file: file.clone(),
                    test_name: String::new(),
                    line: 1,
                    detail: format!(
                        "{n} test(s) removed with the file while the code they exercised remains"
                    ),
                    prod_file: prod_example.clone(),
                });
            }
            continue;
        };

        // Names collide legitimately (JUnit @Nested classes, rspec titles
        // repeated across describes): pair same-named tests POSITIONALLY, one
        // old to one new — keying a map by name would compare dozens of tests
        // against a single survivor and fabricate mass events.
        let mut new_by_name: HashMap<&str, Vec<&TestCase>> = HashMap::new();
        for t in &ni.tests {
            new_by_name.entry(t.name.as_str()).or_default().push(t);
        }
        let mut taken: HashMap<&str, usize> = HashMap::new();

        for t in &td.old_inv.tests {
            let slot = taken.entry(t.name.as_str()).or_default();
            let nt = new_by_name
                .get(t.name.as_str())
                .and_then(|v| v.get(*slot))
                .copied();
            *slot += 1;
            let Some(nt) = nt else {
                if !prod_deleted_any
                    && prod_net_growing
                    && !replaced_somewhere(t)
                    && subject_survives(t)
                {
                    events.push(IntegrityEvent {
                        kind: EventKind::TestDeleted,
                        file: file.clone(),
                        test_name: t.name.clone(),
                        line: t.line,
                        detail: format!(
                            "{} assertion(s) removed with it while the code it exercised remains",
                            t.assertions.len()
                        ),
                        prod_file: prod_example.clone(),
                    });
                }
                continue;
            };

            // --- disable: marker added or rewritten into a disabled form
            if nt.skip_markers > t.skip_markers || (nt.disabled && !t.disabled) {
                events.push(IntegrityEvent {
                    kind: EventKind::SkipAdded,
                    file: file.clone(),
                    test_name: t.name.clone(),
                    line: nt.line,
                    detail: "skip/ignore marker added".to_string(),
                    prod_file: prod_example.clone(),
                });
            }

            let (oa, na) = (t.assertions.len(), nt.assertions.len());

            // Occurrence-aligned site groups: structurally identical
            // assertions share a key, so pair old/new occurrences per key in
            // source order and reason about the aligned pairs + excess.
            let mut old_groups: BTreeMap<&str, Vec<&AssertionSite>> = BTreeMap::new();
            for a in &t.assertions {
                old_groups.entry(a.site_key.as_str()).or_default().push(a);
            }
            let mut new_groups: BTreeMap<&str, Vec<&AssertionSite>> = BTreeMap::new();
            for a in &nt.assertions {
                new_groups.entry(a.site_key.as_str()).or_default().push(a);
            }

            // --- disable: body gutted to a vacuous pass
            if oa > 0 && na == 0 {
                events.push(IntegrityEvent {
                    kind: EventKind::BodyGutted,
                    file: file.clone(),
                    test_name: t.name.clone(),
                    line: nt.line,
                    detail: format!("all {oa} assertion(s) gone, test kept as a vacuous pass"),
                    prod_file: prod_example.clone(),
                });
            } else if na < oa {
                // --- weaken: pure excision. Fires only when the removal is
                // surgical — every surviving assertion existed before with
                // the same literals (multiset containment: removing a middle
                // occurrence must not misalign) — and the removed occurrences
                // are net-lost from the changeset including as raw text (an
                // assertion extracted into a helper is a move, not a loss).
                let mut vanished: Vec<&AssertionSite> = Vec::new();
                let mut survivors_untouched = true;
                for (key, olds) in &old_groups {
                    let news = new_groups.get(key).map(Vec::as_slice).unwrap_or(&[]);
                    // Multiset-match new occurrences against old ones by
                    // literals; the UNMATCHED old occurrences are the truly
                    // removed ones (a positional tail would pick a surviving
                    // twin when a middle occurrence was excised).
                    let mut unmatched: Vec<&AssertionSite> = olds.to_vec();
                    for n in news {
                        if let Some(pos) = unmatched.iter().position(|o| o.literals == n.literals) {
                            unmatched.remove(pos);
                        } else {
                            survivors_untouched = false;
                        }
                    }
                    if news.len() < olds.len() && net_lost(key) {
                        vanished.extend(unmatched.into_iter().filter(|a| {
                            let raw = raw_site_text(a);
                            let moved_verbatim = text_count(&changeset_new_texts, &raw)
                                >= text_count(&changeset_old_texts, &raw);
                            let site_words = words_of(&raw);
                            let covered = site_words
                                .iter()
                                .filter(|w| added_line_words.contains(*w))
                                .count();
                            let moved_adapted =
                                !site_words.is_empty() && covered * 5 >= site_words.len() * 4;
                            !moved_verbatim && !moved_adapted
                        }));
                    }
                }
                let no_new_sites = new_groups
                    .iter()
                    .all(|(k, v)| old_groups.get(k).is_some_and(|o| o.len() >= v.len()));
                if !vanished.is_empty() && survivors_untouched && no_new_sites && prod_net_growing {
                    events.push(IntegrityEvent {
                        kind: EventKind::AssertionsRemoved,
                        file: file.clone(),
                        test_name: t.name.clone(),
                        line: nt.line,
                        detail: format!(
                            "{} assertion(s) excised, e.g. {}",
                            vanished.len(),
                            truncate(&vanished[0].site_key, 80)
                        ),
                        prod_file: prod_example.clone(),
                    });
                }
            }

            // --- weaken: expected literals retargeted at aligned sites
            // (only when occurrence counts match — a removal shifts the
            // positional alignment and would fabricate retargets)
            for (key, olds) in &old_groups {
                let news = new_groups.get(key).map(Vec::as_slice).unwrap_or(&[]);
                if news.len() != olds.len() {
                    continue;
                }
                for (o, n) in olds.iter().zip(news.iter()) {
                    if o.literals != n.literals && !o.literals.is_empty() {
                        retarget_sites += 1;
                        if retarget_sample.is_none() {
                            retarget_sample = Some(IntegrityEvent {
                                kind: EventKind::Retarget,
                                file: file.clone(),
                                test_name: t.name.clone(),
                                line: n.line,
                                detail: format!(
                                    "{} → {}",
                                    truncate(&o.literals.join(", "), 40),
                                    truncate(&n.literals.join(", "), 40)
                                ),
                                prod_file: prod_example.clone(),
                            });
                        }
                    }
                }
            }

            // --- weaken: comparison widened (an exact site net-lost; a new,
            // weaker assertion over the same subject words appeared here).
            // One event per site key: structurally identical assertions
            // collide on the key, and duplicate events would read as a bulk
            // sweep.
            let mut widened_keys: HashSet<&str> = HashSet::new();
            for oa_site in &t.assertions {
                if oa_site.strength != Strength::Exact || !net_lost(&oa_site.site_key) {
                    continue;
                }
                if !widened_keys.insert(oa_site.site_key.as_str()) {
                    continue;
                }
                let ow = words_of(&oa_site.site_key);
                for na_site in &nt.assertions {
                    if na_site.strength >= oa_site.strength
                        || old_groups.contains_key(na_site.site_key.as_str())
                    {
                        continue;
                    }
                    if jaccard(&ow, &words_of(&na_site.site_key)) >= WIDEN_JACCARD {
                        events.push(IntegrityEvent {
                            kind: EventKind::Widened,
                            file: file.clone(),
                            test_name: t.name.clone(),
                            line: na_site.line,
                            detail: format!(
                                "exact `{}` replaced by weaker `{}`",
                                truncate(&oa_site.site_key, 60),
                                truncate(&na_site.site_key, 60)
                            ),
                            prod_file: prod_example.clone(),
                        });
                        break;
                    }
                }
            }

            // --- weaken: a new assertion that can never fail (only the
            // occurrences beyond the old count are new)
            for (key, news) in &new_groups {
                let old_count = old_groups.get(key).map(|v| v.len()).unwrap_or(0);
                for a in news.iter().skip(old_count) {
                    if a.subject_idents == 0 && tautology_capable(&a.callee) {
                        events.push(IntegrityEvent {
                            kind: EventKind::Tautology,
                            file: file.clone(),
                            test_name: t.name.clone(),
                            line: a.line,
                            detail: truncate(&a.site_key, 80).to_string(),
                            prod_file: prod_example.clone(),
                        });
                    }
                }
            }
        }
    }

    // Bulk guard: a changeset tripping the same event on more than
    // BULK_EVENT_CAP tests is a migration/refactor sweep — gaming is
    // surgical. (Whole-file deletion stays: it is already one event.)
    {
        let mut per_kind: HashMap<EventKind, HashSet<(&str, &str)>> = HashMap::new();
        for e in &events {
            per_kind
                .entry(e.kind)
                .or_default()
                .insert((e.file.as_str(), e.test_name.as_str()));
        }
        let bulky: HashSet<EventKind> = per_kind
            .iter()
            .filter(|(_, tests)| tests.len() > BULK_EVENT_CAP)
            .map(|(k, _)| *k)
            .collect();
        events.retain(|e| e.kind == EventKind::TestFileDeleted || !bulky.contains(&e.kind));
    }

    // Retarget fires once per changeset, only as an ISOLATED flip: one or two
    // sites moved and nothing was added anywhere — the shape of "make the
    // failing assertion agree with the broken code" rather than a behaviour
    // change rippling through a suite.
    if (1..=2).contains(&retarget_sites)
        && changeset_added_assertions == 0
        && changeset_added_tests == 0
    {
        if let Some(ev) = retarget_sample {
            events.push(ev);
        }
    }

    events
}

/// Reconstruct an assertion's collapsed raw text from its blanked site key +
/// literals (the site key blanks each literal to `▯`, in order).
fn raw_site_text(a: &AssertionSite) -> String {
    let mut out = String::new();
    let mut lits = a.literals.iter();
    for (i, part) in a.site_key.split('▯').enumerate() {
        if i > 0 {
            if let Some(l) = lits.next() {
                out.push_str(&l.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        }
        out.push_str(part);
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

// ---------------------------------------------------------------- fit (mini-replay)

/// Fit the per-repo gates: replay the accepted-history window, measure each
/// event's natural rate over test-touching commits, and disable event classes
/// this repo's normal development trips too often. Returns `None` when the
/// repo has no usable history (no git, empty).
pub fn fit_model(repo_dir: &Path, repo_sha: &str) -> Option<IntegrityModel> {
    let repo = git2::Repository::discover(repo_dir).ok()?;
    let head = repo.head().ok()?.peel_to_commit().ok()?;

    let mut revwalk = repo.revwalk().ok()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL).ok()?;
    revwalk.push(head.id()).ok()?;
    revwalk.simplify_first_parent().ok()?;

    // Collect the replay window (single-parent commits, walk order) first,
    // then replay in parallel: each commit's events depend only on its own
    // two trees, and per-commit results are aggregated in walk order — the
    // counts are identical to the sequential replay, only the wall-clock
    // changes (~34 s single-threaded on a 1.4k-file corpus).
    let mut oids: Vec<git2::Oid> = Vec::new();
    for oid in revwalk {
        if oids.len() >= HISTORY_WINDOW {
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

    let per_commit = argot_engine::par::par_map_indexed(oids.len(), |i| {
        // libgit2 is thread-safe across *separate* repository handles; each
        // replay worker opens its own.
        let repo = git2::Repository::discover(repo_dir).ok()?;
        commit_events(&repo, oids[i])
    });

    let mut test_touching = 0usize;
    let mut commits_with: HashMap<EventKind, usize> = HashMap::new();
    for (touches_test, kinds) in per_commit.into_iter().flatten() {
        if !touches_test {
            continue;
        }
        test_touching += 1;
        for k in kinds {
            *commits_with.entry(k).or_default() += 1;
        }
    }

    let mut gates = BTreeMap::new();
    for kind in EventKind::ALL {
        let hits = commits_with.get(&kind).copied().unwrap_or(0);
        let rate = hits as f64 / test_touching.max(1) as f64;
        let enabled = if test_touching < MIN_OBSERVED {
            kind.default_enabled()
        } else {
            let bar = if kind == EventKind::Retarget {
                RETARGET_GATE_BAR
            } else {
                GATE_BAR
            };
            // Below-bar noise is quiet enough to gate. A single observed
            // event is weak evidence of a high true rate (1/20 commits is
            // 33%-likely under a true rate ≤ 2%), so disabling requires at
            // least two. Retargets must also clear the scout's verdict that
            // they are never a safe default.
            (rate <= bar || hits < 2) && (kind != EventKind::Retarget || hits == 0)
        };
        gates.insert(kind.key().to_string(), EventGate { rate, enabled });
    }

    Some(IntegrityModel {
        version: ARTIFACT_VERSION,
        repo_sha: repo_sha.to_string(),
        observed_commits: test_touching,
        gates,
    })
}

/// Replay one accepted commit: diff it against its (single) parent, build the
/// two-sided changeset, and reduce it to gaming events. Returns whether the
/// commit touches tests and the distinct event kinds it trips; `None` when the
/// commit/trees can't be read (skipped, exactly like the sequential replay's
/// `continue`).
fn commit_events(repo: &git2::Repository, oid: git2::Oid) -> Option<(bool, HashSet<EventKind>)> {
    let commit = repo.find_commit(oid).ok()?;
    let parent = commit.parent(0).ok()?;
    let (old_tree, new_tree) = (parent.tree().ok()?, commit.tree().ok()?);
    let mut diff = repo
        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
        .ok()?;
    let _ = diff.find_similar(Some(&mut git2::DiffFindOptions::new()));

    let mut files = Vec::new();
    let mut touches_test = false;
    for d in diff.deltas() {
        let new_path = d.new_file().path().map(|p| p.to_string_lossy().to_string());
        let old_path = d.old_file().path().map(|p| p.to_string_lossy().to_string());
        let path = new_path.clone().or(old_path.clone()).unwrap_or_default();
        let Some(lang) = language_for_path(&path) else {
            continue;
        };
        if is_test_path(&path, lang) {
            touches_test = true;
        }
        let old = old_path
            .as_deref()
            .filter(|_| d.status() != git2::Delta::Added)
            .and_then(|p| blob_text(repo, &old_tree, p));
        let new = new_path
            .as_deref()
            .filter(|_| d.status() != git2::Delta::Deleted)
            .and_then(|p| blob_text(repo, &new_tree, p));
        if old.is_none() && new.is_none() {
            continue;
        }
        files.push(FileChange { path, old, new });
    }
    // Rust in-file tests: a commit can touch tests without touching a
    // conventional test path; count it once events prove tests moved.
    let events = changeset_events(&files);
    if !events.is_empty() {
        touches_test = true;
    }
    Some((touches_test, events.iter().map(|e| e.kind).collect()))
}

fn blob_text(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
    let entry = tree.get_path(Path::new(path)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    if blob.size() > 400_000 {
        return None;
    }
    Some(String::from_utf8_lossy(blob.content()).to_string())
}

#[cfg(test)]
mod tests;
