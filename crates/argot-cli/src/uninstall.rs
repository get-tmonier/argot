//! `argot uninstall` — remove argot from the machine, completely and honestly.
//!
//! Builds a full inventory first, shows it, then removes after confirmation:
//!
//! - every registered repo's `.argot/` directory and `argot.local.toml`
//!   (machine-local state; the registry in `~/.argot/settings.json` knows
//!   every repo argot ever ran in),
//! - the user cache (`~/.cache/argot` / `%LOCALAPPDATA%\argot`): the embedding
//!   model, the machine-wide embedding cache, update-check state,
//! - the global registry (`~/.argot/settings.json`),
//! - the shell installer's receipt dir (`~/.config/argot`),
//! - the binary itself — deleted for shell/raw installs; npm installs get the
//!   exact `npm uninstall -g` command instead (npm owns those files).
//!
//! What it deliberately leaves: **authored source**, never build state —
//! git-tracked files (`argot.toml`, a CI workflow argot-setup-ci committed, or
//! a voice model a user chose to commit) and the custom-rules tree
//! (`.argot/rules/`, hand-written and committed with the repo). The rest of
//! `.argot/` is a rebuildable artifact and goes. Editing the user's tracked
//! tree behind their back is worse than leaving a small config file; each kept
//! path is listed with a note so removal stays a git decision.

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// How this binary got onto the machine — decides how it comes off.
#[derive(Debug, PartialEq, Eq)]
pub enum InstallMethod {
    /// Exe path crosses `node_modules`: npm owns the files.
    Npm,
    /// An installer receipt exists: the curl/powershell installer.
    ShellInstaller,
    /// Neither: a manually placed binary (or a dev build).
    RawBinary,
}

impl InstallMethod {
    fn describe(&self) -> &'static str {
        match self {
            InstallMethod::Npm => "npm (`npm i -g @tmonier/argot`)",
            InstallMethod::ShellInstaller => "shell installer (curl / powershell)",
            InstallMethod::RawBinary => "raw binary (no receipt, not npm)",
        }
    }
}

/// One registered repo's removable and kept-in-place artifacts.
struct RepoArtifacts {
    /// `.argot/` and `argot.local.toml`, when present — removed.
    removable: Vec<PathBuf>,
    /// Tracked files argot wrote (`argot.toml`, CI workflow) — listed, kept.
    tracked_left: Vec<PathBuf>,
}

pub struct UninstallPlan {
    method: InstallMethod,
    exe: Option<PathBuf>,
    /// User-level directories/files to remove, with a short label each.
    user_level: Vec<(String, PathBuf)>,
    repos: Vec<RepoArtifacts>,
}

/// `${XDG_CONFIG_HOME:-~/.config}/argot` — where the shell installer's
/// receipt lives (cargo-dist convention).
fn receipt_dir() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(x).join("argot"));
    }
    crate::home_dir().map(|h| h.join(".config").join("argot"))
}

fn detect_method(exe: Option<&Path>) -> InstallMethod {
    if crate::is_npm_install(exe) {
        return InstallMethod::Npm;
    }
    let has_receipt = receipt_dir()
        .map(|d| d.join("argot-receipt.json").is_file())
        .unwrap_or(false);
    if has_receipt {
        InstallMethod::ShellInstaller
    } else {
        InstallMethod::RawBinary
    }
}

/// Is `rel` tracked by the repo at `root`? Untracked config is machine-local
/// and removable; tracked config belongs to the user's git history.
fn is_tracked(root: &Path, rel: &str) -> bool {
    git2::Repository::open(root)
        .ok()
        .and_then(|r| r.index().ok())
        .is_some_and(|idx| idx.get_path(Path::new(rel), 0).is_some())
}

/// Top-level entries under `.argot/` that hold authored source we must keep,
/// not delete: git-tracked files (a voice model a user deliberately committed)
/// and the custom-rules tree (`.argot/rules/`, hand-written and committed with
/// the repo — see the write-rule flow). Everything else under `.argot/` is
/// rebuildable build state. Returned as absolute paths of the *top-level* child
/// so the caller can keep a whole subtree (e.g. `.argot/rules/`) intact.
fn preserved_argot_children(root: &Path, argot_dir: &Path) -> BTreeSet<PathBuf> {
    let mut keep = BTreeSet::new();
    if let Some(idx) = git2::Repository::open(root)
        .ok()
        .and_then(|r| r.index().ok())
    {
        for entry in idx.iter() {
            let Ok(rel) = std::str::from_utf8(&entry.path) else {
                continue;
            };
            if let Some(sub) = rel.strip_prefix(".argot/") {
                if let Some(top) = sub.split('/').find(|s| !s.is_empty()) {
                    keep.insert(argot_dir.join(top));
                }
            }
        }
    }
    let rules = argot_dir.join("rules");
    if rules.is_dir() {
        keep.insert(rules);
    }
    keep
}

fn repo_artifacts(root: &Path) -> RepoArtifacts {
    let mut removable = Vec::new();
    let mut tracked_left = Vec::new();
    let argot_dir = root.join(".argot");
    if argot_dir.is_dir() {
        // `.argot/` is mostly rebuildable build state, but it can also hold
        // authored source (custom rules, a committed model). Nuking the whole
        // dir would delete that — remove only the build artifacts, keep the
        // rest, and list what's kept.
        let keep = preserved_argot_children(root, &argot_dir);
        if keep.is_empty() {
            removable.push(argot_dir);
        } else {
            if let Ok(entries) = std::fs::read_dir(&argot_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !keep.contains(&p) {
                        removable.push(p);
                    }
                }
            }
            tracked_left.extend(keep);
        }
    }
    let local = root.join(argot_core::config::LOCAL_CONFIG_FILE);
    if local.is_file() {
        removable.push(local);
    }
    for rel in [
        argot_core::config::CONFIG_FILE,
        ".github/workflows/argot.yml",
    ] {
        let abs = root.join(rel);
        if abs.is_file() {
            if is_tracked(root, rel) {
                tracked_left.push(abs);
            } else {
                removable.push(abs);
            }
        }
    }
    RepoArtifacts {
        removable,
        tracked_left,
    }
}

/// Everything argot ever wrote on this machine, resolved from the live
/// system: the registry names the repos, the conventions name the rest.
pub fn build_plan() -> UninstallPlan {
    let exe = std::env::current_exe().ok();
    let method = detect_method(exe.as_deref());

    let mut user_level = Vec::new();
    if let Ok(cache) = argot_core::cache::cache_dir() {
        if cache.is_dir() {
            user_level.push((
                "cache (embedding model, embedding cache, update state)".to_string(),
                cache,
            ));
        }
    }
    let settings = crate::settings_path();
    if settings.is_file() {
        user_level.push(("global repo registry".to_string(), settings));
    }
    if let Some(dir) = receipt_dir() {
        if dir.exists() {
            user_level.push(("installer receipt".to_string(), dir));
        }
    }

    let repos = crate::read_settings()
        .repos
        .keys()
        .map(|root| repo_artifacts(Path::new(root)))
        .filter(|r| !r.removable.is_empty() || !r.tracked_left.is_empty())
        .collect();

    UninstallPlan {
        method,
        exe,
        user_level,
        repos,
    }
}

/// Recursive size, best-effort — the plan should say what 100 MB is leaving.
fn size_of(path: &Path) -> u64 {
    if path.is_file() {
        return std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                size_of(&p)
            } else {
                std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

fn render_plan(plan: &UninstallPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "argot uninstall — install method: {}\n\nWill remove:\n",
        plan.method.describe()
    ));
    for (label, path) in &plan.user_level {
        out.push_str(&format!(
            "  {} — {label} ({})\n",
            path.display(),
            crate::format_bytes(size_of(path))
        ));
    }
    for repo in &plan.repos {
        for p in &repo.removable {
            out.push_str(&format!(
                "  {} ({})\n",
                p.display(),
                crate::format_bytes(size_of(p))
            ));
        }
    }
    match (&plan.method, &plan.exe) {
        (InstallMethod::Npm, _) => {}
        (_, Some(exe)) => out.push_str(&format!("  {} — the argot binary\n", exe.display())),
        _ => {}
    }

    let tracked: Vec<&PathBuf> = plan.repos.iter().flat_map(|r| &r.tracked_left).collect();
    if !tracked.is_empty() {
        out.push_str(
            "\nLeft in place (committed to a repo — argot never edits your tracked tree):\n",
        );
        for p in tracked {
            out.push_str(&format!("  {}\n", p.display()));
        }
    }
    if plan.method == InstallMethod::Npm {
        out.push_str(
            "\nThe binary is npm-managed; finish with:\n  npm uninstall -g @tmonier/argot\n",
        );
    }
    if plan.method == InstallMethod::ShellInstaller {
        out.push_str(
            "\nNote: the installer may have added its bin dir to PATH in your shell rc — that\nline is left untouched (harmless once the binary is gone).\n",
        );
    }
    out.push_str(
        "\nAlso not touched (not argot's files): installed agent skills (`npx skills`,\nClaude plugin), any MCP registration, and a pre-write hook argot-setup may have\nadded to your agent's config (e.g. .claude/settings.json).\n",
    );
    out
}

fn remove_path(path: &Path) -> bool {
    let ok = if path.is_dir() {
        std::fs::remove_dir_all(path).is_ok()
    } else {
        std::fs::remove_file(path).is_ok()
    };
    if !ok {
        eprintln!("warning: could not remove {}", path.display());
    }
    ok
}

/// Run the uninstall. `dry_run` prints the plan and stops; otherwise the plan
/// is shown and confirmed (tty prompt, or `--yes`) before anything is removed.
pub fn run_uninstall(dry_run: bool, yes: bool) -> ExitCode {
    let plan = build_plan();
    print!("{}", render_plan(&plan));
    if dry_run {
        println!("\n(dry run — nothing removed)");
        return ExitCode::SUCCESS;
    }
    if !yes {
        if !std::io::stdin().is_terminal() {
            eprintln!("error: refusing to uninstall non-interactively — pass --yes");
            return ExitCode::from(2);
        }
        eprint!("\nRemove everything listed above? [y/N] ");
        let mut answer = String::new();
        let _ = std::io::stdin().read_line(&mut answer);
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            println!("aborted — nothing removed");
            return ExitCode::from(2);
        }
    }

    for repo in &plan.repos {
        for p in &repo.removable {
            remove_path(p);
        }
    }
    for (_, path) in &plan.user_level {
        remove_path(path);
    }
    // `~/.argot/` held only the registry; fold the now-empty dir away too.
    if let Some(dir) = crate::settings_path().parent() {
        let _ = std::fs::remove_dir(dir);
    }

    // The binary goes last — on Unix an executing file unlinks cleanly; on
    // Windows it can't, so hand over the one manual step.
    if plan.method != InstallMethod::Npm {
        if let Some(exe) = &plan.exe {
            if std::fs::remove_file(exe).is_err() {
                println!(
                    "argot is removed; delete the binary itself with:\n  rm {}",
                    exe.display()
                );
            }
        }
    }
    println!("argot uninstalled.");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_artifacts_split_tracked_from_removable() {
        let dir = std::env::temp_dir().join(format!("argot_uninst_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".argot")).unwrap();
        std::fs::write(dir.join(".argot/scorer-config.json"), "{}").unwrap();
        std::fs::write(dir.join("argot.toml"), "[exclude]\n").unwrap();
        std::fs::write(dir.join("argot.local.toml"), "").unwrap();

        let repo = git2::Repository::init(&dir).unwrap();

        // Untracked argot.toml is machine-local: removable.
        let a = repo_artifacts(&dir);
        assert!(a.tracked_left.is_empty());
        assert!(a.removable.iter().any(|p| p.ends_with("argot.toml")));
        assert!(a.removable.iter().any(|p| p.ends_with(".argot")));
        assert!(a.removable.iter().any(|p| p.ends_with("argot.local.toml")));

        // Tracked argot.toml stays, with a note.
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("argot.toml")).unwrap();
        index.write().unwrap();
        let a = repo_artifacts(&dir);
        assert!(a.tracked_left.iter().any(|p| p.ends_with("argot.toml")));
        assert!(!a.removable.iter().any(|p| p.ends_with("argot.toml")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_authored_rules_but_removes_generated_state() {
        let dir = std::env::temp_dir().join(format!("argot_uninst_rules_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".argot/rules/no-raw-sql")).unwrap();
        std::fs::write(dir.join(".argot/rules/no-raw-sql/rule.toml"), "").unwrap();
        std::fs::write(dir.join(".argot/scorer-config.json"), "{}").unwrap();
        std::fs::write(dir.join(".argot/.gitignore"), "*\n").unwrap();
        git2::Repository::init(&dir).unwrap();

        let a = repo_artifacts(&dir);
        // Build state is removed…
        assert!(a
            .removable
            .iter()
            .any(|p| p.ends_with(".argot/scorer-config.json")));
        assert!(a.removable.iter().any(|p| p.ends_with(".argot/.gitignore")));
        // …but the authored rules tree is kept and listed, never deleted.
        assert!(a.tracked_left.iter().any(|p| p.ends_with(".argot/rules")));
        assert!(!a.removable.iter().any(|p| p.ends_with("rules")));
        // And the whole `.argot/` dir is not blindly scheduled for removal.
        assert!(!a.removable.iter().any(|p| p.ends_with(".argot")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_render_names_the_npm_handoff() {
        let plan = UninstallPlan {
            method: InstallMethod::Npm,
            exe: Some(PathBuf::from("/x/node_modules/.bin/argot")),
            user_level: vec![],
            repos: vec![],
        };
        let out = render_plan(&plan);
        assert!(out.contains("npm uninstall -g @tmonier/argot"));
        assert!(!out.contains("the argot binary"), "{out}");
    }
}
