# ID: src/less.rs:12
/// Invoke `less --version` and classify the reported version (GNU or BusyBox).
fn detect_less_version(less_path: &dyn AsRef<OsStr>) -> Option<LessVersion> {
    let resolved_path = grep_cli::resolve_binary(less_path.as_ref()).ok()?;
    let output = Command::new(resolved_path)
        .arg("--version")
        .output()
        .ok()?;

    match output.status.success() {
        true => parse_less_version(&output.stdout),
        false => parse_less_version_busybox(&output.stderr),
    }
}
