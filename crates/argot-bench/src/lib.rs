//! argot-bench library surface — shared by the `argot-bench` binary and the
//! research scout binaries under `src/bin/`.

/// Frozen accepted-change replay and combined-brief aggregation. This remains
/// benchmark-only: it calls argot-core's public check facade, whose composition
/// root is the same one used by the distributed binary.
pub mod accept_brief;
/// Architecture-graph floor/gate validation (`--mode arch`). Feature-gated:
/// drives argot-core's `arch_graph` sense over real corpora + real holdout.
#[cfg(feature = "arch")]
pub mod arch;
pub mod catalog;
pub mod dashboard;
pub mod holdout;
/// Test-integrity validation (`--mode integrity-verify` / `integrity-fp`).
/// Feature-gated: drives argot-core's `integrity` sense on the production
/// fit→check path over authored gaming fixtures + replayed accepted history.
#[cfg(feature = "integrity")]
pub mod integrity;
pub mod metrics;
/// Corpus-level parallel driver (`--jobs`): independent corpora fan out over
/// a work-stealing pool, results return in input order.
pub mod pool;
pub mod production;
pub mod report;
pub mod run;
pub mod scorer;
/// Structural-foreignness floor validation (`--mode structural`). Feature-gated:
/// drives argot-core's `structural` sense over real corpora + real holdout,
/// never perturbs the base metric. See the module docs.
#[cfg(feature = "structural")]
pub mod structural;
pub mod targets;
