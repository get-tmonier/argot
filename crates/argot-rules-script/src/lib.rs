//! The scripted rule group — community rules loaded at run time.
//!
//! A rule is a directory under the repo's `.argot/rules/`:
//!
//! ```text
//! .argot/rules/no-raw-sql/
//!   rule.toml      # the declarative frame: name, severity, languages, api
//!   check.rhai     # the detection logic (sandboxed Rhai, host API v2)
//! ```
//!
//! Discovery, manifest validation, sandbox limits, and the host API live
//! here; the engine sees one more [`argot_engine::detector::Detector`]
//! (group `custom`). Custom findings carry reasons `custom:<name>`, so
//! severities (`[rules]`/`--rule`), suppression (`rule=` scopes, `[[mute]]`),
//! and every output format treat them exactly like built-in rules.

pub mod detector;
pub mod discover;
pub mod harness;
pub mod host;
pub mod manifest;
pub mod repo;

pub use detector::ScriptDetector;
