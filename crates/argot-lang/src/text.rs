//! Text utilities that must match CPython string semantics exactly.
//!
//! The extract pipeline does `blob.decode("utf-8", errors="replace")
//! .splitlines()` and later `"\n".join(lines[a:b])`. Rust's `str::lines()`
//! only recognises `\n` and `\r\n`, but CPython's `str.splitlines()`
//! recognises a wider set of Unicode line boundaries. Tokenisation slices
//! lines by index, so any divergence in line splitting shifts every
//! downstream line number and breaks parity. We therefore reimplement
//! CPython's `str.splitlines()` boundary set precisely.

/// The set of characters CPython's `str.splitlines()` treats as line
/// boundaries (keepends=False). See CPython `Objects/unicodeobject.c`
/// `STRINGLIB(splitlines)` / `Py_UNICODE_ISLINEBREAK`.
///
/// - `\n`   U+000A LINE FEED
/// - `\r`   U+000D CARRIAGE RETURN (and `\r\n` as a single boundary)
/// - `\v`   U+000B LINE TABULATION
/// - `\f`   U+000C FORM FEED
/// - `\x1c` U+001C FILE SEPARATOR
/// - `\x1d` U+001D GROUP SEPARATOR
/// - `\x1e` U+001E RECORD SEPARATOR
/// - `\x85` U+0085 NEXT LINE
/// - ` ` LINE SEPARATOR
/// - ` ` PARAGRAPH SEPARATOR
fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{001c}'
            | '\u{001d}'
            | '\u{001e}'
            | '\u{0085}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// Reimplementation of CPython's `str.splitlines()` (keepends=False).
///
/// Splits on the Unicode line-boundary set above, collapsing `\r\n` into a
/// single boundary. A trailing line boundary does NOT produce a final empty
/// element (matching CPython: `"a\n".splitlines() == ["a"]`).
pub fn splitlines(s: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut line_start = 0usize;
    let chars = s.char_indices();
    // We iterate by char to correctly handle multi-byte boundaries
    // (U+0085, U+2028, U+2029).
    let mut iter = chars.peekable();
    while let Some((idx, c)) = iter.next() {
        i = idx;
        if is_line_break(c) {
            out.push(&s[line_start..idx]);
            // Collapse CRLF.
            if c == '\r' {
                if let Some(&(_, '\n')) = iter.peek() {
                    iter.next();
                    // next char consumed; recompute line_start below
                    line_start = idx + c.len_utf8() + '\n'.len_utf8();
                    continue;
                }
            }
            line_start = idx + c.len_utf8();
        }
    }
    let _ = i;
    // Trailing content after the last boundary (no trailing empty element if
    // the string ends exactly on a boundary).
    if line_start < bytes.len() {
        out.push(&s[line_start..]);
    }
    out
}

/// CPython `str.splitlines(keepends=True)`: like [`splitlines`] but each
/// element retains its line terminator (`\r\n` kept as one). Used by the
/// scorer's prose-blanking, which must preserve line count.
pub fn splitlines_keepends(s: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let mut line_start = 0usize;
    let mut iter = s.char_indices().peekable();
    while let Some((idx, c)) = iter.next() {
        if is_line_break(c) {
            let mut end = idx + c.len_utf8();
            if c == '\r' {
                if let Some(&(ni, '\n')) = iter.peek() {
                    iter.next();
                    end = ni + '\n'.len_utf8();
                }
            }
            out.push(&s[line_start..end]);
            line_start = end;
        }
    }
    if line_start < s.len() {
        out.push(&s[line_start..]);
    }
    out
}

/// Reproduce Python `open(...).read()` / `Path.read_text()` universal-newline
/// translation: `\r\n` and lone `\r` both collapse to `\n`. Repo-corpus files
/// are read this way before BPE tokenisation, so matching it keeps token
/// counts identical on CRLF sources.
pub fn universal_newlines(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string();
    }
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Read a file with Python `read_text(encoding="utf-8", errors="replace")`
/// semantics: UTF-8 lossy decode + universal-newline translation.
pub fn read_text_lossy(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(universal_newlines(&String::from_utf8_lossy(&bytes)))
}

#[cfg(test)]
mod tests;
