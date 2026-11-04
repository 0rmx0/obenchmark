use std::hint::black_box;
use std::thread;
use std::time::Instant;

use anyhow::Result;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};

use crate::engines::benchmark::Benchmark;
use crate::util::config::{bench_duration, tick_progress};
use crate::util::sysinfo::get_system_info;

/// Default size of the large buffers used by sequential memory tests.
const DEFAULT_STREAM_MEM_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB
/// Minimum size to keep a usable signal on small machines.
const MIN_STREAM_MEM_BYTES: u64 = 64 * 1024 * 1024; // 64 MiB
/// Maximum fraction of total RAM a single memory test may use.
const MAX_FRACTION_OF_RAM: f64 = 0.25; // 25%

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// Choose a sane buffer size based on available RAM. Never exceeds half of
/// the physical RAM, even on very small machines.
fn choose_buffer_size_bytes(default: u64, min_bytes: u64) -> u64 {
    let sys = get_system_info();
    let total_bytes = sys.total_memory().saturating_mul(1024); // KB -> bytes
    if total_bytes == 0 {
        return default.max(min_bytes);
    }

    let cap_by_fraction = (total_bytes as f64 * MAX_FRACTION_OF_RAM).round() as u64;
    let half_ram = total_bytes / 2;
    let mut size = default.min(cap_by_fraction).min(half_ram);
    if size < min_bytes {
        // Shrink the floor instead of exceeding half the RAM.
        size = min_bytes.min(half_ram);
    }
    size.max(1024 * 1024) // at least 1 MiB
}

/// 1. Database-style operations (push + random reads in a vector).
pub struct MemoryDBOps;
impl Benchmark for MemoryDBOps {
    fn name(&self) -> &str {
        "Mem DB Ops"
    }

    fn run(&self) -> Result<u64> {
        let duration = bench_duration();
        let start = Instant::now();
        let mut db: Vec<u64> = Vec::with_capacity(1_000_000);
        let mut i: u64 = 0;
        let mut ops: u64 = 0;

        while tick_progress(&start, duration) {
            for _ in 0..10_000 {
                db.push(i);
                let idx = (i / 2) as usize % db.len().max(1);
                let _ = black_box(db[idx]);
                i += 1;
                ops += 2; // one push + one read
            }
            // Keep the working set bounded.
            if db.len() > 4_000_000 {
                db.truncate(1_000_000);
            }
        }
        black_box(&db);

        let elapsed = start.elapsed().as_secs_f64().max(1e-6);
        Ok((ops as f64 / elapsed) as u64)
    }
}

/// 2. Heavily cached read (small buffer read many times), timed loop.
pub struct MemoryCachedRead;
impl Benchmark for MemoryCachedRead {
    fn name(&self) -> &str {
        "Mem Cached Read"
    }

    fn run(&self) -> Result<u64> {
        let size = 8 * 1024 * 1024; // 8 MiB
        let data = vec![0u8; size];
        let duration = bench_duration();
        let start = Instant::now();
        let mut sum = 0u64;
        let mut total_bytes: u64 = 0;

        while tick_progress(&start, duration) {
            for &b in &data {
                sum = sum.wrapping_add(b as u64);
            }
            total_bytes += size as u64;
        }
        black_box(sum);

        let elapsed = start.elapsed().as_secs_f64().max(1e-6);
        let mb = bytes_to_mb(total_bytes);
        Ok((mb / elapsed) as u64)
    }
}

/// 3. Sequential read over a large buffer, repeated passes over a timed loop.
pub struct MemoryUncachedRead;
impl Benchmark for MemoryUncachedRead {
    fn name(&self) -> &str {
        "Mem Uncached Read"
    }

    fn run(&self) -> Result<u64> {
        let size_bytes = choose_buffer_size_bytes(DEFAULT_STREAM_MEM_BYTES, MIN_STREAM_MEM_BYTES);
        let size = size_bytes as usize;

        let mut data = vec![0u8; size];

        // Warm-up: touch every page so we don't measure page faults.
        for b in &mut data {
            *b = b.wrapping_add(1);
        }

        let duration = bench_duration();
        let start = Instant::now();
        let mut sum = 0u64;
        let mut total_bytes: u64 = 0;
        while tick_progress(&start, duration) {
            for &b in &data {
                sum = sum.wrapping_add(b as u64);
            }
            total_bytes += size_bytes;
        }
        black_box(sum);

        let elapsed = start.elapsed().as_secs_f64().max(1e-6);
        let mb = bytes_to_mb(total_bytes);
        Ok((mb / elapsed) as u64)
    }
}

/// 4. Sequential memory write bandwidth over a timed loop.
pub struct MemoryWrite;
impl Benchmark for MemoryWrite {
    fn name(&self) -> &str {
        "Mem Write"
    }

    fn run(&self) -> Result<u64> {
        let size_bytes = choose_buffer_size_bytes(DEFAULT_STREAM_MEM_BYTES, MIN_STREAM_MEM_BYTES);
        let size = size_bytes as usize;
        let mut data = vec![0u8; size];
        let duration = bench_duration();
        let start = Instant::now();
        let mut total_bytes: u64 = 0;

        while tick_progress(&start, duration) {
            for (i, b) in data.iter_mut().enumerate() {
                *b = (i & 0xFF) as u8;
            }
            total_bytes += size_bytes;
        }
        // Defeat aggressive loop optimization.
        black_box(&data[0..std::cmp::min(1024, size)]);

        let elapsed = start.elapsed().as_secs_f64().max(1e-6);
        let mb = bytes_to_mb(total_bytes);
        Ok((mb / elapsed) as u64)
    }
}

/// 5. Available RAM (MB) at benchmark time.
pub struct MemoryAvailable;
impl Benchmark for MemoryAvailable {
    fn name(&self) -> &str {
        "Mem Available"
    }

    fn run(&self) -> Result<u64> {
        let sys = get_system_info();
        Ok(sys.available_memory() / 1024) // MB
    }
}

/// 6. Memory latency via pointer chasing on a pseudo-random cycle.
pub struct MemoryLatency;
impl Benchmark for MemoryLatency {
    fn name(&self) -> &str {
        "Mem Latency"
    }

    fn run(&self) -> Result<u64> {
        // Working set ~80 MB on 64-bit platforms.
        let n: usize = 10_000_000;

        // Build a pseudo-random cycle over [0, n).
        let mut order: Vec<usize> = (0..n).collect();
        let mut rng = StdRng::seed_from_u64(0x4D45_4D00);
        order.shuffle(&mut rng);

        let mut next = vec![0usize; n];
        for i in 0..n {
            let from = order[i];
            let to = order[(i + 1) % n];
            next[from] = to;
        }

        let duration = bench_duration();
        let start = Instant::now();
        let mut idx = 0usize;
        let mut steps: u64 = 0;
        while tick_progress(&start, duration) {
            for _ in 0..1_000_000 {
                // Pointer chasing: each access depends on the previous one.
                idx = next[idx];
                steps += 1;
            }
        }
        black_box(idx);

        let elapsed_secs = start.elapsed().as_secs_f64().max(1e-6);
        // Accesses per second (higher = better).
        Ok((steps as f64 / elapsed_secs) as u64)
    }
}

/// 7. Multi-threaded memory bandwidth over a timed loop.
pub struct MemoryThreaded;
impl Benchmark for MemoryThreaded {
    fn name(&self) -> &str {
        "Mem Threaded"
    }

    fn run(&self) -> Result<u64> {
        let threads = num_cpus::get();
        let sys = get_system_info();
        let total_bytes = sys.total_memory().saturating_mul(1024);
        let per_thread_default: u64 = 100u64 * 1024 * 1024;
        let per_thread_cap = (total_bytes / 2) / threads.max(1) as u64;
        let size_bytes = per_thread_default.min(per_thread_cap).max(1024 * 1024);
        let size = size_bytes as usize;

        let duration = bench_duration();
        let start = Instant::now();
        let mut total_written: u64 = 0;

        while tick_progress(&start, duration) {
            let mut handles = Vec::new();
            for _ in 0..threads {
                handles.push(thread::spawn(move || {
                    let mut local = vec![0u8; size];
                    for (i, b) in local.iter_mut().enumerate() {
                        *b = (i % 255) as u8;
                    }
                    black_box(&local[0..std::cmp::min(1024, local.len())]);
                }));
            }
            for h in handles {
                h.join()
                    .map_err(|_| anyhow::anyhow!("memory worker thread panicked"))?;
            }
            total_written += size_bytes * threads as u64;
        }

        let elapsed = start.elapsed().as_secs_f64().max(1e-6);
        let mb = bytes_to_mb(total_written);
        Ok((mb / elapsed) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_size_never_exceeds_half_ram() {
        // Simulate a tiny 96 MiB machine by checking the clamp logic directly:
        // with total=96MiB, half=48MiB, result must be <= 48MiB.
        let total_bytes: u64 = 96 * 1024 * 1024;
        let cap_by_fraction = (total_bytes as f64 * MAX_FRACTION_OF_RAM).round() as u64;
        let half_ram = total_bytes / 2;
        let mut size = DEFAULT_STREAM_MEM_BYTES.min(cap_by_fraction).min(half_ram);
        if size < MIN_STREAM_MEM_BYTES {
            size = MIN_STREAM_MEM_BYTES.min(half_ram);
        }
        size = size.max(1024 * 1024);
        assert!(
            size <= half_ram,
            "size {} exceeds half RAM {}",
            size,
            half_ram
        );
    }

    #[test]
    fn buffer_size_is_at_least_1mib() {
        let size = choose_buffer_size_bytes(DEFAULT_STREAM_MEM_BYTES, MIN_STREAM_MEM_BYTES);
        assert!(size >= 1024 * 1024);
    }
}
