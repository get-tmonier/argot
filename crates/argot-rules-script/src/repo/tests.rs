use super::*;

/// A throwaway directory tree, removed on drop (including on panic).
struct Tree(PathBuf);

impl Tree {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("argot-repo-files-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn write(&self, rel: &str, body: &str) {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn reads_a_file_inside_the_root() {
    let t = Tree::new("read");
    t.write("kernel/contract.inc", "function gui_init;\n");
    let repo = RepoRoot::open(&t.0);
    assert_eq!(
        repo.read("kernel/contract.inc").as_deref(),
        Some("function gui_init;\n")
    );
}

#[test]
fn missing_file_is_none_not_an_error() {
    let t = Tree::new("missing");
    let repo = RepoRoot::open(&t.0);
    assert_eq!(repo.read("nope.txt"), None);
}

#[test]
fn refuses_to_escape_the_root() {
    let t = Tree::new("escape");
    t.write("inside.txt", "ok");
    let repo = RepoRoot::open(&t.0);
    for hostile in [
        "../secrets.txt",
        "kernel/../../secrets.txt",
        "./inside.txt",
        "",
    ] {
        assert_eq!(repo.read(hostile), None, "accepted {hostile:?}");
    }
    // An absolute path to a file that really exists is still refused.
    let abs = t.0.join("inside.txt");
    assert_eq!(repo.read(&abs.to_string_lossy()), None);
}

#[test]
fn refuses_a_symlink_pointing_out_of_the_root() {
    let outside = Tree::new("symlink-target");
    outside.write("secret.txt", "classified");
    let t = Tree::new("symlink");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.0.join("secret.txt"), t.0.join("link.txt")).unwrap();
        let repo = RepoRoot::open(&t.0);
        assert_eq!(repo.read("link.txt"), None);
    }
    #[cfg(not(unix))]
    let _ = &t;
}

#[test]
fn refuses_a_file_over_the_size_cap() {
    let t = Tree::new("toobig");
    t.write("big.txt", &"x".repeat(MAX_FILE_BYTES + 1));
    t.write("ok.txt", &"x".repeat(1024));
    let repo = RepoRoot::open(&t.0);
    assert_eq!(repo.read("big.txt"), None);
    assert_eq!(repo.read("ok.txt").map(|s| s.len()), Some(1024));
}

#[test]
fn reading_a_directory_is_none() {
    let t = Tree::new("dir");
    t.write("sub/file.txt", "x");
    let repo = RepoRoot::open(&t.0);
    assert_eq!(repo.read("sub"), None);
}

#[test]
fn paths_lists_the_tree_sorted_and_glob_filtered() {
    let t = Tree::new("paths");
    t.write("kernel/linux/gui.pas", "");
    t.write("kernel/windows/gui.pas", "");
    t.write("kernel/gui.inc", "");
    t.write("README.md", "");
    let repo = RepoRoot::open(&t.0);
    assert_eq!(
        repo.paths("kernel/*/gui.pas"),
        vec![
            "kernel/linux/gui.pas".to_string(),
            "kernel/windows/gui.pas".to_string()
        ]
    );
    assert_eq!(repo.paths("*.md"), vec!["README.md".to_string()]);
    assert!(repo.paths("").is_empty());
    assert!(repo.paths("nothing/*").is_empty());
}

#[test]
fn an_unresolvable_root_refuses_everything() {
    let repo = RepoRoot::open(Path::new("/definitely/not/here/argot-test"));
    assert_eq!(repo.read("anything"), None);
    assert!(repo.paths("*").is_empty());
}
