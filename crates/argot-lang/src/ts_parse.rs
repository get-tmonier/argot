//! Thread-local reused tree-sitter parsers for the scoring hot path.
//!
//! Creating a `Parser` and reloading the grammar on every call is pure
//! overhead — a reused parser per language avoids re-creating the grammar on
//! every call. In scoring, `extract_imports`,
//! `extract_callees`, `prose_line_ranges`, and typicality each parse per hunk,
//! plus per corpus file at fit; reusing one parser per language per thread
//! removes thousands of parser allocations. Trees are owned, so reuse is safe.

use crate::adapters::Language;
use std::cell::RefCell;
use tree_sitter::{Parser, Tree};

/// The tree-sitter grammar for a scoring language — the one grammar table
/// every parse in the workspace routes through (scorers, adapters, and the
/// scripted rules' `ts_query` host call).
pub fn ts_language(language: Language) -> tree_sitter::Language {
    match language {
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Javascript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::Pascal => tree_sitter_pascal::LANGUAGE.into(),
    }
}

fn new_parser(language: Language) -> Parser {
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = ts_language(language);
    parser
        .set_language(&lang)
        .expect("tree-sitter grammar loads");
    parser
}

thread_local! {
    /// TSX is a *separate* grammar, not a superset: `LANGUAGE_TYPESCRIPT`
    /// cannot read JSX, and `LANGUAGE_TSX` reads `<T>expr` type assertions
    /// differently. Measured on excalidraw, parsing `.tsx` with the TypeScript
    /// grammar fails on **96 % of files** (191 of 200) against 0,5 % for `.ts`
    /// — one error node spanning 12 969 lines, and every rule that reads
    /// structure goes blind behind it. See [`parse`].
    static TSX_PARSER: RefCell<Parser> = RefCell::new({
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .expect("tree-sitter grammar loads");
        parser
    });
    static PY_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Python));
    static TS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Typescript));
    static JS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Javascript));
    static GO_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Go));
    static RUST_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Rust));
    static C_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::C));
    static JAVA_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Java));
    static CS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::CSharp));
    static PHP_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Php));
    static CPP_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Cpp));
    static RB_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Ruby));
    static PAS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Pascal));
}

/// One lexical region of Object Pascal source that is not code.
enum PasSpan {
    /// A `{$…}` or `(*$…*)` compiler directive: its body, and its full extent.
    Directive { start: usize, end: usize },
    /// A comment or string literal: skipped whole, never looked inside.
    Skip { end: usize },
}

/// The non-code region starting at `i`, if one does. Comments (`{…}`,
/// `(*…*)`, `//`) and string literals (`'…'`) are recognised so a directive
/// *inside* one is not mistaken for a live directive — `//{$endif}` is an
/// everyday way to disable a conditional and appears in MSEide/MSEgui, where
/// counting it would unbalance the branch stack for the rest of the unit.
fn pascal_span(source: &str, i: usize) -> Option<PasSpan> {
    let bytes = source.as_bytes();
    let after = |open: usize, close: &str| {
        source[i + open..]
            .find(close)
            .map(|rel| i + open + rel + close.len())
    };
    if bytes[i..].starts_with(b"{$") {
        return Some(match after(2, "}") {
            Some(end) => PasSpan::Directive { start: i + 2, end },
            None => PasSpan::Skip { end: source.len() },
        });
    }
    if bytes[i..].starts_with(b"(*$") {
        return Some(match after(3, "*)") {
            Some(end) => PasSpan::Directive { start: i + 3, end },
            None => PasSpan::Skip { end: source.len() },
        });
    }
    let end = match bytes[i] {
        b'{' => after(1, "}"),
        b'(' if bytes.get(i + 1) == Some(&b'*') => after(2, "*)"),
        b'/' if bytes.get(i + 1) == Some(&b'/') => {
            Some(source[i..].find('\n').map_or(source.len(), |rel| i + rel))
        }
        // A Pascal string literal cannot span a line, so an unterminated quote
        // ends with its line rather than swallowing the rest of the file — and
        // with it every directive below.
        b'\'' => {
            let line_end = source[i..].find('\n').map_or(source.len(), |rel| i + rel);
            Some(after(1, "'").map_or(line_end, |end| end.min(line_end)))
        }
        _ => return None,
    };
    Some(PasSpan::Skip {
        end: end.unwrap_or(source.len()),
    })
}

/// The conditional a directive body opens, continues, or closes.
enum Cond {
    Open,
    Else,
    End,
    Other,
}

fn classify_directive(body: &str) -> Cond {
    let word: String = body
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    match word.to_ascii_lowercase().as_str() {
        "ifdef" | "ifndef" | "ifopt" | "if" => Cond::Open,
        "else" | "elseif" | "elifdef" | "elifndef" | "elsec" => Cond::Else,
        "endif" | "ifend" | "endc" => Cond::End,
        _ => Cond::Other,
    }
}

/// Object Pascal compiler directives (`{$…}`, `(*$…*)`) resolved away, byte for
/// byte, so every offset in the tree still addresses the original.
///
/// Two things happen, and the second is why this is not simply "blank them".
///
/// **Every directive is blanked.** The grammar cannot parse a directive between
/// a routine header and its body — `function f: boolean; {$ifdef FPC}inline;
/// {$endif}` — and an empty `{$ifdef}{$endif}` is enough to trip it. That is an
/// everyday cross-platform idiom, and when it trips, the error node swallows the
/// **rest of the unit**: 7 500 lines of `msedb.pas` in MSEide/MSEgui, whose
/// functions then look structureless and are abstained on by every rule that
/// reads placement. A directive is a preprocessor instruction, not Pascal, so
/// removing it loses nothing the tree should have described; rules that need to
/// *see* one read the raw source, which is where it belongs.
///
/// **Only the first branch of a conditional survives.** Blanking the directives
/// alone leaves *both* branches standing, and in Object Pascal a conditional
/// routinely sits inside a single declaration, where both branches are not two
/// declarations but one broken one:
///
/// ```pascal
/// {$ifdef USERECORDWITHMETHODS} TAes = record {$else} TAes = object {$endif}
/// ```
///
/// blanked to `TAes = record … TAes = object` is a duplicate name and an
/// unterminated `record`, and mORMot's `mormot.crypt.core.pas` loses all 10 643
/// of its lines to one error node because of it. `TStrLen = {$ifdef FPC} SizeInt
/// {$else} integer {$endif};` fails the same way. Keeping both branches is right
/// for C — there they are two complete declarations, and that path is measured
/// good — and wrong here. Taking the *first* branch is deterministic, so a
/// repository parses the same way on every machine and run.
///
/// Dropping a branch costs the vocabulary it names — `uses jpeg` under
/// `{$ifdef DELPHI}` is a dependency this repository really has, and 73 of
/// mORMot's 229 units live only in a branch that is not the first. Which is why
/// the *structure* of a file is read from one branch and its *dependencies* from
/// all of them: see [`parse_pascal_every_branch`].
///
/// Replaces byte for byte and keeps newlines as newlines, so every row and
/// column in the tree still addresses the original source. Borrows unchanged
/// when the source has no directive, so the common file pays nothing.
fn blank_pascal_directives(source: &str, drop_later_branches: bool) -> std::borrow::Cow<'_, str> {
    if !source.contains("{$") && !source.contains("(*$") {
        return std::borrow::Cow::Borrowed(source);
    }
    let mut out = source.to_string();
    // One entry per open conditional: whether its *current* branch is kept.
    // A branch nested inside a dropped branch is dropped whatever it says.
    let mut stack: Vec<bool> = Vec::new();
    // Byte for byte, not character for character: one space per *byte* keeps
    // `out` the exact length of `source`, so every later index still addresses
    // the same place in both. Mapping characters instead turns a multi-byte one
    // into a single space, and from the first accented letter in a dropped
    // branch onwards every offset in the file is silently wrong. A continuation
    // byte is never `\n`, so this cannot split a character or produce anything
    // but ASCII.
    let blank = |out: &mut String, from: usize, to: usize| {
        let replacement: String = source.as_bytes()[from..to]
            .iter()
            .map(|&b| if b == b'\n' { '\n' } else { ' ' })
            .collect();
        out.replace_range(from..to, &replacement);
    };
    let mut i = 0;
    while i < source.len() {
        let Some(span) = pascal_span(source, i) else {
            // Live code: blank it when every enclosing branch is not taken.
            // One whole character at a time, so a multi-byte one is neither
            // split nor mistaken for a delimiter.
            let width = source[i..].chars().next().map_or(1, char::len_utf8);
            if stack.iter().any(|kept| !kept) {
                blank(&mut out, i, i + width);
            }
            i += width;
            continue;
        };
        match span {
            PasSpan::Skip { end } => {
                if stack.iter().any(|kept| !kept) {
                    blank(&mut out, i, end);
                }
                i = end.max(i + 1);
            }
            PasSpan::Directive { start, end } => {
                let body = &source[start..end.saturating_sub(1).max(start)];
                match classify_directive(body) {
                    // The first branch of a conditional is the one kept.
                    Cond::Open => stack.push(true),
                    // Every branch after the first is dropped.
                    Cond::Else => {
                        if let Some(kept) = stack.last_mut() {
                            *kept &= !drop_later_branches;
                        }
                    }
                    Cond::End => {
                        stack.pop();
                    }
                    Cond::Other => {}
                }
                blank(&mut out, i, end);
                i = end.max(i + 1);
            }
        }
    }
    std::borrow::Cow::Owned(out)
}

/// True for a `#if`/`#ifdef`/`#ifndef`/`#elif`/`#else`/`#endif` line.
///
/// `#define` and `#include` are deliberately not conditionals: they declare
/// names and dependencies the scorers read.
fn is_conditional_directive(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix('#') else {
        return false;
    };
    let rest = rest.trim_start();
    ["ifdef", "ifndef", "elif", "endif", "else", "if"]
        .iter()
        .any(|keyword| {
            rest.strip_prefix(keyword)
                .is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        })
}

/// Blank the conditional-compilation *control* lines of a C/C++ file, in place.
///
/// A conditional's branches are code; the `#if`/`#else`/`#endif` lines around
/// them are not, and the grammar can only represent them where a declaration
/// may stand. In an initializer list or halfway through a parameter list they
/// are unparseable, and the failure is not local: on `curl.h` a single
/// unrepresentable conditional turns the **whole file** into one `ERROR` node —
/// 3 348 of 3 347 lines unreadable. Blanking the control lines and keeping both
/// branches takes that to 434 lines, widest span 3 348 → 3.
///
/// Both branches surviving means a file can declare the same name twice. That
/// costs nothing here: the scorers read names and shapes, and a duplicate is
/// what a name already seen looks like.
///
/// Replaces byte for byte, so a directive comment with a multi-byte character
/// cannot shift the offsets the tree addresses. Borrows unchanged when the
/// source has no conditional, so the common file pays nothing.
fn blank_preprocessor_conditionals(source: &str) -> std::borrow::Cow<'_, str> {
    if !source.contains("#if") {
        return std::borrow::Cow::Borrowed(source);
    }
    let mut out = String::with_capacity(source.len());
    let mut continued = false;
    for line in source.split_inclusive('\n') {
        if continued || is_conditional_directive(line) {
            continued = line.trim_end().ends_with('\\');
            out.extend(line.bytes().map(|b| if b == b'\n' { '\n' } else { ' ' }));
        } else {
            out.push_str(line);
        }
    }
    std::borrow::Cow::Owned(out)
}

/// How many distinct lines of a tree fall inside an `ERROR` node.
///
/// Pre-order visits errors in source order, so carrying the last row covered
/// is enough to keep two adjacent errors sharing a line from counting twice.
fn error_rows(tree: &Tree) -> usize {
    fn walk(node: tree_sitter::Node, total: &mut usize, covered_to: &mut Option<usize>) {
        if node.is_error() {
            let (start, end) = (node.start_position().row, node.end_position().row);
            let from = match *covered_to {
                Some(row) if row >= start => row + 1,
                _ => start,
            };
            if end >= from {
                *total += end - from + 1;
            }
            *covered_to = Some(covered_to.map_or(end, |row| row.max(end)));
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk(child, total, covered_to);
        }
    }
    let (mut total, mut covered_to) = (0, None);
    walk(tree.root_node(), &mut total, &mut covered_to);
    total
}

/// Parse `source` with a reused per-thread parser for `language`.
pub fn parse(source: &str, language: Language) -> Option<Tree> {
    if language == Language::Typescript {
        // The extension is not carried here, so choose by outcome: the JSX
        // grammar is used only when the TypeScript one could not read the file.
        // A `.ts` file parses cleanly the first time and never pays for this; a
        // `.tsx` file is unreadable the first time and readable the second.
        let ts = TS_PARSER.with(|p| p.borrow_mut().parse(source, None));
        if ts.as_ref().is_some_and(|t| !t.root_node().has_error()) {
            return ts;
        }
        let tsx = TSX_PARSER.with(|p| p.borrow_mut().parse(source, None));
        return match (ts, tsx) {
            // Keep whichever read the file; on a tie keep TypeScript, so a
            // genuinely broken `.ts` fragment parses as it always did.
            (Some(a), Some(b)) if b.root_node().has_error() => Some(a),
            (_, Some(b)) => Some(b),
            (a, None) => a,
        };
    }
    if language == Language::Pascal {
        // See [`blank_pascal_directives`]: offsets are preserved, so the tree
        // still addresses `source`.
        let masked = blank_pascal_directives(source, true);
        return PAS_PARSER.with(|p| p.borrow_mut().parse(masked.as_ref(), None));
    }
    if language == Language::C || language == Language::Cpp {
        let parse_with = |src: &str| match language {
            Language::C => C_PARSER.with(|p| p.borrow_mut().parse(src, None)),
            _ => CPP_PARSER.with(|p| p.borrow_mut().parse(src, None)),
        };
        let plain = parse_with(source);
        if plain.as_ref().is_some_and(|t| !t.root_node().has_error()) {
            return plain;
        }
        // See [`blank_preprocessor_conditionals`]: offsets are preserved, so
        // the tree still addresses `source`.
        let masked = blank_preprocessor_conditionals(source);
        if matches!(masked, std::borrow::Cow::Borrowed(_)) {
            return plain;
        }
        let sibling = if language == Language::C {
            Language::Cpp
        } else {
            Language::C
        };
        let parse_sibling = |src: &str| match sibling {
            Language::C => C_PARSER.with(|p| p.borrow_mut().parse(src, None)),
            _ => CPP_PARSER.with(|p| p.borrow_mut().parse(src, None)),
        };
        // `.h` is C or C++ and the extension cannot say which. `ext_to_lang_ctx`
        // resolves it by repo-level majority, which is right for the repository
        // and necessarily wrong for its minority headers — a C header in a C++
        // project reads nine times worse under the C++ grammar. Only a file the
        // routed grammar could not read pays for the second attempt.
        let candidates = [
            parse_with(masked.as_ref()),
            parse_sibling(source),
            parse_sibling(masked.as_ref()),
        ];
        // Keep whichever read the most of the file, preferring the routed
        // grammar on a tie, so a source these were not to blame for parses as
        // it always did.
        let mut best = plain;
        let mut fewest = best.as_ref().map(error_rows);
        for candidate in candidates.into_iter().flatten() {
            let rows = error_rows(&candidate);
            if fewest.is_none_or(|current| rows < current) {
                (best, fewest) = (Some(candidate), Some(rows));
            }
        }
        return best;
    }
    match language {
        Language::Python => PY_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Typescript => TS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Javascript => JS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Go => GO_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Rust => RUST_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::C => C_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Java => JAVA_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::CSharp => CS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Php => PHP_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Cpp => CPP_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Ruby => RB_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Pascal => PAS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
    }
}

/// Parse Object Pascal with **every** conditional branch standing.
///
/// [`parse`] keeps one branch, because a conditional inside a declaration makes
/// the others a broken duplicate of it and the whole unit is lost. That is right
/// for reading structure and wrong for reading dependencies: a unit named only
/// under `{$ifdef DELPHI}` is one this repository really depends on, and if the
/// model never learns it, every later `uses` of it reads as a brand-new
/// dependency — a false alarm manufactured by the parser. 73 of mORMot's 229
/// units, and 16 of Castle Game Engine's, are named only outside the first
/// branch.
///
/// The tree this returns is *not* a reliable description of the file's shape —
/// that is the trade being made — so it answers one question only: which units
/// appear. Offsets are preserved exactly as in [`parse`], so its spans address
/// the same source.
pub fn parse_pascal_every_branch(source: &str) -> Option<Tree> {
    let masked = blank_pascal_directives(source, false);
    PAS_PARSER.with(|p| p.borrow_mut().parse(masked.as_ref(), None))
}

/// The direct children of `node`, in left-to-right tree order.
///
/// tree-sitter's index accessor `Node::child(i)` takes a `u32` and costs
/// log(i) per call; a cursor walk is the idiomatic linear traversal and keeps
/// callers free of `usize`→`u32` index casts. Reverse the result for the
/// stack-based pre-order DFS the scorers use (push children reversed so they
/// pop left-to-right).
pub fn child_nodes<'t>(node: tree_sitter::Node<'t>) -> Vec<tree_sitter::Node<'t>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// The named direct children of `node`, in tree order (skips anonymous tokens).
pub fn named_child_nodes<'t>(node: tree_sitter::Node<'t>) -> Vec<tree_sitter::Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Whether `node` has an ancestor of one of `kinds` — a function declared
/// inside another function, whatever the language calls the node.
///
/// Sibling-span containment cannot answer this. Object Pascal's
/// `dbtrystringtoguid` declares two local procedures and the grammar does not
/// parse it at all — the whole rest of the unit becomes one `ERROR` node — so
/// the enclosing definition is never extracted and its children look top-level.
/// Passing `ERROR` among the kinds therefore also covers the honest case: a
/// callable recovered from inside a parse error has no known parent, and
/// nothing that depends on where it sits should claim to know.
pub fn has_ancestor_of_kind(node: tree_sitter::Node<'_>, kinds: &[&str]) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if kinds.contains(&n.kind()) {
            return true;
        }
        cur = n.parent();
    }
    false
}

/// What blanking a prose node costs: whole rows, or just the node's own span.
enum Prose {
    Rows(Vec<usize>),
    Span(usize, usize, usize),
}

/// Where a source's prose sits: rows to blank whole, plus `(row, col_start,
/// col_end)` spans to blank in place on rows that also carry code.
///
/// Masking by the *line* alone deletes the code beside a comment. That is
/// not hypothetical: `uses msedynload{,mseguiintf};` (Object Pascal, a unit
/// commented out in place) lost its whole `uses` clause, after which the
/// parser recovered the next type name as a module and the import stage
/// scored a dependency that does not exist — and the same shape under-reads
/// every language, a trailing `// note` after an import taking the import
/// with it. Masking by the *span* alone is no better: a multi-row node's
/// opening row would keep its delimiter after the rest is gone. So single-row
/// prose sharing its line with code is blanked in place, everything else by
/// the line.
#[derive(Debug, Default, Clone)]
pub struct ProseMask {
    pub rows: std::collections::HashSet<usize>,
    pub spans: Vec<(usize, usize, usize)>,
}

impl ProseMask {
    fn classify(source: &str, node: tree_sitter::Node<'_>) -> Prose {
        let (start, end) = (node.start_position(), node.end_position());
        if start.row == end.row {
            let line = source.split('\n').nth(start.row).unwrap_or("");
            let blank = |s: Option<&str>| s.is_none_or(|p| p.trim().is_empty());
            if !(blank(line.get(..start.column)) && blank(line.get(end.column..))) {
                return Prose::Span(start.row + 1, start.column, end.column);
            }
        }
        Prose::Rows((start.row + 1..=end.row + 1).collect())
    }

    /// Record one prose node.
    pub fn add(&mut self, source: &str, node: tree_sitter::Node<'_>) {
        match Self::classify(source, node) {
            Prose::Rows(rows) => self.rows.extend(rows),
            Prose::Span(r, a, b) => self.spans.push((r, a, b)),
        }
    }
}

#[cfg(test)]
mod tests;
