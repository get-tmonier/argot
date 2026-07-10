# ID: crates/globset/src/glob.rs:419
fn literal_prefix(pat: &GlobTokens) -> Option<String> {
    if pat.opts.case_insensitive {
        return None;
    }
    let (end, need_sep) = match *pat.tokens.last()? {
        Token::ZeroOrMore => {
            if pat.opts.literal_separator {
                return None;
            }
            (pat.tokens.len() - 1, false)
        }
        Token::RecursiveSuffix => (pat.tokens.len() - 1, true),
        _ => (pat.tokens.len(), false),
    };
    let mut lit = String::new();
    for t in &pat.tokens[0..end] {
        let Token::Literal(c) = *t else { return None };
        lit.push(c);
    }
    if need_sep {
        lit.push('/');
    }
    if lit.is_empty() { None } else { Some(lit) }
}
