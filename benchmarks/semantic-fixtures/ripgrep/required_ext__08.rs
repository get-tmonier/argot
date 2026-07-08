# ID: crates/globset/src/glob.rs:390
fn necessary_extension(pat: &GlobTokens) -> Option<String> {
    if pat.opts.case_insensitive {
        return None;
    }
    let mut ext: Vec<char> = vec![];
    for t in pat.tokens.iter().rev() {
        match *t {
            Token::Literal('/') => return None,
            Token::Literal(c) => {
                ext.push(c);
                if c == '.' {
                    break;
                }
            }
            _ => return None,
        }
    }
    if ext.last() != Some(&'.') {
        return None;
    }
    ext.reverse();
    Some(ext.into_iter().collect())
}
