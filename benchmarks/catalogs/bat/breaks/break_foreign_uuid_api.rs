// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: prefix a render label with the crate tag, in the printer voice.
fn render_label(kind: &str) -> String {
    format!("bat:{kind}")
}

// Break: uuid v4 generation stamping each render with a unique session tag,
// referenced by fully-qualified path (no `use` import). Verified foreign at the
// pinned SHA 78951393e29b: `uuid` = 0 grep hits across *.rs and absent from
// Cargo.toml; bat identifies inputs by their ordinal/filename in InputReader
// and never mints UUIDs.
// Break: begin
fn render_session_tag() -> String {
    let id = uuid::Uuid::new_v4();
    format!("bat-session-{}", id.simple())
}
// Break: end

/// Decoy: whether a session tag is well-formed, in the printer voice.
fn is_valid_tag(tag: &str) -> bool {
    tag.starts_with("bat-session-") && tag.len() > 12
}
