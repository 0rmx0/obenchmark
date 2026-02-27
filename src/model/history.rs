use serde::{Deserialize, Serialize};
use crate::model::result::BenchResult;

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub date: String,
    pub result: BenchResult,
}