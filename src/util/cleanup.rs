use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static TEMP_FILES: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn registry() -> &'static Mutex<Vec<PathBuf>> {
    TEMP_FILES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Build a collision-free temp file path: `obenchmark-<label>-<pid>-<n>.bin`.
pub fn unique_temp_path(dir: &Path, label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(
        "obenchmark-{}-{}-{:04}.bin",
        label,
        std::process::id(),
        n
    ))
}

/// RAII guard that always removes the test file, even on error or panic.
pub struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        registry().lock().unwrap().push(path.clone());
        TempFileGuard { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        registry().lock().unwrap().retain(|p| p != &self.path);
    }
}

fn parse_pid(name: &str) -> Option<u32> {
    // obenchmark-<label>-<pid>-<counter>.bin (labels contain no '-')
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() == 4 && parts[0] == "obenchmark" {
        parts[2].parse().ok()
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

#[cfg(not(target_os = "linux"))]
fn pid_alive(_pid: u32) -> bool {
    // Conservative: never delete files we cannot verify are stale.
    true
}

/// Best-effort removal of leftover test files from dead processes.
pub fn cleanup_stale_temp_files() {
    let dir = std::env::temp_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("obenchmark-") && name.ends_with(".bin") {
            if let Some(pid) = parse_pid(&name) {
                if !pid_alive(pid) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn cleanup_all_registered() {
    let paths: Vec<PathBuf> = registry().lock().unwrap().drain(..).collect();
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}

/// Install SIGINT/SIGTERM handlers so test files are removed on Ctrl-C.
#[cfg(unix)]
pub fn install_signal_handlers() {
    type SigHandler = extern "C" fn(i32);
    extern "C" {
        fn signal(signum: i32, handler: SigHandler) -> isize;
    }
    extern "C" fn handler(sig: i32) {
        cleanup_all_registered();
        std::process::exit(if sig == 15 { 143 } else { 130 });
    }
    unsafe {
        signal(2, handler); // SIGINT
        signal(15, handler); // SIGTERM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_temp_path_has_pid_and_counter() {
        let p1 = unique_temp_path(Path::new("/tmp"), "seq_read");
        let p2 = unique_temp_path(Path::new("/tmp"), "seq_read");
        assert_ne!(p1, p2);
        let name = p1.file_name().unwrap().to_string_lossy().to_string();
        assert!(name.starts_with("obenchmark-seq_read-"));
        assert!(name.ends_with(".bin"));
        assert_eq!(parse_pid(&name), Some(std::process::id()));
    }

    #[test]
    fn guard_removes_file_on_drop() {
        let path = unique_temp_path(std::env::temp_dir().as_path(), "guard_test");
        std::fs::write(&path, b"x").unwrap();
        {
            let _guard = TempFileGuard::new(path.clone());
            assert!(path.exists());
        }
        assert!(!path.exists());
    }

    #[test]
    fn parse_pid_rejects_garbage() {
        assert_eq!(parse_pid("obenchmark-x-1-0001.bin"), Some(1));
        assert_eq!(parse_pid("random.bin"), None);
        assert_eq!(parse_pid("obenchmark-a-b-0001.bin"), None);
    }
}
