use std::fs::{File, remove_file};
use std::io::{Write};
use std::time::Instant;
use std::env;
use anyhow::Result;
use crate::benchmarks::Benchmark;

pub struct DiskBenchmark;

impl Benchmark for DiskBenchmark {
    fn name(&self) -> &'static str {
        "Disk"
    }

    fn run(&self) -> Result<u64> {
        let path = env::temp_dir().join("obench_test.tmp");
        let mut file = File::create(&path)?;

        let buffer = vec![0u8; 50_000_000];

        let start = Instant::now();
        file.write_all(&buffer)?;
        file.sync_all()?;
        let duration = start.elapsed().as_secs_f64();

        remove_file(&path)?;

        Ok((buffer.len() as f64 / duration) as u64)
    }
}