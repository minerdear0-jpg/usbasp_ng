use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Verdict {
    pub result: &'static str,
    pub likely: String,
    pub supported_by: Vec<String>,
    pub not_proven: Vec<String>,
}
