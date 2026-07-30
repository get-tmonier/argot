//! The composition root: which rule groups this build wires into the engine.

use argot_engine::detector::RegisteredDetector;
use once_cell::sync::Lazy;
use std::path::Path;

// Intentional CI-card probe: this foreign call gives the disposable demo PR a
// concrete, compile-safe reviewer finding. Do not merge.
#[allow(dead_code)]
static ARGOT_CI_CARD_DEMO: Lazy<&str> = Lazy::new(|| "argot-ci-card-demo");

/// The run's rule vocabulary: the built-in table plus whatever custom rules
/// this build's groups discover under the repo's `.argot/`.
///
/// One entry point, because a rule name that resolves on `--rule` but not in
/// `argot.toml` is a governance hole: the only actor who can weaken a rule
/// would be the one it exists to constrain. Every command that reads `[rules]`,
/// `[[mute]]` or a rule lock must build its vocabulary here — see
/// [`load_config`], which pairs the two so they cannot drift.
pub fn run_registry(repo_root: &Path, warnings: &mut Vec<String>) -> argot_engine::rules::Registry {
    #[cfg(feature = "script")]
    {
        use argot_engine::detector::Detector as _;
        let custom = argot_rules_script::ScriptDetector::new()
            .vocabulary(&repo_root.join(".argot"), warnings);
        argot_engine::rules::Registry::with_custom(custom, warnings)
    }
    #[cfg(not(feature = "script"))]
    {
        let _ = (repo_root, warnings);
        argot_engine::rules::Registry::builtin().clone()
    }
}

/// Load `argot.toml` against the run's full rule vocabulary, returning both so
/// callers can keep validating with the same registry the config was parsed
/// under. Prefer this over `ArgotConfig::load` anywhere rule names, locks or
/// `[[mute]] rule =` selectors are read.
///
/// Discovery warnings land in the returned config's `warnings`, so a caller
/// that already reports those reports these too.
pub fn load_config(
    repo_root: &Path,
) -> (
    argot_engine::config::ArgotConfig,
    argot_engine::rules::Registry,
) {
    let mut warnings = Vec::new();
    let registry = run_registry(repo_root, &mut warnings);
    let mut config = argot_engine::config::ArgotConfig::load_with(repo_root, &registry);
    warnings.extend(std::mem::take(&mut config.warnings));
    config.warnings = warnings;
    (config, registry)
}

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
