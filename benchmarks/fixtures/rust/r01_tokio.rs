use tokio::runtime::Runtime;
pub fn run_async(f: impl std::future::Future<Output = ()>) { let rt = Runtime::new().unwrap(); rt.block_on(f); }
