static mut COUNTER: usize = 0;
pub fn bump() { unsafe { COUNTER += 1; if COUNTER % 100 == 0 { println!("count {}", COUNTER); } } }
