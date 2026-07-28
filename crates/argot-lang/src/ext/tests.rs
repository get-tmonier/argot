use super::*;

/// A repository that writes Object Pascal.
fn pascal_repo() -> RepoLangs {
    RepoLangs {
        header_is_cpp: false,
        has_pascal_units: true,
        has_c_units: false,
    }
}

#[test]
fn an_include_belongs_to_the_language_the_repository_writes() {
    // `.inc` is Object Pascal's include extension and equally C's, and the name
    // cannot tell them apart. Routing it to Pascal unconditionally put 28
    // RocksDB files and 6 curl files — ~11 600 lines of C — through the Pascal
    // grammar, losing 95 % and 100 % of their lines and, worse, teaching the
    // model C as Pascal vocabulary.
    assert_eq!(ext_to_lang_ctx(".inc", pascal_repo()), Some("pascal"));

    let c_repo = RepoLangs {
        has_c_units: true,
        ..RepoLangs::default()
    };
    assert_eq!(ext_to_lang_ctx(".inc", c_repo), Some("c"));

    let cpp_repo = RepoLangs {
        header_is_cpp: true,
        has_c_units: true,
        ..RepoLangs::default()
    };
    assert_eq!(ext_to_lang_ctx(".inc", cpp_repo), Some("cpp"));

    // Neither language: better unscored than misread. A repository that writes
    // neither Pascal nor C has no claim on what its `.inc` files contain.
    assert_eq!(ext_to_lang_ctx(".inc", RepoLangs::default()), None);

    // Pascal wins over C when the repository has both — mORMot ships C
    // translation units beside its units, and its `.inc` are Pascal.
    let both = RepoLangs {
        has_pascal_units: true,
        has_c_units: true,
        header_is_cpp: true,
    };
    assert_eq!(ext_to_lang_ctx(".inc", both), Some("pascal"));
}

#[test]
fn a_header_still_routes_by_translation_unit_majority() {
    // Unchanged: `.h` is C or C++ by the repo's own majority, and nothing about
    // the include rule may disturb it.
    let c_repo = RepoLangs {
        has_c_units: true,
        ..RepoLangs::default()
    };
    assert_eq!(ext_to_lang_ctx(".h", c_repo), Some("c"));
    let cpp_repo = RepoLangs {
        header_is_cpp: true,
        has_c_units: true,
        ..RepoLangs::default()
    };
    assert_eq!(ext_to_lang_ctx(".h", cpp_repo), Some("cpp"));
}

#[test]
fn every_unambiguous_extension_ignores_the_repository() {
    // Only `.h` and `.inc` are context-sensitive; the rest must answer the same
    // whatever the repository writes, or fit and check could disagree.
    for (ext, want) in [
        (".pas", "pascal"),
        (".dpr", "pascal"),
        (".py", "python"),
        (".ts", "typescript"),
        (".cpp", "cpp"),
        (".c", "c"),
    ] {
        for langs in [RepoLangs::default(), pascal_repo()] {
            assert_eq!(ext_to_lang_ctx(ext, langs), Some(want), "{ext} {langs:?}");
        }
    }
    assert_eq!(ext_to_lang_ctx(".md", pascal_repo()), None);
}
