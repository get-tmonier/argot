# ID: crates/printer/src/util.rs:535
fn strip_trailing_terminator<'b>(
    searcher: &Searcher,
    buf: &'b [u8],
    line: &mut Match,
) -> &'b [u8] {
    let lineterm = searcher.line_terminator();
    if !lineterm.is_suffix(&buf[*line]) {
        return &[];
    }
    let mut end = line.end() - 1;
    if lineterm.is_crlf() && end > 0 && buf.get(end - 1) == Some(&b'\r') {
        end -= 1;
    }
    let orig_end = line.end();
    *line = line.with_end(end);
    &buf[end..orig_end]
}
