//! `json.dumps`-compatible JSON serialisation.
//!
//! Output JSON follows the `json.dumps` convention, whose defaults differ from
//! serde_json's compact output in two parity-relevant ways:
//!   1. Separators are `", "` (items) and `": "` (key/value) — spaces that
//!      serde_json omits.
//!   2. `ensure_ascii=True`: every non-ASCII scalar is escaped as `\uXXXX`
//!      (lowercase hex; surrogate pairs for astral chars) rather than emitted
//!      as UTF-8.
//!
//! Reproducing both is required for byte-identical `dataset.jsonl`.

use serde::Serialize;
use serde_json::ser::{Formatter, Serializer};
use std::io;

/// serde_json [`Formatter`] that matches `json.dumps(obj)` defaults
/// (compact, `ensure_ascii=True`, spaced separators).
#[derive(Clone, Debug, Default)]
pub struct PythonFormatter;

fn write_unicode_escape<W: ?Sized + io::Write>(writer: &mut W, ch: char) -> io::Result<()> {
    let c = ch as u32;
    if c <= 0xFFFF {
        write!(writer, "\\u{c:04x}")
    } else {
        // Astral plane → UTF-16 surrogate pair, matching CPython.
        let v = c - 0x10000;
        let hi = 0xD800 + (v >> 10);
        let lo = 0xDC00 + (v & 0x3FF);
        write!(writer, "\\u{hi:04x}\\u{lo:04x}")
    }
}

impl Formatter for PythonFormatter {
    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }

    fn write_string_fragment<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        // serde_json hands us runs of characters it did not itself escape
        // (i.e. everything except `"`, `\`, and control bytes). Non-ASCII
        // scalars live here; escape them to match ensure_ascii=True.
        let bytes = fragment.as_bytes();
        let mut start = 0usize;
        for (i, ch) in fragment.char_indices() {
            if (ch as u32) >= 0x80 {
                if start < i {
                    writer.write_all(&bytes[start..i])?;
                }
                write_unicode_escape(writer, ch)?;
                start = i + ch.len_utf8();
            }
        }
        if start < bytes.len() {
            writer.write_all(&bytes[start..])?;
        }
        Ok(())
    }
}

/// Serialise `value` as compact JSON matching `json.dumps(value)`.
pub fn to_string<T: Serialize>(value: &T) -> serde_json::Result<String> {
    let mut buf = Vec::with_capacity(128);
    let mut ser = Serializer::with_formatter(&mut buf, PythonFormatter);
    value.serialize(&mut ser)?;
    // Serialisation only writes valid UTF-8.
    Ok(String::from_utf8(buf).expect("serde_json produced valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn spaced_separators() {
        let v = json!({"a": 1, "b": [2, 3]});
        assert_eq!(to_string(&v).unwrap(), r#"{"a": 1, "b": [2, 3]}"#);
    }

    #[test]
    fn ensure_ascii_escapes_non_ascii() {
        // Python: json.dumps("café") == '"caf\\u00e9"'
        let v = json!("café");
        assert_eq!(to_string(&v).unwrap(), "\"caf\\u00e9\"");
    }

    #[test]
    fn ensure_ascii_surrogate_pair() {
        // Python: json.dumps("😀") == '"\\ud83d\\ude00"'
        let v = json!("😀");
        assert_eq!(to_string(&v).unwrap(), "\"\\ud83d\\ude00\"");
    }

    #[test]
    fn control_chars_use_short_escapes() {
        // Python: json.dumps("a\n\tb") == '"a\\n\\tb"'
        let v = json!("a\n\tb");
        assert_eq!(to_string(&v).unwrap(), r#""a\n\tb""#);
    }

    #[test]
    fn null_and_bools() {
        let v = json!({"x": null, "y": true});
        assert_eq!(to_string(&v).unwrap(), r#"{"x": null, "y": true}"#);
    }
}
