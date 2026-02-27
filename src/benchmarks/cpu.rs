use rayon::prelude::*;
use std::time::Instant;
use anyhow::Result;
use crate::engines::benchmark::Benchmark;

pub struct CpuBenchmark;

impl Benchmark for CpuBenchmark {
    fn name(&self) -> &str { "CPU Multi-Core" }
    fn weight(&self) -> u64 { 5 }

    fn run(&self) -> Result<u64> {
        let start = Instant::now();
        let duration_secs = 5;

        let mut iterations: u64 = 0;

        while start.elapsed().as_secs() < duration_secs {
            let batch: u64 = (0..1_000_000)
                .into_par_iter()
                .map(|i: u64| i.wrapping_mul(6364136223846793005).wrapping_add(1))
                .count() as u64;
            iterations += batch;
        }

        let elapsed = start.elapsed().as_secs_f64();
        Ok((iterations as f64 / elapsed) as u64)
    }
}