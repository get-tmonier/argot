#![no_main]
//! Fuzz the tree-sitter parse + tokenization pipeline on arbitrary bytes.
//!
//! `tokenize` runs the untrusted source through the language's tree-sitter
//! grammar and then the leaf-token + BPE tokenizer — the exact path `extract`
//! takes over every source file in a repo. It must never panic, hang, or use
//! unbounded memory on adversarial input.

use argot_lang::dataset::Language;
use argot_lang::tokenize::tokenize;
use libfuzzer_sys::fuzz_target;

const LANGS: &[Language] = &[
    Language::Typescript,
    Language::Javascript,
    Language::Python,
    Language::Go,
    Language::Rust,
    Language::C,
    Language::Java,
    Language::Csharp,
    Language::Php,
    Language::Cpp,
    Language::Ruby,
];

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    // First byte selects the language; the rest is the source.
    let lang = LANGS[(data[0] as usize) % LANGS.len()];
    let _ = tokenize(&data[1..], lang);
});
