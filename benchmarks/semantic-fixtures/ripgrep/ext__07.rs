# ID: crates/globset/src/glob.rs:353
fn sufficient_extension(pat: &GlobTokens) -> Option<String> {
    if pat.opts.case_insensitive {
        return None;
    }
    let start = match *pat.tokens.get(0)? {
        Token::RecursivePrefix => 1,
        _ => 0,
    };
    match *pat.tokens.get(start)? {
        Token::ZeroOrMore => {
            if start == 0 && pat.opts.literal_separator {
                return None;
            }
        }
        _ => return None,
    }
    if !matches!(*pat.tokens.get(start + 1)?, Token::Literal('.')) {
        return None;
    }
    let mut lit = ".".to_string();
    for t in pat.tokens[start + 2..].iter() {
        match *t {
            Token::Literal('.') | Token::Literal('/') => return None,
            Token::Literal(c) => lit.push(c),
            _ => return None,
        }
    }
    if lit.is_empty() { None } else { Some(lit) }
}
