//! Extension → language routing.

/// Extension → language name.
pub const EXT_TO_LANG: &[(&str, &str)] = &[
    (".py", "python"),
    (".ts", "typescript"),
    (".tsx", "typescript"),
    (".js", "javascript"),
    (".jsx", "javascript"),
    (".go", "go"),
    (".rs", "rust"),
    (".c", "c"),
    (".h", "c"),
    (".java", "java"),
    (".cs", "csharp"),
    (".php", "php"),
    (".cpp", "cpp"),
    (".cc", "cpp"),
    (".hpp", "cpp"),
    (".cxx", "cpp"),
    (".rb", "ruby"),
    (".pas", "pascal"),
    (".pp", "pascal"),
    (".dpr", "pascal"),
    (".lpr", "pascal"),
    (".inc", "pascal"),
];

/// The scoring language name for a lowercase file extension (with dot), or
/// `None` when unsupported. Public so out-of-process consumers of `check`'s
/// JSON (the bench, scripts) classify paths the exact way `check` routes them.
pub fn ext_to_lang(ext: &str) -> Option<&'static str> {
    EXT_TO_LANG.iter().find(|(e, _)| *e == ext).map(|(_, l)| *l)
}

/// What a repository writes, for the extensions whose name does not say which
/// language they are. Computed once per repo (see `argot_engine::corpus`), so
/// every stage routes a file to the same model calibrate built it into.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepoLangs {
    /// `.h` is C or C++ by translation-unit majority.
    pub header_is_cpp: bool,
    /// The repository has at least one Object Pascal compilation unit, so its
    /// `.inc` files are Pascal includes.
    pub has_pascal_units: bool,
    /// The repository has at least one C or C++ translation unit, so its `.inc`
    /// files are C includes when it has no Pascal.
    pub has_c_units: bool,
}

/// [`ext_to_lang`], resolving the extensions the name alone cannot settle
/// against what the repository actually writes. Every other extension is
/// unchanged.
///
/// `.h` is C or C++ by repo majority. `.inc` is an *include* — Object Pascal's,
/// but equally C's, and the name cannot tell them apart. Routing it to Pascal
/// unconditionally put 28 RocksDB files and 6 curl files, ~11 600 lines of C,
/// through the Pascal grammar: 95 % and 100 % of their lines unreadable, and
/// worse than unreadable, C learned as Pascal vocabulary. Those same files read
/// at 10,6 % loss as C. So an `.inc` belongs to Pascal in a repository that has
/// Pascal units, to C/C++ in one that has neither, and to nothing at all in a
/// repository that writes neither language — better unscored than misread.
pub fn ext_to_lang_ctx(ext: &str, langs: RepoLangs) -> Option<&'static str> {
    match ext {
        ".h" if langs.header_is_cpp => Some("cpp"),
        ".inc" if !langs.has_pascal_units => match (langs.has_c_units, langs.header_is_cpp) {
            (true, true) => Some("cpp"),
            (true, false) => Some("c"),
            (false, _) => None,
        },
        _ => ext_to_lang(ext),
    }
}

/// Python `Path(path).suffix.lower()`.
pub fn extension(path: &str) -> String {
    let name = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name[i..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;
