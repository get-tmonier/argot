//! The composition root: which rule groups this build wires into the engine.

use argot_engine::detector::RegisteredDetector;

pub(crate) fn default_detectors() -> Vec<RegisteredDetector<'static>> {
    // Order table (parity-critical): execution_rank runs additive passes
    // first, the base pass last (stderr interleave); merge_rank puts the
    // base pass's findings first (stdout). See argot-engine's detector.rs.
    // `mut` is only exercised by the `push`es below, which are all cfg-gated —
    // a build with none of semantic/arch/integrity compiled in never mutates
    // `v` after the literal.
    #[allow(unused_mut)]
    let mut v: Vec<RegisteredDetector<'static>> = vec![RegisteredDetector {
        detector: Box::new(argot_rules_voice::VoiceDetector::new()),
        execution_rank: 4,
        merge_rank: 0,
    }];
    #[cfg(feature = "semantic")]
    v.push(RegisteredDetector {
        detector: Box::new(argot_rules_semantic::SemanticDetector::new()),
        execution_rank: 0,
        merge_rank: 1,
    });
    #[cfg(feature = "arch")]
    v.push(RegisteredDetector {
        detector: Box::new(argot_rules_arch::ArchDetector),
        execution_rank: 1,
        merge_rank: 2,
    });
    #[cfg(feature = "integrity")]
    v.push(RegisteredDetector {
        detector: Box::new(argot_rules_integrity::IntegrityDetector),
        execution_rank: 2,
        merge_rank: 3,
    });
    #[cfg(feature = "script")]
    v.push(RegisteredDetector {
        detector: Box::new(argot_rules_script::ScriptDetector::new()),
        execution_rank: 3,
        merge_rank: 4,
    });
    v
}

/// The fit-time lifecycle detectors, in artifact-write order (semantic, arch,
/// integrity — the fit diagnostics' byte order). The voice model's own fit IS
/// `run_calibrate`, so it does not register here; hooks self-gate on the
/// resolved `[rules]` severities.
// Not a `vec![]` literal: every element is cfg-gated, and cfg attributes
// only attach to statements.
#[allow(clippy::vec_init_then_push)]
pub(crate) fn fit_detectors() -> Vec<Box<dyn argot_engine::detector::Detector>> {
    #[allow(unused_mut)]
    let mut v: Vec<Box<dyn argot_engine::detector::Detector>> = Vec::new();
    #[cfg(feature = "semantic")]
    v.push(Box::new(argot_rules_semantic::SemanticDetector::new()));
    #[cfg(feature = "arch")]
    v.push(Box::new(argot_rules_arch::ArchDetector));
    #[cfg(feature = "integrity")]
    v.push(Box::new(argot_rules_integrity::IntegrityDetector));
    v
}
