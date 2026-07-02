use chrono::Utc;
pub fn timestamp() -> String { Utc::now().to_rfc3339() }
