use crate::model::result::BenchResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub date: String,
    pub result: BenchResult,
}
