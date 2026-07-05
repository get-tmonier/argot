// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::ffi::OsString;
use std::path::PathBuf;

/// Decoy: a parsed config line in the flags/config reader's voice.
#[derive(Clone, Debug, Default)]
struct ConfigLine {
    args: Vec<OsString>,
    source: PathBuf,
}

/// Decoy: whether a config line contributed any arguments.
fn line_has_args(line: &ConfigLine) -> bool {
    !line.args.is_empty()
}

// Break: clap derive arg parser standing in for ripgrep's hand-rolled
// lexopt-based flag layer, import inside hunk. Verified foreign at the pinned
// SHA 48b0c795f4fe: `clap` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; ripgrep parses argv itself through crates/core/flags
// (lowargs/hiargs over the `lexopt` crate), never a derive-macro parser.
// Break: begin
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rg", version)]
struct CliArgs {
    #[arg(short, long)]
    ignore_case: bool,
    #[arg(value_name = "PATTERN")]
    pattern: String,
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
}

fn parse_cli_args() -> CliArgs {
    CliArgs::parse()
}
// Break: end

/// Decoy: collect the argument vectors from a batch of config lines.
fn flatten_args(lines: &[ConfigLine]) -> Vec<OsString> {
    lines.iter().flat_map(|l| l.args.clone()).collect()
}
