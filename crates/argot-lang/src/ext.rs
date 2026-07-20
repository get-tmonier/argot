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

/// [`ext_to_lang`], resolving the `.h` C/C++ ambiguity with the repo-level
/// `header_is_cpp` decision (translation-unit majority) so check routes a
/// header to the same model calibrate built it into. All other extensions are
/// unchanged.
pub fn ext_to_lang_ctx(ext: &str, header_is_cpp: bool) -> Option<&'static str> {
    if header_is_cpp && ext == ".h" {
        return Some("cpp");
    }
    ext_to_lang(ext)
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
