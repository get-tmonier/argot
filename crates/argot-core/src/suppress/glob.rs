//! Glob matching for the suppression surfaces.
//!
//! Two flavours live here:
//! - [`fnmatch`] — shell-style matching where `*` crosses `/`. This is the
//!   semantics of `check --only/--exclude` and of `suppressions.yaml` path
//!   globs (one consistent user-facing glob dialect for flat patterns).
//! - [`segments_match`] — per-segment matching with `**` spanning segments,
//!   used by the gitignore-style `.argotignore` patterns.

/// Shell-style glob match (`fnmatch.fnmatch`), case-sensitive (posix
/// normcase). `*` matches any characters including `/`.
pub fn fnmatch(name: &str, pat: &str) -> bool {
    let n: Vec<char> = name.chars().collect();
    let p: Vec<char> = pat.chars().collect();
    fn rec(n: &[char], p: &[char]) -> bool {
        if p.is_empty() {
            return n.is_empty();
        }
        match p[0] {
            '*' => rec(n, &p[1..]) || (!n.is_empty() && rec(&n[1..], p)),
            '?' => !n.is_empty() && rec(&n[1..], &p[1..]),
            '[' => {
                if n.is_empty() {
                    return false;
                }
                let mut i = 1;
                let neg = i < p.len() && p[i] == '!';
                if neg {
                    i += 1;
                }
                let mut set: Vec<char> = Vec::new();
                let mut ranges: Vec<(char, char)> = Vec::new();
                let mut first = true;
                let mut closed = false;
                let mut j = i;
                while j < p.len() {
                    if p[j] == ']' && !first {
                        closed = true;
                        j += 1;
                        break;
                    }
                    if j + 2 < p.len() && p[j + 1] == '-' && p[j + 2] != ']' {
                        ranges.push((p[j], p[j + 2]));
                        j += 3;
                    } else {
                        set.push(p[j]);
                        j += 1;
                    }
                    first = false;
                }
                if !closed {
                    return n[0] == '[' && rec(&n[1..], &p[1..]);
                }
                let c = n[0];
                let mut matched =
                    set.contains(&c) || ranges.iter().any(|(a, b)| *a <= c && c <= *b);
                if neg {
                    matched = !matched;
                }
                matched && rec(&n[1..], &p[j..])
            }
            ch => !n.is_empty() && n[0] == ch && rec(&n[1..], &p[1..]),
        }
    }
    rec(&n, &p)
}

/// Match one glob segment against one path segment. Neither contains `/`, so
/// [`fnmatch`]'s `*` cannot cross a separator here.
fn segment_match(name: &str, pat: &str) -> bool {
    fnmatch(name, pat)
}

/// Match a sequence of pattern segments against a sequence of path segments,
/// with `**` spanning zero or more path segments.
pub fn segments_match(pat: &[String], path: &[&str]) -> bool {
    if pat.is_empty() {
        return path.is_empty();
    }
    if pat[0] == "**" {
        // `**` matches zero or more leading path segments.
        (0..=path.len()).any(|k| segments_match(&pat[1..], &path[k..]))
    } else {
        !path.is_empty() && segment_match(path[0], &pat[0]) && segments_match(&pat[1..], &path[1..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(p: &str) -> Vec<String> {
        p.split('/').map(String::from).collect()
    }

    #[test]
    fn fnmatch_star_crosses_slashes() {
        assert!(fnmatch("src/deep/file.py", "src/*.py"));
        assert!(fnmatch("a/b/c.ts", "*.ts"));
        assert!(!fnmatch("a/b/c.tsx", "*.ts"));
    }

    #[test]
    fn fnmatch_question_and_class() {
        assert!(fnmatch("a1.py", "a?.py"));
        assert!(fnmatch("a1.py", "a[0-9].py"));
        assert!(!fnmatch("ax.py", "a[0-9].py"));
        assert!(fnmatch("ax.py", "a[!0-9].py"));
    }

    #[test]
    fn segments_double_star_spans() {
        assert!(segments_match(
            &segs("**/vendor/**"),
            &["a", "vendor", "x.py"]
        ));
        assert!(segments_match(&segs("**/vendor/**"), &["vendor", "x.py"]));
        assert!(!segments_match(&segs("**/vendor/**"), &["src", "x.py"]));
        assert!(segments_match(&segs("src/**/gen.py"), &["src", "gen.py"]));
        assert!(segments_match(
            &segs("src/**/gen.py"),
            &["src", "a", "b", "gen.py"]
        ));
    }

    #[test]
    fn segments_star_stays_within_segment() {
        assert!(segments_match(&segs("src/*.py"), &["src", "x.py"]));
        assert!(!segments_match(&segs("src/*.py"), &["src", "a", "x.py"]));
    }
}
