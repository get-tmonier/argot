// libgit2 is held at 1.8.1 (libgit2-sys 0.17 — see the workspace Cargo.toml
// comment and libgit2/libgit2#7313). That vintage's build script predates the
// fix that declares `advapi32` (registry lookups, SID ownership checks,
// CryptGenRandom), so on Windows the final link of every binary embedding
// this crate fails with unresolved `__imp_OpenProcessToken`-style externals.
// Declare the system library here; delete this file when the git2 pin moves
// back to a libgit2-sys that declares it itself (0.18+).
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
