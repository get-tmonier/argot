pub fn load_all(paths: Vec<&str>) -> Vec<String> {
    paths.iter().map(|p| std::fs::read_to_string(p).expect("read failed")).collect()
}
