//! `.argot/health.json` — the fit's self-diagnosis, persisted.
//!
//! The calibration-drift scan (new generated, data-heavy, or vendored
//! directories shaping the voice) and the config fingerprint are computed
//! **at fit time**, where
//! the tree walk is already paid for — and written here so every later
//! command can surface them for free:
//!
//! - `check` reads this file and prints a one-line note only when material
//!   accepted drift makes deliberate maintenance worth recommending.
//! - `status` renders it as the one-stop "is my setup healthy?" answer.
//! - adaptive freshness compares the accepted tree with a compact fit-time
//!   profile. It only recommends a deliberate local fit-and-commit refresh.

use crate::config::ArgotConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The health artifact, alongside `scorer-config.json`.
pub const HEALTH_FILE: &str = "health.json";

/// What the fit knew about its own calibration when it ran.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FitHealth {
    /// SHA the voice was fitted at.
    #[serde(default)]
    pub fit_sha: String,
    /// Fingerprint of the fit-relevant config (`[exclude]` + `[detect]`) at
    /// fit time — a later mismatch means the fit no longer reflects the
    /// user's configuration.
    #[serde(default)]
    pub config_fingerprint: String,
    /// Directories that looked generated, data-heavy, or vendored and were NOT
    /// excluded — the calibration-drift candidates (`argot init --suggest`
    /// shows the evidence). Empty on a well-configured repo.
    #[serde(default)]
    pub drift_candidates: Vec<String>,
    /// Compact denominators for adaptive freshness. Later commands compare
    /// the accepted tree with `fit_sha`; they never rebuild a model or index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_profile: Option<crate::refresh::FitProfile>,
}

/// FNV-1a over the fit-relevant config. Dependency-free and stable across
/// releases (unlike `DefaultHasher`), so a binary upgrade never fakes a
/// "config changed" signal. Collisions only cost a skipped refresh note.
pub fn config_fingerprint(config: &ArgotConfig) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // Field separator so ["ab","c"] and ["a","bc"] differ.
        hash ^= 0x1f;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for p in &config.exclude.recommended {
        eat(p.as_bytes());
    }
    eat(b"--paths--");
    for p in &config.exclude.paths {
        eat(p.as_bytes());
    }
    eat(b"--detect--");
    eat(config
        .detect
        .data_threshold
        .to_bits()
        .to_le_bytes()
        .as_ref());
    for m in &config.detect.generated_markers {
        eat(m.as_bytes());
    }
    format!("{hash:016x}")
}

/// Write the health artifact atomically. Best-effort artifact diagnostics must
/// never corrupt an otherwise successful fit.
pub fn write(argot_dir: &Path, health: &FitHealth) {
    let Ok(body) = serde_json::to_string_pretty(health) else {
        return;
    };
    let path = argot_dir.join(HEALTH_FILE);
    let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// Read the health artifact, if present and parseable.
pub fn read(argot_dir: &Path) -> Option<FitHealth> {
    let raw = std::fs::read_to_string(argot_dir.join(HEALTH_FILE)).ok()?;
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // ArgotConfig has a private field, so a struct literal can't be built
    // outside config.rs — mutate a Default instead.
    #[allow(clippy::field_reassign_with_default)]
    fn fingerprint_is_stable_and_sensitive_to_fit_relevant_config() {
        let base = ArgotConfig::default();
        let a = config_fingerprint(&base);
        assert_eq!(a, config_fingerprint(&ArgotConfig::default()), "stable");

        let mut changed = ArgotConfig::default();
        changed.exclude.paths.push("vendor/".to_string());
        assert_ne!(a, config_fingerprint(&changed), "excludes change it");

        let mut changed = ArgotConfig::default();
        changed.detect.data_threshold = 0.5;
        assert_ne!(a, config_fingerprint(&changed), "detect changes it");

        // Rules/update/fit sections are check-time — they must NOT force a
        // refit signal.
        let mut rules_only = ArgotConfig::default();
        rules_only.update_check = false;
        rules_only.fit_refresh_after = Some(250);
        assert_eq!(a, config_fingerprint(&rules_only));
    }

    #[test]
    // Same private-field constraint as above.
    #[allow(clippy::field_reassign_with_default)]
    fn fingerprint_separates_field_boundaries() {
        let mut x = ArgotConfig::default();
        x.exclude.paths = vec!["ab".into(), "c".into()];
        let mut y = ArgotConfig::default();
        y.exclude.paths = vec!["a".into(), "bc".into()];
        assert_ne!(config_fingerprint(&x), config_fingerprint(&y));
    }

    #[test]
    fn health_roundtrips_and_tolerates_missing_fields() {
        let dir = std::env::temp_dir().join(format!("argot_health_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read(&dir).is_none(), "absent file → None");
        let h = FitHealth {
            fit_sha: "abc".into(),
            config_fingerprint: "deadbeef".into(),
            drift_candidates: vec!["gen/".into()],
            refresh_profile: Some(crate::refresh::FitProfile::default()),
        };
        write(&dir, &h);
        let back = read(&dir).unwrap();
        assert_eq!(back.fit_sha, "abc");
        assert_eq!(back.drift_candidates, vec!["gen/"]);
        // Forward-compat: an older file with missing fields still parses.
        std::fs::write(dir.join(HEALTH_FILE), "{}").unwrap();
        assert!(read(&dir).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
