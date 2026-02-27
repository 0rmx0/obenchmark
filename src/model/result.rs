use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct BenchScore {
    pub name: String,
    pub raw_score: u64,
    pub weight: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BenchResult {
    pub scores: Vec<BenchScore>,
    pub final_score: u64,
}