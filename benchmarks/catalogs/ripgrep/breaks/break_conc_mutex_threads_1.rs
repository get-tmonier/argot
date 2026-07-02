/*!
Break fixture — not for compilation against the real workspace.
*/

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Decoy: match result carried per file, in the ignore crate's voice.
#[derive(Debug, Clone)]
struct MatchedPath {
    path: PathBuf,
    is_dir: bool,
}

/// Decoy: filter in the dir matcher's style.
fn is_hidden(path: &PathBuf) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

// Break: raw thread::spawn fan-out over Arc<Mutex<...>> shared state. At
// the pinned SHA the parallel walker is crossbeam-deque work stealing with
// a visitor (crates/ignore/src/walk.rs uses crossbeam_deque::Stealer and
// WalkParallel); Arc<Mutex<Vec<..>>> accumulation appears only in walk.rs
// tests (line 2116), never in production.
// Break: begin
fn collect_parallel(paths: Vec<PathBuf>, threads: usize) -> Vec<MatchedPath> {
    let queue = Arc::new(Mutex::new(paths));
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::with_capacity(threads);
    for _ in 0..threads {
        let queue = Arc::clone(&queue);
        let results = Arc::clone(&results);
        handles.push(thread::spawn(move || loop {
            let path = match queue.lock().unwrap().pop() {
                Some(path) => path,
                None => break,
            };
            if is_hidden(&path) {
                continue;
            }
            let is_dir = path.is_dir();
            results.lock().unwrap().push(MatchedPath { path, is_dir });
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}
// Break: end

/// Decoy: sequential counterpart.
fn collect_sequential(paths: Vec<PathBuf>) -> Vec<MatchedPath> {
    paths
        .into_iter()
        .filter(|p| !is_hidden(p))
        .map(|path| MatchedPath { is_dir: path.is_dir(), path })
        .collect()
}
