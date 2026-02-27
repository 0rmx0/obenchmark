use std::time::Instant;
use anyhow::Result;
use crate::engines::benchmark::Benchmark;

pub struct MemoryBenchmark;

impl Benchmark for MemoryBenchmark {
    fn name(&self) -> &str { "Memory Bandwidth" }
    fn weight(&self) -> u64 { 3 }

    fn run(&self) -> Result<u64> {
        let size = 512 * 1024 * 1024;
        let mut data = vec![0u8; size];

        let start_write = Instant::now();
        for i in 0..size {
            data[i] = (i % 255) as u8;
        }
        let write_time = start_write.elapsed().as_secs_f64();

        let start_read = Instant::now();
        let mut _sum = 0u64;
        for &b in &data {
            _sum += b as u64;
        }
        let read_time = start_read.elapsed().as_secs_f64();

        let bandwidth = (size as f64 / (write_time + read_time)) / 1_000_000.0;
        Ok(bandwidth as u64)
    }
}