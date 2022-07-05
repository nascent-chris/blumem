#[derive(Debug)]
pub struct SearchResult {
    pub module: Option<String>,
    pub results: Vec<u64>,
}
