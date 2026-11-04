use crossbeam_channel::Sender;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const DEFAULT_BENCH_DURATION: Duration = Duration::from_secs(5);
pub const QUICK_BENCH_DURATION: Duration = Duration::from_secs(1);

/// Per-run configuration shared by every benchmark.
#[derive(Clone)]
pub struct RunConfig {
    /// Duration of each timed benchmark loop.
    pub bench_duration: Duration,
    /// Directory where disk benchmarks create their test files.
    pub disk_dir: PathBuf,
    /// When true, disk test files are smaller (fast smoke runs).
    pub quick: bool,
    /// Optional sink for intra-benchmark progress (0.0..=1.0).
    pub progress_tx: Option<Sender<f32>>,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig {
            bench_duration: DEFAULT_BENCH_DURATION,
            disk_dir: std::env::temp_dir(),
            quick: false,
            progress_tx: None,
        }
    }
}

static CONFIG: OnceLock<RwLock<RunConfig>> = OnceLock::new();
static VERBOSE: AtomicBool = AtomicBool::new(false);

fn lock() -> &'static RwLock<RunConfig> {
    CONFIG.get_or_init(|| RwLock::new(RunConfig::default()))
}

pub fn set_run_config(cfg: RunConfig) {
    *lock().write().unwrap() = cfg;
}

pub fn with_run_config<R>(f: impl FnOnce(&RunConfig) -> R) -> R {
    f(&lock().read().unwrap())
}

pub fn bench_duration() -> Duration {
    with_run_config(|c| c.bench_duration)
}

pub fn disk_dir() -> PathBuf {
    with_run_config(|c| c.disk_dir.clone())
}

pub fn is_quick() -> bool {
    with_run_config(|c| c.quick)
}

/// Sequential / random disk test file sizes, shrunk in quick mode.
pub fn disk_file_sizes() -> (u64, u64) {
    if is_quick() {
        (64 * 1024 * 1024, 256 * 1024 * 1024)
    } else {
        (512 * 1024 * 1024, 1024 * 1024 * 1024)
    }
}

pub fn set_verbose(v: bool) {
    VERBOSE.store(v, Ordering::Relaxed);
}

pub fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Returns true while the timed loop should continue, reporting progress.
pub fn tick_progress(start: &Instant, duration: Duration) -> bool {
    let elapsed = start.elapsed();
    let frac = if duration.is_zero() {
        1.0
    } else {
        (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0)
    };
    let tx = with_run_config(|c| c.progress_tx.clone());
    if let Some(tx) = tx {
        tx.send(frac as f32).ok();
    }
    elapsed < duration
}
