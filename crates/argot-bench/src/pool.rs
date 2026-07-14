//! Corpus-level parallelism for the bench driver.
//!
//! Every corpus is fully independent — its own clone under `--data-dir`, its
//! own fit artifacts, its own replay — and the engine mutates no process
//! state, so corpora can run concurrently. A work-stealing index queue (not
//! contiguous chunks) keeps workers busy despite wildly uneven corpus
//! runtimes (dagster ≫ hono); results come back in input order, so reports,
//! tables, and the dashboard are byte-identical to a sequential run.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Default worker count: enough to hide the long tail without soaking every
/// core or ballooning peak RSS (each in-flight corpus holds a fitted model).
pub fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
        .min(8)
}

/// Run `f` over `items` on up to `jobs` scoped threads, returning results in
/// input order.
pub fn run_indexed<T: Sync, R: Send>(
    items: &[T],
    jobs: usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let jobs = jobs.max(1).min(items.len().max(1));
    if jobs <= 1 {
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(item);
                *slots[i].lock().expect("result slot poisoned") = Some(r);
            });
        }
    });
    slots
        .into_iter()
        .map(|m| {
            m.into_inner()
                .expect("result slot poisoned")
                .expect("worker filled every slot")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_input_order_at_any_job_count() {
        let items: Vec<usize> = (0..37).collect();
        for jobs in [1, 2, 5, 64] {
            let out = run_indexed(&items, jobs, |&i| i * 2);
            let seq: Vec<usize> = items.iter().map(|&i| i * 2).collect();
            assert_eq!(out, seq, "jobs={jobs}");
        }
    }

    #[test]
    fn empty_input_is_fine() {
        let out: Vec<usize> = run_indexed(&[] as &[usize], 4, |&i| i);
        assert!(out.is_empty());
    }
}
