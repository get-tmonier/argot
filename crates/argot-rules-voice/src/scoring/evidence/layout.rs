//! Shared layout primitives for the per-reason evidence formatters.
//!
//! Centralised so all three formatters render overflow / frequency / floor /
//! rarity wording identically. The UX caps and floors are the constants here.

use super::types::{CommonEntry, RarityStat};

/// Visible-head cap for the `↳` names line.
pub const TOP_K_NAMES: usize = 3;
/// Visible-head cap for the `common here:` line.
pub const TOP_K_COMMON_HERE: usize = 3;
/// The top-1 `common here` entry must occur at least this many times.
pub const COMMON_HERE_FLOOR: i64 = 3;
/// Show "0 of N" only when N is at least this (else "never seen in ...").
pub const RARITY_DENOMINATOR_FLOOR: i64 = 30;

/// Python `format(n, ",")` — thousands separated by comma. Non-negative counts
/// are the norm; a leading sign is preserved for completeness.
fn comma_int(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// Render `items` as `"a, b, c"` or `"a, b, c (+N more)"`. The `(+N more)`
/// suffix appears only when the input is strictly longer than `k`.
pub fn truncate_with_overflow(items: &[String], k: usize) -> String {
    if items.len() <= k {
        return items.join(", ");
    }
    let head = items[..k].join(", ");
    let overflow = items.len() - k;
    format!("{head} (+{overflow} more)")
}

/// Compact frequency suffix: `847` → `"847×"`; `3200` → `"3,200×"`.
pub fn format_frequency(count: i64) -> String {
    format!("{}×", comma_int(count))
}

/// Whether the `common here:` line is informative — suppressed when even the
/// top entry occurs fewer than [`COMMON_HERE_FLOOR`] times.
pub fn should_show_common_here(entries: &[CommonEntry]) -> bool {
    !entries.is_empty() && entries[0].count >= COMMON_HERE_FLOOR
}

/// Render the rarity fragment to the right of the `↳` names. Above
/// [`RARITY_DENOMINATOR_FLOOR`]: `"0 of 12,400 identifiers in repo"`. Below it:
/// `"never seen in {where}"`.
pub fn format_rarity(rarity: &RarityStat) -> String {
    if rarity.attested_total < RARITY_DENOMINATOR_FLOOR {
        format!("never seen in {}", rarity.where_)
    } else {
        format!(
            "{} of {} {}",
            rarity.flagged_count,
            comma_int(rarity.attested_total),
            rarity.scope_label()
        )
    }
}

/// Render the `common here:` line body — caller prepends the label. Each entry
/// becomes `"<name> (<count>×)"`; the `(+N more)` suffix appears when there are
/// more than `k` entries.
pub fn format_common_here_line(entries: &[CommonEntry], k: usize) -> String {
    let head = &entries[..k.min(entries.len())];
    let body = head
        .iter()
        .map(|e| format!("{} ({})", e.name, format_frequency(e.count)))
        .collect::<Vec<_>>()
        .join(", ");
    let overflow = entries.len() - head.len();
    if overflow > 0 {
        format!("{body} (+{overflow} more)")
    } else {
        body
    }
}

#[cfg(test)]
mod tests;
