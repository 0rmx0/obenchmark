use std::fs::File;
use std::io::{Write, Read};
use std::time::Instant;
use anyhow::Result;
use crate::engines::benchmark::Benchmark;

pub struct DiskBenchmark;

impl Benchmark for DiskBenchmark {
    fn name(&self) -> &str { "Disk Sequential" }
    fn weight(&self) -> u64 { 2 }

    fn run(&self) -> Result<u64> {
        let size = 256 * 1024 * 1024;
        let data = vec![1u8; size];

        let start_write = Instant::now();
        let mut file = File::create("benchmark_temp.dat")?;
        file.write_all(&data)?;
        file.sync_all()?;
        let write_time = start_write.elapsed().as_secs_f64();

        let start_read = Instant::now();
        let mut file = File::open("benchmark_temp.dat")?;
        let mut buffer = vec![0u8; size];
        file.read_exact(&mut buffer)?;
        let read_time = start_read.elapsed().as_secs_f64();

        std::fs::remove_file("benchmark_temp.dat")?;

        let speed_mb_s = (size as f64 / (write_time + read_time)) / 1_000_000.0;
        Ok(speed_mb_s as u64)
    }
}