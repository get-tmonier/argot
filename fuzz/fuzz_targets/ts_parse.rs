#![no_main]
//! Fuzz raw tree-sitter parse-tree construction on arbitrary source.
//!
//! Isolates the grammar layer (`ts_parse::parse`) from tokenization, so a crash
//! here points at the tree-sitter grammar/ABI rather than argot's token
//! extraction. Must return `Some`/`None` without panicking on any input.

use argot_lang::adapters::Language;
use argot_lang::ts_parse::parse;
use libfuzzer_sys::fuzz_target;

const LANGS: &[Language] = &[
    Language::Python,
    Language::Typescript,
    Language::Javascript,
    Language::Go,
    Language::Rust,
    Language::C,
    Language::Java,
    Language::CSharp,
    Language::Php,
    Language::Cpp,
    Language::Ruby,
    Language::Pascal,
];

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let lang = LANGS[(data[0] as usize) % LANGS.len()];
    let source = String::from_utf8_lossy(&data[1..]);
    let _ = parse(&source, lang);
});
