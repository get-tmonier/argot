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
        detector: Box::new(crate::check_passes::voice::VoiceDetector::new()),
        execution_rank: 3,
        merge_rank: 0,
    }];
    #[cfg(feature = "semantic")]
    v.push(RegisteredDetector {
        detector: Box::new(crate::check_passes::semantic::SemanticDetector),
        execution_rank: 0,
        merge_rank: 1,
    });
    #[cfg(feature = "arch")]
    v.push(RegisteredDetector {
        detector: Box::new(crate::check_passes::arch::ArchDetector),
        execution_rank: 1,
        merge_rank: 2,
    });
    #[cfg(feature = "integrity")]
    v.push(RegisteredDetector {
        detector: Box::new(crate::check_passes::integrity::IntegrityDetector),
        execution_rank: 2,
        merge_rank: 3,
    });
    v
}
