use super::*;

/// Build a Python `LangAcc` from in-memory sources the way `build_catalog`'s
/// walk does, then rank its internal API.
fn python_acc(files: &[&str]) -> LangAcc {
    let a = adapter_for("python").unwrap();
    let mut acc = LangAcc::default();
    for src in files {
        let ib = a.internal_import_bindings(src);
        let tp: HashSet<String> = a.import_bindings(src).into_iter().map(|(b, _)| b).collect();
        for d in a.callable_definitions(src) {
            acc.defined.insert(d);
        }
        let idx = acc.files.len();
        for b in &ib {
            acc.binding_files.entry(b.clone()).or_default().insert(idx);
        }
        acc.files.push((src.to_string(), ib, tp));
    }
    acc
}

#[test]
fn imported_repo_symbol_is_vocabulary_and_folds_in_receiver_use() {
    // `db` is imported from a repo-internal module AND routed through as a
    // receiver in every file → one vocabulary entry with both counts, not two.
    let f = |n: usize| {
        format!(
            "from .database import db\ndef handler_{n}():\n    db.execute('q')\n    db.commit()\n"
        )
    };
    let srcs: Vec<String> = (0..5).map(f).collect();
    let acc = python_acc(&srcs.iter().map(String::as_str).collect::<Vec<_>>());
    let (vocab, types, routing) = internal_api(&acc, "python");

    let db = vocab
        .iter()
        .find(|x| x.name == "db")
        .expect("db in vocabulary");
    assert_eq!(db.imported_in, 5);
    assert_eq!(
        db.called_in, 5,
        "receiver use folded into the vocabulary entry"
    );
    // Deduped: `db` appears nowhere else.
    assert!(!types.iter().chain(routing.iter()).any(|x| x.name == "db"));
}

#[test]
fn third_party_receiver_never_surfaces() {
    let srcs: Vec<String> = (0..5)
        .map(|n| format!("import httpx\ndef h_{n}():\n    httpx.get('u')\n    httpx.post('u')\n"))
        .collect();
    let acc = python_acc(&srcs.iter().map(String::as_str).collect::<Vec<_>>());
    let (vocab, types, routing) = internal_api(&acc, "python");
    assert!(
        !vocab
            .iter()
            .chain(types.iter())
            .chain(routing.iter())
            .any(|x| x.name.starts_with("httpx")),
        "foreign receiver leaked"
    );
}

#[test]
fn below_min_files_is_dropped() {
    // A repo-local symbol used in only 2 files is under MIN_FILES (4).
    let srcs: Vec<String> = (0..2)
        .map(|n| format!("from .db import db\ndef g_{n}():\n    db.q()\n"))
        .collect();
    let acc = python_acc(&srcs.iter().map(String::as_str).collect::<Vec<_>>());
    let (vocab, types, routing) = internal_api(&acc, "python");
    assert!(vocab.is_empty(), "{vocab:?}");
    assert!(types.is_empty() && routing.is_empty());
}

#[test]
fn sort_conventions_orders_by_reach_desc_then_name() {
    let c = |name: &str, imp: usize, call: usize| Convention {
        name: name.into(),
        imported_in: imp,
        called_in: call,
    };
    // reach = max(imported, called): a(5), c(5), b(3).
    let mut v = vec![c("b", 3, 1), c("a", 2, 5), c("c", 5, 0)];
    sort_conventions(&mut v);
    assert_eq!(
        v.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["a", "c", "b"]
    );
}
