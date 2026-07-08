# ID: crates/matcher/src/interpolate.rs:97
fn scan_capture_ref(replacement: &[u8]) -> Option<CaptureRef<'_>> {
    if replacement.len() <= 1 || replacement[0] != b'$' {
        return None;
    }
    let mut i = 1;
    let brace = replacement[i] == b'{';
    if brace {
        i += 1;
    }
    let mut cap_end = i;
    while replacement.get(cap_end).map_or(false, is_valid_cap_letter) {
        cap_end += 1;
    }
    if cap_end == i {
        return None;
    }
    let cap = std::str::from_utf8(&replacement[i..cap_end])
        .expect("valid UTF-8 capture name");
    if brace {
        if !replacement.get(cap_end).map_or(false, |&b| b == b'}') {
            return None;
        }
        cap_end += 1;
    }
    let cap = match cap.parse::<u32>() {
        Ok(n) => Ref::Number(n as usize),
        Err(_) => Ref::Named(cap),
    };
    Some(CaptureRef { cap, end: cap_end })
}
