// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::collections::HashMap;

/// Decoy: a file-type definition in ignore/types.rs's voice.
#[derive(Clone, Debug, Default)]
struct FileTypeDef {
    name: String,
    globs: Vec<String>,
}

/// Decoy: whether a definition names any globs.
fn def_is_empty(def: &FileTypeDef) -> bool {
    def.globs.is_empty()
}

// Break: serde_yaml deserializing a user file-type map from a YAML document,
// import inside hunk. Verified foreign at the pinned SHA 48b0c795f4fe:
// `serde_yaml` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; ripgrep's file-type definitions are compiled-in
// Rust tables (ignore/src/default_types.rs) extended via `--type-add`
// name:glob strings, never parsed from a YAML config.
// Break: begin
use serde_yaml;

fn load_type_defs(doc: &str) -> Vec<FileTypeDef> {
    let raw: HashMap<String, Vec<String>> =
        serde_yaml::from_str(doc).expect("parse file-type YAML");
    raw.into_iter()
        .map(|(name, globs)| FileTypeDef { name, globs })
        .collect()
}
// Break: end

/// Decoy: index the defs by their type name.
fn index_defs(defs: Vec<FileTypeDef>) -> HashMap<String, FileTypeDef> {
    defs.into_iter().map(|d| (d.name.clone(), d)).collect()
}
