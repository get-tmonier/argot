use rusqlite::Connection;
pub fn open_db(path: &str) -> Connection { Connection::open(path).expect("db open failed") }
