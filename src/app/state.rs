use crate::model::result::BenchResult;

pub enum AppState {
    Idle,
    Running {
        current_test: String,
        completed: usize,
        total: usize,
    },
    Showing(BenchResult),
    Error(String),
}