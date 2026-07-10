# ID: src/syntax_mapping.rs:27
/// Compile a glob pattern into a matcher with the requested case sensitivity.
fn build_glob_matcher(from: &str, case: Case) -> Result<GlobMatcher> {
    let case_insensitive = matches!(case, Case::Insensitive);
    let compiled = GlobBuilder::new(from)
        .literal_separator(true)
        .case_insensitive(case_insensitive)
        .build()?
        .compile_matcher();
    Ok(compiled)
}
