//! argot-bench library surface — shared by the `argot-bench` binary and the
//! research scout binaries under `src/bin/`.

pub mod catalog;
pub mod dashboard;
pub mod holdout;
pub mod metrics;
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
