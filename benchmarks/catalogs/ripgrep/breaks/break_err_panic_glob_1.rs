/*!
Break fixture — not for compilation against the real workspace.
*/

/// Decoy: glob token, in the style of the globset parser.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
    Literal(char),
    Any,
    ZeroOrMore,
    RecursivePrefix,
}

/// Decoy: options in globset's builder voice.
#[derive(Clone, Copy, Debug, Default)]
struct ParseOptions {
    case_insensitive: bool,
    literal_separator: bool,
}

// Break: panic!/expect on user-supplied pattern input. At the pinned SHA
// globset reports every malformed pattern through its own `Error` +
// `ErrorKind` (crates/globset/src/lib.rs:167) built with err()/error()
// helpers; parsing never panics on input.
// Break: begin
fn parse_glob(pattern: &str, opts: ParseOptions) -> Vec<Token> {
    if pattern.is_empty() {
        panic!("invalid glob: pattern must be non-empty");
    }
    let mut tokens = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next().expect("peeked star vanished");
                    tokens.push(Token::RecursivePrefix);
                } else {
                    tokens.push(Token::ZeroOrMore);
                }
            }
            '?' => tokens.push(Token::Any),
            '[' => panic!("invalid glob '{}': classes unsupported", pattern),
            c => {
                let c = if opts.case_insensitive {
                    c.to_lowercase().next().unwrap()
                } else {
                    c
                };
                tokens.push(Token::Literal(c));
            }
        }
    }
    tokens
}
// Break: end

/// Decoy: harmless sibling helper.
fn has_recursive_prefix(tokens: &[Token]) -> bool {
    tokens.first() == Some(&Token::RecursivePrefix)
}
