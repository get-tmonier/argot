use reqwest::blocking::Client;
pub fn fetch(url: &str) -> String { let c = Client::new(); c.get(url).send().unwrap().text().unwrap() }
