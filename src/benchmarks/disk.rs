use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Once;
use std::thread;
use std::time::Instant;

use anyhow::{anyhow, Result};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::engines::benchmark::Benchmark;
use crate::util::cleanup::{cleanup_stale_temp_files, unique_temp_path, TempFileGuard};
use crate::util::config::{disk_dir, disk_file_sizes};
use crate::util::fadvise::drop_page_cache;

/// Block sizes.
const SEQ_BLOCK_SIZE: usize = 1024 * 1024; // 1 MiB
const RAND_32K_BLOCK_SIZE: usize = 32 * 1024;
const RAND_4K_BLOCK_SIZE: usize = 4 * 1024;

/// Approximate queue depths (number of worker threads).
const QD_32K: usize = 20; // "QD20"
const QD_4K: usize = 1; // "QD1"

/// Minimum file size before giving up.
const MIN_FILE_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB

static CLEANUP_ONCE: Once = Once::new();

fn ensure_stale_cleanup() {
    CLEANUP_ONCE.call_once(cleanup_stale_temp_files);
}

/// Create a test file of `target_size` filled with zeros, shrinking by half
/// on disk-space errors. The returned guard deletes the file on drop.
fn create_test_file_best_effort(label: &str, target_size: u64) -> Result<(TempFileGuard, u64)> {
    ensure_stale_cleanup();

    let dir = disk_dir();
    std::fs::create_dir_all(&dir)?;

    let mut size = target_size;
    let mut last_err: Option<anyhow::Error> = None;

    while size >= MIN_FILE_SIZE {
        let path = unique_temp_path(&dir, label);
        match create_file_with_size(&path, size) {
            Ok(()) => return Ok((TempFileGuard::new(path), size)),
            Err(e) => {
                let _ = std::fs::remove_file(&path);
                last_err = Some(e);
                size /= 2;
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("not enough disk space for the disk benchmark")))
}

fn create_file_with_size(path: &Path, size: u64) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;

    // Pre-allocate + sequential write with a 1 MiB buffer.
    // This time is NOT measured.
    file.set_len(size)?;

    let buffer = vec![0u8; SEQ_BLOCK_SIZE];
    let mut remaining = size;
    while remaining > 0 {
        let to_write = std::cmp::min(remaining, buffer.len() as u64) as usize;
        file.write_all(&buffer[..to_write])?;
        remaining -= to_write as u64;
    }
    file.sync_all()?;
    // Drop the freshly written pages so reads hit the device, not RAM.
    drop_page_cache(&file);
    Ok(())
}

fn bytes_per_sec_to_mb_per_sec(bytes: u64, secs: f64) -> u64 {
    if secs <= 0.0 {
        return 0;
    }
    let mb = (bytes as f64) / (1024.0 * 1024.0);
    (mb / secs) as u64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IoMode {
    Read,
    Write,
}

/// Random I/O (read or write) at `block_size` with `queue_depth` workers.
/// Returns IOPS computed over the *measured* elapsed time.
fn random_iops(
    path: &Path,
    file_size: u64,
    block_size: usize,
    queue_depth: usize,
    mode: IoMode,
) -> Result<u64> {
    let start = Instant::now();

    let max_offset = file_size
        .checked_sub(block_size as u64)
        .ok_or_else(|| anyhow!("test file too small for the block size"))?;

    // Drop cached pages before measuring.
    if mode == IoMode::Read {
        if let Ok(f) = File::open(path) {
            drop_page_cache(&f);
        }
    }

    let mut handles = Vec::with_capacity(queue_depth);

    for thread_id in 0..queue_depth {
        let path = path.to_owned();

        let handle = thread::spawn(move || -> Result<u64> {
            let mut rng = StdRng::seed_from_u64(0xD15C_0000 + thread_id as u64);
            let mut file = match mode {
                IoMode::Read => OpenOptions::new().read(true).open(&path)?,
                IoMode::Write => OpenOptions::new().write(true).open(&path)?,
            };
            let mut buffer = vec![0u8; block_size];
            let mut ops: u64 = 0;

            loop {
                if start.elapsed() >= crate::util::config::bench_duration() {
                    break;
                }
                let offset = rng.gen_range(0..=max_offset);
                file.seek(SeekFrom::Start(offset))?;
                match mode {
                    IoMode::Read => {
                        file.read_exact(&mut buffer)?;
                    }
                    IoMode::Write => {
                        file.write_all(&buffer)?;
                    }
                }
                ops += 1;
            }
            if mode == IoMode::Write {
                file.sync_all()?;
            }

            Ok(ops)
        });

        handles.push(handle);
    }

    let mut total_ops: u64 = 0;
    for h in handles {
        let res = h
            .join()
            .map_err(|_| anyhow!("disk worker thread panicked"))?;
        total_ops += res?;
    }

    // Use the real elapsed time, not the nominal duration: workers can run
    // slightly past the deadline, which would under-report IOPS otherwise.
    let secs = start.elapsed().as_secs_f64().max(1e-6);
    Ok((total_ops as f64 / secs) as u64)
}

/// Sequential large-block read (MB/s).
pub struct DiskSequentialRead;

impl Benchmark for DiskSequentialRead {
    fn name(&self) -> &str {
        "Disk Seq Read"
    }

    fn run(&self) -> Result<u64> {
        let (seq_size, _) = disk_file_sizes();
        let (guard, size) = create_test_file_best_effort("seq_read", seq_size)?;

        let mut file = OpenOptions::new().read(true).open(guard.path())?;
        // Make sure the read is not served from the page cache.
        drop_page_cache(&file);
        let mut buffer = vec![0u8; SEQ_BLOCK_SIZE];
        let start = Instant::now();
        let mut read_bytes: u64 = 0;

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            read_bytes += n as u64;
        }

        let elapsed = start.elapsed().as_secs_f64();
        Ok(bytes_per_sec_to_mb_per_sec(
            std::cmp::min(read_bytes, size),
            elapsed,
        ))
    }
}

/// Sequential large-block write (MB/s), including the final flush.
pub struct DiskSequentialWrite;

impl Benchmark for DiskSequentialWrite {
    fn name(&self) -> &str {
        "Disk Seq Write"
    }

    fn run(&self) -> Result<u64> {
        ensure_stale_cleanup();
        let (seq_size, _) = disk_file_sizes();
        let dir = disk_dir();
        std::fs::create_dir_all(&dir)?;
        let path = unique_temp_path(&dir, "seq_write");
        let _guard = TempFileGuard::new(path.clone());

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;

        let buffer = vec![0u8; SEQ_BLOCK_SIZE];
        let mut remaining = seq_size;

        let start = Instant::now();
        let mut written: u64 = 0;
        while remaining > 0 {
            let to_write = std::cmp::min(remaining, buffer.len() as u64) as usize;
            file.write_all(&buffer[..to_write])?;
            written += to_write as u64;
            remaining -= to_write as u64;
        }
        // sync_all is inside the measured window: the score includes the flush.
        file.sync_all()?;
        let elapsed = start.elapsed().as_secs_f64();

        Ok(bytes_per_sec_to_mb_per_sec(written, elapsed))
    }
}

/// Random read IOPS, 32K blocks, queue depth ~20.
pub struct DiskRandomIOPS32K;

impl Benchmark for DiskRandomIOPS32K {
    fn name(&self) -> &str {
        "Disk IOPS 32K QD20"
    }

    fn run(&self) -> Result<u64> {
        let (_, rand_size) = disk_file_sizes();
        let (guard, size) = create_test_file_best_effort("rand_32k", rand_size)?;
        random_iops(
            guard.path(),
            size,
            RAND_32K_BLOCK_SIZE,
            QD_32K,
            IoMode::Read,
        )
    }
}

/// Random read IOPS, 4K blocks, queue depth 1 (latency-bound).
pub struct DiskRandomIOPS4K;

impl Benchmark for DiskRandomIOPS4K {
    fn name(&self) -> &str {
        "Disk IOPS 4K QD1"
    }

    fn run(&self) -> Result<u64> {
        let (_, rand_size) = disk_file_sizes();
        let (guard, size) = create_test_file_best_effort("rand_4k", rand_size)?;
        random_iops(guard.path(), size, RAND_4K_BLOCK_SIZE, QD_4K, IoMode::Read)
    }
}

/// Random write IOPS, 32K blocks, queue depth ~20.
pub struct DiskRandomWrite32K;

impl Benchmark for DiskRandomWrite32K {
    fn name(&self) -> &str {
        "Disk IOPS 32K QD20 Write"
    }

    fn run(&self) -> Result<u64> {
        let (_, rand_size) = disk_file_sizes();
        let (guard, size) = create_test_file_best_effort("randw_32k", rand_size)?;
        random_iops(
            guard.path(),
            size,
            RAND_32K_BLOCK_SIZE,
            QD_32K,
            IoMode::Write,
        )
    }
}

/// Random write IOPS, 4K blocks, queue depth 1.
pub struct DiskRandomWrite4K;

impl Benchmark for DiskRandomWrite4K {
    fn name(&self) -> &str {
        "Disk IOPS 4K QD1 Write"
    }

    fn run(&self) -> Result<u64> {
        let (_, rand_size) = disk_file_sizes();
        let (guard, size) = create_test_file_best_effort("randw_4k", rand_size)?;
        random_iops(guard.path(), size, RAND_4K_BLOCK_SIZE, QD_4K, IoMode::Write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mb_per_sec_conversion() {
        assert_eq!(bytes_per_sec_to_mb_per_sec(1024 * 1024, 1.0), 1);
        assert_eq!(bytes_per_sec_to_mb_per_sec(512 * 1024 * 1024, 2.0), 256);
        assert_eq!(bytes_per_sec_to_mb_per_sec(1024, 0.0), 0);
    }

    #[test]
    fn io_mode_is_copy() {
        let m = IoMode::Read;
        let _ = m;
        assert_eq!(m, IoMode::Read);
    }
}
