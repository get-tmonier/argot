//! Regression coverage for public CLI language.
//!
//! Help is part of the product surface: keep a reviewed snapshot for each
//! public command, then reject retired positioning language everywhere else in
//! user-visible CLI Rust. The narrow allowlist preserves compatibility wording
//! that remains intentionally exposed by the `voice-diff`, MCP, and STYLE.md
//! surfaces.

use std::path::Path;
use std::process::Command;

const HELP_SNAPSHOTS: &str = include_str!("fixtures/public_cli_help.snap");

const PUBLIC_HELP: &[(&str, &[&str])] = &[
    ("root", &[]),
    ("audit", &["audit"]),
    ("init", &["init"]),
    ("fit", &["fit"]),
    ("check", &["check"]),
    ("rules", &["rules"]),
    ("rules-test", &["rules", "test"]),
    ("review", &["review"]),
    ("voice-diff", &["voice-diff"]),
    ("inspect", &["inspect"]),
    ("mute", &["mute"]),
    ("list-mutes", &["list-mutes"]),
    ("review-mutes", &["review-mutes"]),
    ("status", &["status"]),
    ("list", &["list"]),
    ("update", &["update"]),
    ("cache", &["cache"]),
    ("cache-clear", &["cache", "clear"]),
    ("uninstall", &["uninstall"]),
    ("mcp", &["mcp"]),
    ("describe-voice", &["describe-voice"]),
    ("conventions", &["conventions"]),
];

fn public_help() -> Vec<(&'static str, &'static [&'static str])> {
    #[cfg(feature = "semantic")]
    {
        let mut commands = PUBLIC_HELP.to_vec();
        commands[0] = ("root-semantic", &[]);
        commands.push(("model", &["model"]));
        commands.push(("model-fetch", &["model", "fetch"]));
        commands.push(("model-status", &["model", "status"]));
        commands.push(("model-clean", &["model", "clean"]));
        commands
    }
    #[cfg(not(feature = "semantic"))]
    {
        PUBLIC_HELP.to_vec()
    }
}

const FORBIDDEN_PHRASES: &[&str] = &[
    "voice linter",
    "your codebase has a voice",
    "style linter",
    "out of voice",
    "out-of-voice",
    "AI snuck in",
    "catches AI mistakes",
    "automatically checks every change before you accept it",
    "detects who wrote code",
    "what AI introduced",
];

/// Compatibility phrases that are intentionally user-visible. Keep this list
/// path- and phrase-specific: it is an audit trail, not a blanket exemption.
const PHRASE_ALLOWLIST: &[(&str, &str)] = &[
    ("crates/argot-cli/src/describe.rs", "out of voice"),
    ("crates/argot-cli/src/main.rs", "out-of-voice"),
    ("crates/argot-cli/src/mcp.rs", "out of voice"),
];

fn snapshot(name: &str) -> &str {
    let marker = format!("===== {name} =====\n");
    let (_, after_marker) = HELP_SNAPSHOTS
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing {name} snapshot"));
    after_marker
        .split_once("\n===== ")
        .map_or(after_marker, |(snapshot, _)| snapshot)
        .trim_end_matches('\n')
}

#[test]
fn every_public_command_help_matches_its_reviewed_snapshot() {
    for (name, args) in public_help() {
        let output = Command::new(env!("CARGO_BIN_EXE_argot"))
            .args(args)
            .arg("--help")
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|error| panic!("run {name} help: {error}"));
        assert!(
            output.status.success(),
            "{name} help failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).expect("help output is UTF-8");
        assert_eq!(actual.trim_end(), snapshot(name), "{name} help changed");
    }
}

#[test]
fn forbidden_positioning_phrases_are_allowlisted_by_path() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut paths = vec![src];
    while let Some(directory) = paths.pop() {
        for entry in std::fs::read_dir(directory).expect("read CLI source directory") {
            let entry = entry.expect("read CLI source entry");
            let path = entry.path();
            if path.is_dir() {
                paths.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            let display_path = path.to_string_lossy().replace('\\', "/");
            let source = std::fs::read_to_string(&path).expect("read CLI Rust source");
            for phrase in FORBIDDEN_PHRASES {
                if source.contains(phrase)
                    && !PHRASE_ALLOWLIST
                        .iter()
                        .any(|(allowed_path, allowed_phrase)| {
                            display_path.ends_with(allowed_path) && phrase == allowed_phrase
                        })
                {
                    panic!(
                    "forbidden user-visible phrase `{phrase}` in {display_path}; add reviewed wording or a narrow allowlist entry"
                );
                }
            }
        }
    }
}
