//! `argot update` — self-update without touching the GitHub API.
//!
//! The previous implementation delegated to axoupdater, which resolves
//! releases through `api.github.com` — anonymous quota 60 requests/hour per
//! IP, so users behind a shared NAT/VPN saw `403 rate limit exceeded` at
//! exactly the moment they asked for an update. Nothing here needs that API:
//!
//! 1. the latest version comes from the published `version.json` — the same
//!    un-rate-limited, ETag-friendly source the daily passive notice reads,
//! 2. the installer script is fetched from the release's *web* download URL
//!    (`releases/download/…`, a plain redirect, never the API),
//! 3. receipt semantics mirror cargo-dist/axoupdater exactly: read
//!    `<config>/argot/argot-receipt.json`, refuse to update an executable
//!    that isn't the installed copy the receipt describes, pin the install
//!    prefix via `CARGO_DIST_FORCE_INSTALL_DIR`, and respect the recorded
//!    `modify_path` choice.

use serde::Deserialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::update_check;

/// Web (non-API) home of the release assets.
const INSTALLER_URL_BASE: &str = "https://github.com/get-tmonier/argot/releases/download";

/// The installer one-liner offered whenever self-update can't proceed.
const CURL_FALLBACK: &str =
    "  curl -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh";

fn default_true() -> bool {
    true
}

/// The fields of the cargo-dist install receipt the update flow needs
/// (unknown fields are ignored, so newer receipt versions keep parsing).
#[derive(Debug, Deserialize)]
struct InstallReceipt {
    /// Where the app was installed — forced onto the next installer run.
    install_prefix: PathBuf,
    /// Whether the original install was allowed to modify PATH; the update
    /// must not widen that choice. Missing in pre-0.23 receipts → true.
    #[serde(default = "default_true")]
    modify_path: bool,
}

/// `<config>/argot/argot-receipt.json`, matching where the cargo-dist shell
/// installer writes it: `$XDG_CONFIG_HOME/argot` when set, else
/// `%LOCALAPPDATA%\argot` on Windows / `~/.config/argot` elsewhere.
fn receipt_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        candidates.push(PathBuf::from(x).join("argot"));
    }
    if cfg!(windows) {
        if let Some(l) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            candidates.push(PathBuf::from(l).join("argot"));
        }
    } else if let Some(h) = crate::home_dir() {
        candidates.push(h.join(".config").join("argot"));
    }
    candidates
        .into_iter()
        .map(|d| d.join("argot-receipt.json"))
        .find(|p| p.is_file())
}

fn load_receipt(path: &Path) -> Result<InstallReceipt, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    serde_json::from_str(&body).map_err(|e| format!("could not parse {}: {e}", path.display()))
}

/// Is the running executable the installed copy the receipt describes?
/// Guards against updating "through" a dev build or a manually copied binary
/// (the receipt semantics axoupdater enforced). An exe inside the prefix's
/// `bin/` counts as inside the prefix.
fn receipt_is_for_this_exe(receipt: &InstallReceipt, exe: &Path) -> bool {
    let Ok(exe) = exe.canonicalize() else {
        return false;
    };
    let Ok(receipt_root) = receipt.install_prefix.canonicalize() else {
        return false;
    };
    let mut exe_root = match exe.parent() {
        Some(p) => p.to_path_buf(),
        None => exe,
    };
    if exe_root.file_name() == Some(OsStr::new("bin"))
        && receipt_root.file_name() != Some(OsStr::new("bin"))
    {
        if let Some(p) = exe_root.parent() {
            exe_root = p.to_path_buf();
        }
    }
    exe_root == receipt_root
}

/// Latest published version, from version.json (never the GitHub API).
fn fetch_latest_version() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build();
    let body = agent
        .get(update_check::VERSION_URL)
        .call()
        .map_err(|e| format!("could not reach {}: {e}", update_check::VERSION_URL))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let doc: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("invalid version.json: {e}"))?;
    doc["latest"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "published version.json has no `latest` field".to_string())
}

/// The release's installer script — a `releases/download/` web URL, which is
/// served as a plain redirect and is not subject to the API rate limit.
fn installer_url(version: &str) -> String {
    let v = version.trim().trim_start_matches('v');
    let ext = if cfg!(windows) { "ps1" } else { "sh" };
    format!("{INSTALLER_URL_BASE}/v{v}/argot-installer.{ext}")
}

/// Download the installer for `version` into a temp file (0744 on unix).
fn download_installer(version: &str) -> Result<PathBuf, String> {
    let url = installer_url(version);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(60))
        .build();
    let body = agent
        .get(&url)
        .call()
        .map_err(|e| format!("could not download {url}: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let ext = if cfg!(windows) { "ps1" } else { "sh" };
    let path = std::env::temp_dir().join(format!("argot-installer-{}.{ext}", std::process::id()));
    std::fs::write(&path, body).map_err(|e| format!("could not write installer: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o744);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("could not chmod installer: {e}"))?;
    }
    Ok(path)
}

/// Run the installer with the exact env contract axoupdater used, so the
/// update lands in the same prefix with the same PATH policy as the original
/// install.
fn run_installer(
    receipt: &InstallReceipt,
    installer: &Path,
) -> Result<std::process::Output, String> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("powershell");
        // Opt-in for default-security-policy machines; doesn't bypass
        // organization-set policies.
        c.arg("-ExecutionPolicy").arg("ByPass").arg(installer);
        // Fixes launching from PowerShell Core parents
        // (https://github.com/PowerShell/PowerShell/issues/18530).
        c.env_remove("PSModulePath");
        c
    } else {
        Command::new(installer)
    };
    cmd.env("CARGO_DIST_FORCE_INSTALL_DIR", &receipt.install_prefix);
    cmd.env("ARGOT_INSTALL_DIR", &receipt.install_prefix);
    if !receipt.modify_path {
        cmd.env("ARGOT_NO_MODIFY_PATH", "1");
    }
    cmd.output()
        .map_err(|e| format!("failed to launch installer: {e}"))
}

/// Windows can't overwrite a running exe: move ourselves aside first, restore
/// on failure, self-delete the parked copy on success.
#[cfg(windows)]
fn park_running_exe(exe: &Path) -> Result<PathBuf, String> {
    let mut parked = exe.as_os_str().to_os_string();
    parked.push(".previous.exe");
    let parked = PathBuf::from(parked);
    std::fs::rename(exe, &parked)
        .map_err(|e| format!("could not move the running executable aside: {e}"))?;
    Ok(parked)
}

pub fn run_update() -> ExitCode {
    let current = env!("CARGO_PKG_VERSION");

    let exe = std::env::current_exe().ok();
    if crate::is_npm_install(exe.as_deref()) {
        println!("argot {current} was installed via npm; update it with:");
        println!("  npm install -g @tmonier/argot@latest");
        return ExitCode::SUCCESS;
    }

    let Some(receipt_file) = receipt_path() else {
        eprintln!("argot {current}: no install receipt found, cannot self-update.");
        eprintln!("Re-install with the installer to enable `argot update`:");
        eprintln!("{CURL_FALLBACK}");
        return ExitCode::FAILURE;
    };
    let receipt = match load_receipt(&receipt_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("argot {current}: {e}");
            eprintln!("Re-install with the installer to repair the receipt:");
            eprintln!("{CURL_FALLBACK}");
            return ExitCode::FAILURE;
        }
    };
    let is_installed_copy = exe
        .as_deref()
        .is_some_and(|e| receipt_is_for_this_exe(&receipt, e));
    if !is_installed_copy {
        eprintln!(
            "argot {current}: this executable is not the installed copy recorded in the install receipt; skipping self-update."
        );
        eprintln!("Update the installed copy by running `argot update` from it directly.");
        return ExitCode::FAILURE;
    }

    println!("argot {current} — checking for updates...");
    let latest = match fetch_latest_version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Update check failed: {e}");
            eprintln!("Retry later, or re-install directly:");
            eprintln!("{CURL_FALLBACK}");
            return ExitCode::FAILURE;
        }
    };
    if !update_check::is_newer(&latest, current) {
        println!("Already up to date.");
        return ExitCode::SUCCESS;
    }

    println!("Updating to argot {latest}...");
    let installer = match download_installer(&latest) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Update failed: {e}");
            eprintln!("Re-install directly instead:");
            eprintln!("{CURL_FALLBACK}");
            return ExitCode::FAILURE;
        }
    };

    #[cfg(windows)]
    let parked = match exe.as_deref().map(park_running_exe).transpose() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Update failed: {e}");
            let _ = std::fs::remove_file(&installer);
            return ExitCode::FAILURE;
        }
    };

    let result = run_installer(&receipt, &installer);
    let _ = std::fs::remove_file(&installer);

    let output = match result {
        Ok(o) => o,
        Err(e) => {
            #[cfg(windows)]
            restore_parked(parked.as_deref(), exe.as_deref());
            eprintln!("Update failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if !output.status.success() {
        #[cfg(windows)]
        restore_parked(parked.as_deref(), exe.as_deref());
        eprintln!("Update failed: installer exited with {}", output.status);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("{}", stderr.trim_end());
        }
        eprintln!("Re-install directly instead:");
        eprintln!("{CURL_FALLBACK}");
        return ExitCode::FAILURE;
    }

    #[cfg(windows)]
    if let Some(parked) = parked {
        // Best-effort: the parked old exe can't delete itself synchronously.
        if self_replace::self_delete_at(&parked).is_err() {
            eprintln!(
                "note: could not remove the previous executable at {}",
                parked.display()
            );
        }
    }

    println!("Updated to argot {latest}.");
    ExitCode::SUCCESS
}

#[cfg(windows)]
fn restore_parked(parked: Option<&Path>, exe: Option<&Path>) {
    if let (Some(parked), Some(exe)) = (parked, exe) {
        let _ = std::fs::rename(parked, exe);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_url_is_the_web_download_endpoint() {
        let url = installer_url("0.3.0");
        assert!(url.starts_with("https://github.com/get-tmonier/argot/releases/download/v0.3.0/"));
        assert!(!url.contains("api.github.com"));
        // A published tag already carrying the v-prefix is not doubled.
        assert_eq!(installer_url("v0.3.0"), installer_url("0.3.0"));
        #[cfg(not(windows))]
        assert!(url.ends_with("argot-installer.sh"));
        #[cfg(windows)]
        assert!(url.ends_with("argot-installer.ps1"));
    }

    #[test]
    fn receipt_parses_real_and_minimal_shapes() {
        // Shape written by the cargo-dist shell installer (unknown fields
        // must be ignored so newer receipts keep parsing).
        let full = r#"{
            "binaries": ["argot"],
            "install_layout": "flat",
            "install_prefix": "/home/u/.local/bin",
            "modify_path": false,
            "provider": {"source": "cargo-dist", "version": "0.30.0"},
            "source": {"app_name": "argot", "name": "argot", "owner": "get-tmonier", "release_type": "github"},
            "version": "0.2.59"
        }"#;
        let r: InstallReceipt = serde_json::from_str(full).unwrap();
        assert_eq!(r.install_prefix, PathBuf::from("/home/u/.local/bin"));
        assert!(!r.modify_path);

        // Pre-0.23 receipts had no modify_path — defaults to true.
        let minimal = r#"{"install_prefix": "/opt/argot"}"#;
        let r: InstallReceipt = serde_json::from_str(minimal).unwrap();
        assert!(r.modify_path);
    }

    #[cfg(unix)]
    #[test]
    fn installer_runs_under_the_pinned_env_contract() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("argot-env-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let dump = root.join("env.txt");
        let script = root.join("fake-installer.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n%s\\n%s\\n' \"$CARGO_DIST_FORCE_INSTALL_DIR\" \"$ARGOT_INSTALL_DIR\" \"$ARGOT_NO_MODIFY_PATH\" > {}\n",
                dump.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o744)).unwrap();

        let receipt = InstallReceipt {
            install_prefix: PathBuf::from("/tmp/argot-prefix"),
            modify_path: false,
        };
        let out = run_installer(&receipt, &script).unwrap();
        assert!(out.status.success());
        let env = std::fs::read_to_string(&dump).unwrap();
        let lines: Vec<&str> = env.lines().collect();
        assert_eq!(lines[0], "/tmp/argot-prefix");
        assert_eq!(lines[1], "/tmp/argot-prefix");
        assert_eq!(
            lines[2], "1",
            "modify_path=false must set ARGOT_NO_MODIFY_PATH"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn receipt_guard_accepts_the_installed_copy_only() {
        let root = std::env::temp_dir().join(format!("argot-update-test-{}", std::process::id()));
        let prefix = root.join("prefix");
        let bin = prefix.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe_in_bin = bin.join("argot");
        std::fs::write(&exe_in_bin, b"x").unwrap();
        let elsewhere = root.join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let stray_exe = elsewhere.join("argot");
        std::fs::write(&stray_exe, b"x").unwrap();

        // Prefix recorded as the root: an exe under its bin/ is the install.
        let receipt = InstallReceipt {
            install_prefix: prefix.clone(),
            modify_path: true,
        };
        assert!(receipt_is_for_this_exe(&receipt, &exe_in_bin));
        assert!(!receipt_is_for_this_exe(&receipt, &stray_exe));

        // Prefix recorded as the bin dir itself (flat layout).
        let receipt = InstallReceipt {
            install_prefix: bin.clone(),
            modify_path: true,
        };
        assert!(receipt_is_for_this_exe(&receipt, &exe_in_bin));
        assert!(!receipt_is_for_this_exe(&receipt, &stray_exe));

        // Missing prefix on disk never matches.
        let receipt = InstallReceipt {
            install_prefix: root.join("gone"),
            modify_path: true,
        };
        assert!(!receipt_is_for_this_exe(&receipt, &exe_in_bin));

        std::fs::remove_dir_all(&root).ok();
    }
}
