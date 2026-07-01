//! Per-reason `Evidence` payloads plus the pre-computed `EvidenceCorpus`.
//!
//! Port of `engine/argot/scoring/evidence/types.py`. The shape is uniform —
//! names + a rarity stat + a small `common here` sample — but each reason
//! scopes the data differently (BPE / import repo-wide; call-receiver
//! cluster-scoped). Renderers dispatch on the [`Evidence`] enum variant, not
//! on a string `reason`, so per-reason label conventions stay out of `check`.

use serde_json::Value;
use std::collections::HashMap;

/// A 1-indexed line + 0-indexed column range inside a hunk's text.
///
/// `col_start` is inclusive, `col_end` exclusive — the half-open convention
/// tree-sitter uses. Both columns are **byte** offsets in the raw source line;
/// callers must not align carets against ANSI-highlighted output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
}

/// One entry on a `common here:` line — name + observed frequency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonEntry {
    pub name: String,
    pub count: i64,
}

/// Denominator-floored rarity context for the offending names.
///
/// `noun` and `where_` are the two lexical halves of the rendered fragment —
/// e.g. `noun="identifiers"` + `where_="repo"` → "identifiers in repo".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RarityStat {
    pub flagged_count: i64,
    pub attested_total: i64,
    pub noun: String,
    pub where_: String,
}

impl RarityStat {
    /// Full natural-language fragment: `"<noun> in <where>"`.
    pub fn scope_label(&self) -> String {
        format!("{} in {}", self.noun, self.where_)
    }
}

/// Evidence for a BPE-fired hit: identifiers reconstructed from surprising
/// token spans, each paired with its repo-wide attestation count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BpeEvidence {
    pub surprising_identifiers: Vec<CommonEntry>,
}

/// Evidence for an import-fired hit: the foreign top-level specifiers, plus
/// their `{spec -> SourceSpan}` map (insertion-ordered) for `(L7)` annotation
/// and caret underlines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEvidence {
    pub foreign_specifiers: Vec<String>,
    pub rarity: RarityStat,
    pub common_here: Vec<CommonEntry>,
    /// Insertion-ordered `(spec, span)` pairs; a small linear-lookup map
    /// mirroring the Python dict (specifiers without a span are omitted).
    pub foreign_specifier_spans: Vec<(String, SourceSpan)>,
}

impl ImportEvidence {
    /// Look up the span captured for `name`, if any.
    pub fn span_for(&self, name: &str) -> Option<&SourceSpan> {
        self.foreign_specifier_spans
            .iter()
            .find(|(spec, _)| spec == name)
            .map(|(_, span)| span)
    }
}

/// Evidence for a call-receiver-fired hit: the unattested callees, scoped to
/// the hunk file's MinHash cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallReceiverEvidence {
    pub unfamiliar_callees: Vec<String>,
    pub rarity: RarityStat,
    pub common_here: Vec<CommonEntry>,
}

/// Runtime union of evidence payloads. Renderers dispatch on the variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    Bpe(BpeEvidence),
    Import(ImportEvidence),
    CallReceiver(CallReceiverEvidence),
}

/// Denominators for the rarity stats baked into [`EvidenceCorpus`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCorpusTotals {
    pub import_specifiers_attested: i64,
    pub callees_attested_by_cluster: HashMap<usize, i64>,
}

/// Pre-computed per-dimension samples persisted in the scorer config's
/// `evidence_corpus` block. Loaded at check time by the per-reason collectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCorpus {
    pub imports: Vec<CommonEntry>,
    pub identifiers: HashMap<String, i64>,
    pub callees_by_cluster: HashMap<usize, Vec<CommonEntry>>,
    pub totals: EvidenceCorpusTotals,
}

fn parse_common_entry(v: &Value) -> Option<CommonEntry> {
    Some(CommonEntry {
        name: v.get("name")?.as_str()?.to_string(),
        count: v.get("count")?.as_i64()?,
    })
}

fn parse_common_entries(v: &Value) -> Option<Vec<CommonEntry>> {
    v.as_array()?.iter().map(parse_common_entry).collect()
}

impl EvidenceCorpus {
    /// Parse the scorer-config `evidence_corpus` block. Inverse of the Python
    /// `EvidenceCorpus.from_json_dict`: JSON keys come back as strings, and
    /// cluster ids are coerced to `usize` so they line up with the
    /// call-receiver scorer's cluster maps. Returns `None` when the block is
    /// absent or malformed — the check-time loader then renders no evidence
    /// (the Rust port keeps evidence optional so a config without the block
    /// still runs).
    pub fn from_json(raw: &Value) -> Option<Self> {
        let obj = raw.as_object()?;

        let imports = parse_common_entries(obj.get("imports")?)?;

        let identifiers_obj = obj.get("identifiers")?.as_object()?;
        let mut identifiers = HashMap::with_capacity(identifiers_obj.len());
        for (k, v) in identifiers_obj {
            identifiers.insert(k.clone(), v.as_i64()?);
        }

        let callees_obj = obj.get("callees_by_cluster")?.as_object()?;
        let mut callees_by_cluster = HashMap::with_capacity(callees_obj.len());
        for (k, v) in callees_obj {
            let cid: usize = k.parse().ok()?;
            callees_by_cluster.insert(cid, parse_common_entries(v)?);
        }

        let totals_obj = obj.get("totals")?.as_object()?;
        let import_specifiers_attested = totals_obj.get("import_specifiers_attested")?.as_i64()?;
        let callees_total_obj = totals_obj.get("callees_attested_by_cluster")?.as_object()?;
        let mut callees_attested_by_cluster = HashMap::with_capacity(callees_total_obj.len());
        for (k, v) in callees_total_obj {
            let cid: usize = k.parse().ok()?;
            callees_attested_by_cluster.insert(cid, v.as_i64()?);
        }

        Some(EvidenceCorpus {
            imports,
            identifiers,
            callees_by_cluster,
            totals: EvidenceCorpusTotals {
                import_specifiers_attested,
                callees_attested_by_cluster,
            },
        })
    }
}
