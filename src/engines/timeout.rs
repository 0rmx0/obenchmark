//! Timeout module for benchmark execution.
//!
//! This module provides timeout functionality to prevent benchmarks from running
//! indefinitely, ensuring reliable and predictable execution times.

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Default timeout for benchmark execution: 30 seconds.
/// This provides a good balance between allowing complex benchmarks to complete
/// and preventing hangs on problematic systems.
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

/// Error type for timeout-related failures.
#[derive(Debug, Clone)]
pub struct TimeoutError {
    pub benchmark_name: String,
    pub timeout_duration: Duration,
    pub message: String,
}

impl TimeoutError {
    /// Create a new TimeoutError.
    pub fn new(benchmark_name: &str, timeout_duration: Duration) -> Self {
        Self {
            benchmark_name: benchmark_name.to_string(),
            timeout_duration,
            message: format!(
                "Benchmark '{}' timed out after {:?}",
                benchmark_name, timeout_duration
            ),
        }
    }

    /// Get the error message.
    pub fn to_string(&self) -> String {
        self.message.clone()
    }
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TimeoutError {}

/// Result type that can be converted from TimeoutError.
pub type TimeoutResult<T> = Result<T, TimeoutError>;

/// Execute a function with a timeout.
///
/// # Arguments
/// * `benchmark_name` - Name of the benchmark for error reporting
/// * `timeout_duration` - Maximum duration to allow for execution
/// * `func` - Function to execute (takes no arguments, returns T)
///
/// # Returns
/// Result containing the function's return value or a TimeoutError if the timeout was exceeded.
pub fn run_with_timeout<T, F>(benchmark_name: &str, timeout_duration: Duration, func: F) -> TimeoutResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_clone = timed_out.clone();

    let handle = thread::spawn(move || {
        let result = func();
        if timed_out_clone.load(Ordering::Relaxed) {
            // Function was interrupted by timeout, but we still return the result
            // This handles cases where the function completed just as timeout occurred
            Err(TimeoutError::new(benchmark_name, timeout_duration))
        } else {
            Ok(result)
        }
    });

    let start_time = Instant::now();
    
    // Wait for the thread to complete or timeout
    let received_result = handle.join_timeout(timeout_duration);

    match received_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Thread panicked or was dropped
            timed_out.store(true, Ordering::Relaxed);
            Err(TimeoutError::new(benchmark_name, timeout_duration))
        }
    }
}

/// Execute a fallible function with a timeout.
///
/// # Arguments
/// * `benchmark_name` - Name of the benchmark for error reporting
/// * `timeout_duration` - Maximum duration to allow for execution
/// * `func` - Function to execute (takes no arguments, returns Result<T, E>)
///
/// # Returns
/// Result containing the function's return value or a TimeoutError if the timeout was exceeded.
pub fn run_with_timeout_fallible<T, E, F>(
    benchmark_name: &str,
    timeout_duration: Duration,
    func: F,
) -> Result<T, TimeoutError>
where
    T: Send + 'static,
    E: Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_clone = timed_out.clone();

    let handle = thread::spawn(move || {
        match func() {
            Ok(result) => {
                if timed_out_clone.load(Ordering::Relaxed) {
                    Err(TimeoutError::new(benchmark_name, timeout_duration))
                } else {
                    Ok(result)
                }
            }
            Err(_) => {
                // Original function returned an error - convert to timeout error
                // for consistency, or we could create a new error type
                Err(TimeoutError::new(benchmark_name, timeout_duration))
            }
        }
    });

    let received_result = handle.join_timeout(timeout_duration);

    match received_result {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Thread panicked
            timed_out.store(true, Ordering::Relaxed);
            Err(TimeoutError::new(benchmark_name, timeout_duration))
        }
    }
}

/// Execute a function with the default timeout.
///
/// # Arguments
/// * `benchmark_name` - Name of the benchmark for error reporting
/// * `func` - Function to execute (takes no arguments, returns T)
///
/// # Returns
/// Result containing the function's return value or a TimeoutError if the timeout was exceeded.
pub fn run_with_default_timeout<T, F>(benchmark_name: &str, func: F) -> TimeoutResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    run_with_timeout(benchmark_name, Duration::from_secs(DEFAULT_TIMEOUT_SECONDS), func)
}

/// Create a timeout error from an elapsed time.
pub fn create_timeout_error(benchmark_name: &str, elapsed: Duration) -> TimeoutError {
    TimeoutError {
        benchmark_name: benchmark_name.to_string(),
        timeout_duration: elapsed,
        message: format!(
            "Benchmark '{}' timed out after {:?}",
            benchmark_name, elapsed
        ),
    }
}

/// Check if a timeout has occurred based on start time and timeout duration.
pub fn is_timeout_exceeded(start_time: Instant, timeout_duration: Duration) -> bool {
    start_time.elapsed() >= timeout_duration
}

/// Get remaining time before timeout.
pub fn remaining_time(start_time: Instant, timeout_duration: Duration) -> Duration {
    let elapsed = start_time.elapsed();
    if elapsed >= timeout_duration {
        Duration::from_secs(0)
    } else {
        timeout_duration - elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_run_with_timeout_success() {
        let result = run_with_timeout("test", Duration::from_secs(1), || {
            std::thread::sleep(Duration::from_millis(100));
            42
        });
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_run_with_timeout_timeout() {
        let result = run_with_timeout("test_timeout", Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_secs(1));
            42
        });
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("timed out"));
        assert!(error.to_string().contains("test_timeout"));
    }

    #[test]
    fn test_run_with_timeout_fallible_success() {
        let result = run_with_timeout_fallible("test_fallible", Duration::from_secs(1), || {
            std::thread::sleep(Duration::from_millis(100));
            Ok(42)
        });
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_run_with_timeout_fallible_timeout() {
        let result = run_with_timeout_fallible("test_fallible_timeout", Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_secs(1));
            Ok(42)
        });
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }

    #[test]
    fn test_run_with_default_timeout() {
        let result = run_with_default_timeout("test_default", || {
            std::thread::sleep(Duration::from_millis(100));
            123
        });
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn test_timeout_error_display() {
        let error = TimeoutError::new("Test Benchmark", Duration::from_secs(30));
        let display = format!("{}", error);
        
        assert!(display.contains("Test Benchmark"));
        assert!(display.contains("timed out"));
    }

    #[test]
    fn test_is_timeout_exceeded() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(50));
        
        assert!(!is_timeout_exceeded(start, Duration::from_secs(1)));
        assert!(is_timeout_exceeded(start, Duration::from_millis(10)));
    }

    #[test]
    fn test_remaining_time() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(50));
        
        let remaining = remaining_time(start, Duration::from_secs(1));
        assert!(remaining > Duration::from_secs(0));
        assert!(remaining < Duration::from_secs(1));
        
        let zero_remaining = remaining_time(start, Duration::from_millis(10));
        assert_eq!(zero_remaining, Duration::from_secs(0));
    }

    #[test]
    fn test_create_timeout_error() {
        let error = create_timeout_error("Test", Duration::from_secs(30));
        
        assert_eq!(error.benchmark_name, "Test");
        assert_eq!(error.timeout_duration, Duration::from_secs(30));
        assert!(error.message.contains("Test"));
        assert!(error.message.contains("timed out"));
    }
}