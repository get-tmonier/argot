/*!
Break fixture — not for compilation against the real workspace.
*/

/// Decoy: candidate path wrapper in globset's voice.
#[derive(Clone, Debug)]
struct Candidate {
    path: Vec<u8>,
    basename: usize,
}

impl Candidate {
    fn new(path: &[u8]) -> Candidate {
        let basename = path
            .iter()
            .rposition(|&b| b == b'/')
            .map(|i| i + 1)
            .unwrap_or(0);
        Candidate { path: path.to_vec(), basename }
    }
}

// Break: camelCase methods and Java get_/set_ accessor shape. At the pinned
// SHA the repo is uniformly snake_case with fluent builder setters that
// take `yes: bool` and return `&mut Self` (e.g. GlobBuilder::
// case_insensitive, SearcherBuilder::line_number); there are no getFoo/
// setFoo accessors anywhere in src.
// Break: begin
#[derive(Clone, Debug, Default)]
struct GlobMatcherOptions {
    caseInsensitive: bool,
    literalSeparator: bool,
    matchCount: u64,
}

impl GlobMatcherOptions {
    fn setCaseInsensitive(&mut self, enabledFlag: bool) {
        self.caseInsensitive = enabledFlag;
    }

    fn getCaseInsensitive(&self) -> bool {
        self.caseInsensitive
    }

    fn setLiteralSeparator(&mut self, enabledFlag: bool) {
        self.literalSeparator = enabledFlag;
    }

    fn getMatchCount(&self) -> u64 {
        self.matchCount
    }

    fn incrementMatchCount(&mut self, candidatePath: &Candidate) {
        if !candidatePath.path.is_empty() {
            self.matchCount += 1;
        }
    }
}
// Break: end

/// Decoy: snake_case sibling in the crate's real voice.
fn basename_bytes(candidate: &Candidate) -> &[u8] {
    &candidate.path[candidate.basename..]
}
