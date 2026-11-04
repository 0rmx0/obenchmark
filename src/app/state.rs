use crate::model::result::BenchResult;

#[derive(Clone)]
pub enum AppState {
    Idle,
    Running {
        current_test: String,
        completed: usize,
        total: usize,
        /// Progress inside the current benchmark, 0.0..=1.0.
        current_progress: f32,
        /// Benchmarks that failed so far (run continues without them).
        failed: Vec<String>,
        /// Last finished benchmark result, for live display.
        last_result: String,
    },
    Showing(Box<BenchResult>),
    Error(String),
}
