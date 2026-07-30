//! Atomic `.argot/` artifact writes — shared by the base calibration output
//! (`scorer-config.json`) and every additive rule slice's own sibling
//! artifact (the semantic index, the layering graph, the integrity gates).

/// Write a fit artifact via temp-file + rename, so a fit killed mid-write (a
/// laptop shutdown or interrupted explicit fit) can never leave a
/// half-written artifact behind — the previous version survives intact.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
