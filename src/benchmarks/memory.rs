use std::time::Instant;
use anyhow::Result;
use crate::benchmarks::Benchmark;

pub struct MemoryBenchmark;

impl Benchmark for MemoryBenchmark {
    fn name(&self) -> &'static str {
        "Memory"
    }

    fn run(&self) -> Result<u64> {
        let size = 200_000_000;
        let mut data = vec![0u8; size];

        let start = Instant::now();
        for i in 0..size {
            data[i] = (i % 255) as u8;
        }
        let duration = start.elapsed().as_secs_f64();

        Ok((size as f64 / duration) as u64)
    }
}