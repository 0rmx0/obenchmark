use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Write, Seek, SeekFrom};
use std::time::Instant;
use std::env;
use anyhow::Result;

use crate::benchmarks::Benchmark;

pub struct DiskBenchmark;

impl DiskBenchmark {

    /// Test écriture séquentielle
    fn sequential_write(path: &std::path::Path) -> Result<u64> {
        let mut file = File::create(path)?;
        let size = 100 * 1024 * 1024; // 100 MB
        let buffer = vec![0u8; 1 * 1024 * 1024]; // 1MB block

        let start = Instant::now();
        let mut written = 0;

        while written < size {
            file.write_all(&buffer)?;
            written += buffer.len();
        }

        file.sync_all()?;
        let secs = start.elapsed().as_secs_f64();
        Ok((written as f64 / secs) as u64)
    }

    /// Test lecture séquentielle
    fn sequential_read(path: &std::path::Path) -> Result<u64> {
        let mut file = File::open(path)?;
        let size = file.metadata()?.len() as usize;
        let mut buffer = vec![0u8; 1 * 1024 * 1024]; // 1MB block

        let start = Instant::now();
        let mut read = 0;

        while read < size {
            let n = file.read(&mut buffer)?;
            if n == 0 { break; }
            read += n;
        }

        let secs = start.elapsed().as_secs_f64();
        Ok((read as f64 / secs) as u64)
    }

    /// Test IOPS (taille bloc + queue depth)
    fn iops_test(path: &std::path::Path, block_size: usize, queue_depth: usize) -> Result<u64> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let test_size = 50 * 1024 * 1024; // 50 MB
        file.set_len(test_size as u64)?;

        let mut buffer = vec![0u8; block_size];
        let ops = test_size / block_size;

        let start = Instant::now();

        for _ in 0..ops {
            // Seek random position
            let offset = fastrand::usize(..test_size - block_size);
            file.seek(SeekFrom::Start(offset as u64))?;

            // Random read/write depending on block_size test
            if block_size == 4096 {
                file.read_exact(&mut buffer)?;
            } else {
                file.write_all(&buffer)?;
            }
        }

        let secs = start.elapsed().as_secs_f64();
        Ok((ops as f64 / secs) as u64)
    }
}

impl Benchmark for DiskBenchmark {
    fn name(&self) -> &'static str {
        "Disk"
    }

    fn run(&self) -> Result<u64> {
        let path = env::temp_dir().join("obench_disk_test.tmp");

        println!("▶ Test écriture séquentielle…");
        let write_mb_s = Self::sequential_write(&path)?;
        println!("   → Write: {} MB/s", write_mb_s / 1024 / 1024);

        println!("▶ Test lecture séquentielle…");
        let read_mb_s = Self::sequential_read(&path)?;
        println!("   → Read: {} MB/s", read_mb_s / 1024 / 1024);

        println!("▶ Test IOPS 32K QD20…");
        let iops_32k = Self::iops_test(&path, 32 * 1024, 20)?;
        println!("   → 32K QD20: {} IOPS", iops_32k);

        println!("▶ Test IOPS 4K QD1…");
        let iops_4k = Self::iops_test(&path, 4 * 1024, 1)?;
        println!("   → 4K QD1: {} IOPS", iops_4k);

        remove_file(&path)?;

        // Retourne un simple indicateur global
        Ok(write_mb_s)
    }
}