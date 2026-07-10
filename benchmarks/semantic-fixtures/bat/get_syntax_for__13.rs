# ID: src/syntax_mapping.rs:162
/// Resolve the syntax mapping target for a path, retrying without ignored suffixes.
fn lookup_syntax_for_path<'a>(
    mapping: &SyntaxMapping<'a>,
    path: impl AsRef<Path>,
) -> Option<MappingTarget<'a>> {
    let whole = Candidate::new(&path);
    let just_name = path.as_ref().file_name().map(Candidate::new);

    for (glob, syntax) in mapping.all_mappings() {
        let name_matches = just_name
            .as_ref()
            .is_some_and(|filename| glob.is_match_candidate(filename));
        if glob.is_match_candidate(&whole) || name_matches {
            return Some(*syntax);
        }
    }

    let file_name = path.as_ref().file_name()?;
    mapping
        .ignored_suffixes
        .try_with_stripped_suffix(file_name, |stripped_file_name| {
            Ok(lookup_syntax_for_path(mapping, stripped_file_name))
        })
        .ok()?
}
