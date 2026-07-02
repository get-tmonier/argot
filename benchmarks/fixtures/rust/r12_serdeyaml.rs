use serde_yaml;
pub fn parse_cfg(s: &str) -> serde_yaml::Value { serde_yaml::from_str(s).unwrap() }
