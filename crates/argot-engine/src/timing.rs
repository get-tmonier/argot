//! Per-phase wall-clock timing, printed to stderr when `ARGOT_TIMING` is
//! truthy (set, non-empty, not `0`). A diagnostic surface for performance
//! investigations — long pipelines (audit, fit on a large corpus) are opaque
//! walls of time without it. Inert by default: nothing is printed, and the
//! only cost is one cached env lookup per phase.
//!
//! Usage: `let _t = timing::phase("fit: calibrate");` — the elapsed time is
//! printed when the guard drops. Nest freely; labels carry the structure.

use std::sync::OnceLock;
use std::time::Instant;

/// The env switch. Same truthiness convention as `ARGOT_OFFLINE`.
pub const TIMING_ENV: &str = "ARGOT_TIMING";

/// Is timing output enabled for this process? Cached after the first call.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(TIMING_ENV)
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false)
    })
}

/// RAII phase guard: prints `[timing] <label>: <secs>s` to stderr on drop.
/// A no-op (no clock read on drop, no output) when timing is disabled.
pub struct Phase {
    label: Option<String>,
    start: Instant,
}

/// Start a phase. The label is only materialised when timing is enabled.
pub fn phase(label: impl Into<String>) -> Phase {
    Phase {
        label: enabled().then(|| label.into()),
        start: Instant::now(),
    }
}

impl Phase {
    /// End the phase now (explicit alternative to letting the guard drop).
    pub fn done(self) {}

    /// Rewrite the label mid-phase — for counts only known after the work
    /// started (e.g. how many functions were embedded).
    pub fn relabel(&mut self, label: impl Into<String>) {
        if self.label.is_some() {
            self.label = Some(label.into());
        }
    }
}

impl Drop for Phase {
    fn drop(&mut self) {
        if let Some(label) = &self.label {
            eprintln!(
                "[timing] {label}: {:.2}s",
                self.start.elapsed().as_secs_f64()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_guard_is_inert_when_disabled() {
        // The test env has no ARGOT_TIMING; the guard must carry no label
        // (nothing to print on drop) and dropping must not panic.
        if !enabled() {
            let p = phase("unit-test phase");
            assert!(p.label.is_none());
            drop(p);
        }
    }

    #[test]
    fn relabel_keeps_disabled_phases_unlabelled() {
        if !enabled() {
            let mut p = phase("before");
            p.relabel("after");
            assert!(p.label.is_none());
            p.done();
        }
    }
}
