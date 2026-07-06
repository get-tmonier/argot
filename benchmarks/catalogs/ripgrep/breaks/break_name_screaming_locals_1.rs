/*!
Break fixture — not for compilation against the real workspace.
*/

/// Decoy: sunk bytes trimming in printer util's voice.
fn trim_ascii_prefix(slice: &[u8]) -> usize {
    slice
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

/// Decoy: replacement byte substitution, mirroring util.rs helpers.
fn replace_bytes(slice: &mut [u8], src: u8, replacement: u8) {
    for b in slice.iter_mut() {
        if *b == src {
            *b = replacement;
        }
    }
}

// Break: SCREAMING_SNAKE locals and Hungarian-prefixed identifiers. At the
// pinned SHA locals are lower snake_case throughout; upper-case names are
// reserved for true consts/statics, and no identifier carries str/u/n type
// prefixes anywhere in crates/printer/src.
// Break: begin
#[allow(non_snake_case)]
fn summarize_offsets(strHaystackName: &str, uOffsets: &[u64]) -> String {
    let N_TOTAL = uOffsets.len();
    let mut U_MAX_OFFSET = 0u64;
    let mut U_SUM = 0u64;
    for &uOffset in uOffsets {
        if uOffset > U_MAX_OFFSET {
            U_MAX_OFFSET = uOffset;
        }
        U_SUM += uOffset;
    }
    let STR_LABEL = format!("{}[{}]", strHaystackName, N_TOTAL);
    format!("{} max={} sum={}", STR_LABEL, U_MAX_OFFSET, U_SUM)
}
// Break: end

/// Decoy: snake_case sibling in the crate's real voice.
fn max_offset(offsets: &[u64]) -> Option<u64> {
    offsets.iter().copied().max()
}
