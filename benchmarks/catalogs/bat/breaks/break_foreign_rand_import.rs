// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: the theme names bat ships embedded, in the assets voice.
fn builtin_theme_names() -> Vec<&'static str> {
    vec!["Monokai Extended", "Monokai Extended Light", "ansi"]
}

// Break: rand thread-local RNG picking a theme at random from the embedded
// set, import inside hunk. Verified foreign at the pinned SHA 78951393e29b:
// `rand` = 0 grep hits across *.rs and absent from Cargo.toml; bat resolves the
// theme deterministically from config/env (HighlightingAssets::get_theme in
// assets.rs), never at random.
// Break: begin
use rand::Rng;

fn random_theme_name(names: &[&'static str]) -> &'static str {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..names.len());
    names[idx]
}
// Break: end

/// Decoy: whether a theme name is the built-in default, in the assets voice.
fn is_default_theme(name: &str) -> bool {
    name == "Monokai Extended"
}
