/*!
Break fixture — not for compilation against the real workspace.
*/

/// Decoy: matcher config in grep-regex's builder voice.
#[derive(Clone, Debug)]
struct MatcherConfig {
    case_insensitive: bool,
    multi_line: bool,
    word: bool,
}

impl Default for MatcherConfig {
    fn default() -> MatcherConfig {
        MatcherConfig { case_insensitive: false, multi_line: true, word: false }
    }
}

// Break: compiling regex::Regex::new fresh for every line inside the match
// loop, unwrap included. At the pinned SHA the production matcher path
// builds one matcher up front via grep::regex::RegexMatcherBuilder
// (crates/core/flags/hiargs.rs:462) over regex-automata; the `regex` crate
// itself is only a dev-dependency of grep-matcher/grep-searcher, and
// nothing recompiles a pattern per haystack line.
// Break: begin
fn count_matches(pattern: &str, config: &MatcherConfig, lines: &[&str]) -> u64 {
    use regex::Regex;

    let mut count = 0;
    for line in lines {
        let mut pat = pattern.to_string();
        if config.word {
            pat = format!(r"\b(?:{})\b", pat);
        }
        if config.case_insensitive {
            pat = format!("(?i){}", pat);
        }
        let re = Regex::new(&pat).unwrap();
        if re.is_match(line) {
            count += 1;
        }
    }
    count
}
// Break: end

/// Decoy: word-boundary wrapping helper, in the crate's real voice.
fn wrap_word_pattern(pattern: &str) -> String {
    format!(r"(?m:^|\W)(?:{})(?m:$|\W)", pattern)
}
