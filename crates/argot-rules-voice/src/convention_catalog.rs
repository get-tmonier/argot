//! Convention catalog — `argot conventions`.
//!
//! A reliable, human-readable list of the conventions this repo already follows.
//! The spine is the repo's **own internal API** — the shared helpers and objects
//! everyone routes through (`db.session`, a query builder, a logger). Every
//! non-trivial repo has one, so the listing is reliable on every repo (unlike
//! history-based or dominance-based signals). It is computed by walking the
//! fitted corpus and ranking, by cross-file fan-in, the receivers/bindings the
//! adapter can prove are repo-local (`callable_definitions` +
//! `internal_import_bindings`), excluding third-party imports
//! (`import_bindings`). Familiar imports come straight from the fitted model.
//!
//! Three deduped views (precision measured across 20 repos / 12 languages,
//! evidence in `docs/research/evidence/convention-miner-receiver-funnel-probe.md`):
//! `vocabulary` (imported repo symbols) + Capitalized `type_funnels` are ~90%
//! precision; lowercase `routing_objects` are the review zone. A name that is
//! both imported and called as a receiver appears once, in `vocabulary`.
//!
//! Naming morphology + syntax idioms (also learned conventions) are *not* here
//! yet: the fitted `ConventionModel` isn't persisted to the artifact, so
//! surfacing them needs a fit-time change — tracked separately.

use crate::inspect::{inspect_model, ClusterView};
use anyhow::Result;
use argot_lang::adapters::{adapter_for, Language, LanguageAdapter};
use argot_lang::callees::non_none_callees;
use argot_lang::ext::{ext_to_lang, extension};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const MIN_FILES: usize = 4;
const MAX_BLOB: usize = 800_000;
/// Single-letter lowercase receivers (`c`, `a`) need this many files to count —
/// keeps a real convention (`c` in a web handler) while dropping an incidental
/// loop variable. Two-char names (`db`) are exempt — too likely a real helper.
const SHORT_NAME_FLOOR: usize = 8;
/// self/local pronouns — never a convention receiver.
const PRONOUNS: &[&str] = &["self", "this", "super", "base", "cls", "me", "_", "it"];
/// Generic throwaway variable names — never a convention (kept small; real
/// routing conventions like `db`/`app`/`log`/`ctx`/`req`/`res` stay out).
const GENERIC: &[&str] = &[
    "result", "ret", "tmp", "temp", "val", "value", "obj", "out", "output", "buf", "buffer", "acc",
    "idx", "arr", "cur", "prev", "elem", "item", "items", "data", "args", "kwargs", "params",
    "opts", "options", "err", "error", "entry", "entries", "name", "names", "path", "paths",
    "line", "lines", "key", "keys", "value", "values", "node", "nodes", "children", "files",
    "root", "src", "props", "state", "s", "p", "m", "n", "e", "x", "y", "t", "v", "f", "g", "h",
    "i", "j", "k",
];

/// One convention entry: a repo-local name and how the repo reaches it —
/// imported from the repo's own modules, routed through as a call receiver, or
/// both. Deduped: a name that is both imported and called appears once, with
/// each signal counted.
#[derive(Serialize, Clone, Debug)]
pub struct Convention {
    pub name: String,
    /// Files that import this symbol from a repo-internal module.
    pub imported_in: usize,
    /// Files that route calls through it as a receiver.
    pub called_in: usize,
}

impl Convention {
    /// Reach used for ranking — the larger of the two signals.
    fn reach(&self) -> usize {
        self.imported_in.max(self.called_in)
    }
}

/// One identifier-morphology share of the repo's naming (`snake_case` → 0.96).
#[derive(Serialize, Clone, Debug)]
pub struct NamingShape {
    pub shape: String,
    pub fraction: f64,
}

/// One syntax idiom the repo leans on (`decorator` → 418 occurrences).
#[derive(Serialize, Clone, Debug)]
pub struct SyntaxIdiom {
    pub kind: String,
    pub count: u64,
}

/// The conventions of one language present in the repo.
#[derive(Serialize)]
pub struct LangConventions {
    pub corpus_files: usize,
    /// The repo's familiar import surface — anything outside it is foreign.
    pub familiar_imports: Vec<String>,
    /// Dominant identifier morphologies (`snake_case`, `camelCase`), most-used
    /// first — the repo's naming convention.
    pub naming: Vec<NamingShape>,
    /// Normal syntax constructs (`async_function_definition`, `decorator`),
    /// most-frequent first — the repo's structural idioms.
    pub idioms: Vec<SyntaxIdiom>,
    /// The repo's shared vocabulary — symbols imported from its own modules
    /// (`Effect`, `Console`, `RepoAddress`), each annotated with how often it is
    /// also routed through as a receiver.
    pub vocabulary: Vec<Convention>,
    /// Capitalized repo-local receivers that are *not* imported — repo-declared
    /// types/facades everyone routes through (`Str`, `Preconditions`).
    /// High-confidence convention hubs.
    pub type_funnels: Vec<Convention>,
    /// Lowercase repo-local receivers (`client`, `context`, `logger`) — routing
    /// objects; the human-review tier (some generic locals mix in).
    pub routing_objects: Vec<Convention>,
    /// Typical calls by area (the learned call clusters).
    pub clusters: Vec<ClusterView>,
}

/// The repo's conventions, per language.
#[derive(Serialize)]
pub struct ConventionCatalog {
    pub model_hash: String,
    pub languages: BTreeMap<String, LangConventions>,
    /// Placement conventions — where the repo keeps a kind of code (repo-wide,
    /// across languages).
    pub placement: Vec<crate::placement::PlacementConvention>,
}

/// Per-language accumulator for the internal-API walk.
#[derive(Default)]
struct LangAcc {
    /// Repo-declared symbol names (callable_definitions), repo-wide.
    defined: HashSet<String>,
    /// Per-file (source, internal bindings, third-party binding leads).
    files: Vec<(String, HashSet<String>, HashSet<String>)>,
    /// internal-import binding name → distinct file indices.
    binding_files: HashMap<String, HashSet<usize>>,
}

/// Build the convention catalog from a fitted repo (`.argot/scorer-config.json`)
/// plus a walk of its corpus. `top_n` bounds callees listed per cluster.
pub fn build_catalog(repo_dir: &Path, top_n: usize) -> Result<ConventionCatalog> {
    let report = inspect_model(repo_dir, top_n)?;
    let model_hash = report
        .manifest
        .as_ref()
        .map(|m| m.model_hash.clone())
        .filter(|h| !h.is_empty())
        .or_else(|| {
            report
                .languages
                .values()
                .next()
                .map(|v| v.model_hash.clone())
        })
        .unwrap_or_default();

    // One corpus walk, grouped by language, respecting the fit's scope.
    let mut accs: HashMap<&'static str, LangAcc> = HashMap::new();
    let mut adapters: HashMap<&'static str, Box<dyn LanguageAdapter>> = HashMap::new();
    for path in argot_engine::corpus::collect_source_files(repo_dir) {
        let ext = extension(&path.to_string_lossy());
        let Some(lang) = ext_to_lang(&ext) else {
            continue;
        };
        if Language::from_scoring_name(lang).is_none() {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        if src.len() > MAX_BLOB {
            continue;
        }
        adapters
            .entry(lang)
            .or_insert_with(|| adapter_for(lang).unwrap());
        let a = &adapters[lang];
        let ib = a.internal_import_bindings(&src);
        let tp: HashSet<String> = a
            .import_bindings(&src)
            .into_iter()
            .map(|(b, _)| b)
            .collect();
        let acc = accs.entry(lang).or_default();
        for d in a.callable_definitions(&src) {
            acc.defined.insert(d);
        }
        let idx = acc.files.len();
        for b in &ib {
            acc.binding_files.entry(b.clone()).or_default().insert(idx);
        }
        acc.files.push((src, ib, tp));
    }

    let mut languages = BTreeMap::new();
    for (lang, view) in report.languages {
        let internal = accs
            .get(lang.as_str())
            .map(|acc| internal_api(acc, lang.as_str()))
            .unwrap_or_default();
        let (naming, idioms) = accs
            .get(lang.as_str())
            .map(|acc| naming_idioms(acc, lang.as_str(), top_n))
            .unwrap_or_default();
        languages.insert(
            lang,
            LangConventions {
                corpus_files: view.corpus_files,
                familiar_imports: view.familiar_imports,
                naming,
                idioms,
                vocabulary: internal.0,
                type_funnels: internal.1,
                routing_objects: internal.2,
                clusters: view.clusters,
            },
        );
    }

    Ok(ConventionCatalog {
        model_hash,
        languages,
        placement: crate::placement::mine_placement(repo_dir),
    })
}

/// The language's naming morphology + syntax idioms, computed from the corpus
/// with the same `fit_convention_frequencies` the scoring stage uses (the fitted
/// `ConventionModel` isn't persisted to the artifact, so we recompute it here
/// from the walked sources). Naming: shares >= 1%, most-dominant first. Idioms:
/// the `top_n` most-frequent named node kinds.
fn naming_idioms(acc: &LangAcc, lang: &str, top_n: usize) -> (Vec<NamingShape>, Vec<SyntaxIdiom>) {
    let Some(language) = Language::from_scoring_name(lang) else {
        return (Vec::new(), Vec::new());
    };
    let corpus: Vec<(std::path::PathBuf, String)> = acc
        .files
        .iter()
        .map(|(src, _, _)| (std::path::PathBuf::new(), src.clone()))
        .collect();
    let model = crate::scoring::conventions::fit_convention_frequencies(&corpus, language);

    let mut naming: Vec<NamingShape> = if model.total_idents == 0 {
        Vec::new()
    } else {
        model
            .ident_shapes
            .iter()
            .map(|(shape, n)| NamingShape {
                shape: shape.clone(),
                fraction: *n as f64 / model.total_idents as f64,
            })
            .filter(|s| s.fraction >= 0.01)
            .collect()
    };
    naming.sort_by(|a, b| {
        b.fraction
            .partial_cmp(&a.fraction)
            .unwrap()
            .then_with(|| a.shape.cmp(&b.shape))
    });

    let mut idioms: Vec<SyntaxIdiom> = model
        .node_kinds
        .iter()
        .map(|(kind, count)| SyntaxIdiom {
            kind: kind.clone(),
            count: *count,
        })
        .collect();
    idioms.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));
    idioms.truncate(top_n);

    (naming, idioms)
}

/// Rank the language's internal API by cross-file fan-in into three tiers.
fn internal_api(acc: &LangAcc, lang: &str) -> (Vec<Convention>, Vec<Convention>, Vec<Convention>) {
    let language = Language::from_scoring_name(lang).unwrap();

    // Repo-local receiver funnels: distinct files that route a call through a
    // repo-local receiver (defined symbol or internal-import binding), excluding
    // third-party imports and self/pronoun receivers.
    let mut recv_files: HashMap<String, HashSet<usize>> = HashMap::new();
    for (idx, (src, internal_bindings, third_party)) in acc.files.iter().enumerate() {
        for callee in non_none_callees(src, language) {
            let Some(dot) = callee.rfind('.') else {
                continue; // bare call: no receiver funnel
            };
            let recv = &callee[..dot];
            let lead = recv.split(['.', ':']).next().unwrap_or(recv);
            if lead.is_empty() || PRONOUNS.contains(&lead) {
                continue;
            }
            let is_internal = acc.defined.contains(lead) || internal_bindings.contains(lead);
            let is_external = third_party.contains(lead);
            if is_internal && !is_external {
                recv_files.entry(recv.to_string()).or_default().insert(idx);
            }
        }
    }

    // (1) VOCABULARY — imported repo symbols, deduped with their receiver use.
    // A name imported from the repo's own modules; if it is also called as a
    // receiver (`Effect` → `Effect.gen`), fold that count in rather than listing
    // it twice.
    let mut vocabulary: Vec<Convention> = acc
        .binding_files
        .iter()
        .filter(|(_, f)| f.len() >= MIN_FILES)
        .map(|(name, f)| Convention {
            name: name.clone(),
            imported_in: f.len(),
            called_in: recv_files.get(name).map(HashSet::len).unwrap_or(0),
        })
        .collect();
    sort_conventions(&mut vocabulary);

    // (2)/(3) receiver funnels for names that are NOT part of the import
    // vocabulary (repo-declared types reached as `Type.method`, or param/local
    // routing objects). Capitalized → high-confidence type funnels; lowercase →
    // the noisier routing tier.
    let (mut types, mut routing): (Vec<Convention>, Vec<Convention>) = (Vec::new(), Vec::new());
    for (recv, files) in &recv_files {
        let lead = recv.split(['.', ':']).next().unwrap_or(recv);
        if files.len() < MIN_FILES || acc.binding_files.contains_key(lead) {
            continue; // imported names already live in the vocabulary
        }
        let entry = Convention {
            name: recv.clone(),
            imported_in: 0,
            called_in: files.len(),
        };
        if lead.chars().next().is_some_and(char::is_uppercase) {
            types.push(entry);
        } else {
            // Routing objects are the noisier tier: drop generic throwaways and
            // single-letter names that don't clear a higher fan-in bar.
            if GENERIC.contains(&lead) || (lead.len() == 1 && files.len() < SHORT_NAME_FLOOR) {
                continue;
            }
            routing.push(entry);
        }
    }
    sort_conventions(&mut types);
    sort_conventions(&mut routing);
    (vocabulary, types, routing)
}

fn sort_conventions(v: &mut [Convention]) {
    v.sort_by(|a, b| b.reach().cmp(&a.reach()).then_with(|| a.name.cmp(&b.name)));
}

#[cfg(test)]
mod tests;
