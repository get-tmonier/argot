/*!
Break fixture — not for compilation against the real workspace.
*/

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Decoy: a unit of search work, in the core search worker's voice.
#[derive(Debug)]
struct WorkItem {
    haystack_id: u64,
    payload: Vec<u8>,
}

/// Decoy: aggregate stats, mirroring grep-printer's Stats shape.
#[derive(Debug, Default)]
struct SearchStats {
    searches: u64,
    searches_with_match: u64,
}

// Break: busy-wait polling with thread::sleep against a hand-rolled
// Mutex<VecDeque> queue. At the pinned SHA there is no thread::sleep
// anywhere in production src; cross-thread handoff is crossbeam-channel
// (crates/core/main.rs print thread) or the crossbeam-deque walker.
// Break: begin
fn drain_results(
    queue: Arc<Mutex<VecDeque<WorkItem>>>,
    done: Arc<AtomicBool>,
) -> SearchStats {
    let mut stats = SearchStats::default();
    loop {
        let item = queue.lock().unwrap().pop_front();
        match item {
            Some(item) => {
                stats.searches += 1;
                if !item.payload.is_empty() {
                    stats.searches_with_match += 1;
                }
            }
            None => {
                if done.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
    stats
}
// Break: end

/// Decoy: id formatting helper.
fn format_haystack_id(item: &WorkItem) -> String {
    format!("haystack-{:08}", item.haystack_id)
}
