// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: whether an asset blob carries the expected magic, in the assets voice.
fn has_asset_magic(raw: &[u8]) -> bool {
    raw.len() >= 4 && &raw[..4] == b"bat\0"
}

// Break: zstd streaming decoder reached through a SUBMODULE import
// (`use zstd::stream::read::Decoder`) decompressing a theme blob. Verified
// foreign at the pinned SHA 78951393e29b: `zstd` = 0 grep hits across *.rs and
// absent from Cargo.toml; bat decompresses its embedded syntax/theme assets
// with `flate2` (COMPRESS_THEMES / from_binary in assets.rs), never zstd.
// Break: begin
use zstd::stream::read::Decoder;

fn decompress_theme_blob(raw: &[u8]) -> Vec<u8> {
    let mut decoder = Decoder::new(raw).expect("failed to init zstd decoder");
    let mut out = Vec::new();
    std::io::copy(&mut decoder, &mut out).expect("zstd decompression failed");
    out
}
// Break: end

/// Decoy: the uncompressed size hint for a theme blob, in the assets voice.
fn theme_blob_size_hint(raw: &[u8]) -> usize {
    raw.len().saturating_mul(3)
}
