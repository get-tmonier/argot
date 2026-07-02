use std::thread;
use std::time::Duration;
pub fn poll(done: &std::sync::atomic::AtomicBool) {
    while !done.load(std::sync::atomic::Ordering::Relaxed) { thread::sleep(Duration::from_millis(50)); }
}
