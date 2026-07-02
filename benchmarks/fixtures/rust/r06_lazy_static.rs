use lazy_static::lazy_static;
lazy_static! { static ref GLOBAL_CACHE: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new()); }
pub fn cache_push(s: String) { GLOBAL_CACHE.lock().unwrap().push(s); }
