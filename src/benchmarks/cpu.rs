use std::thread;
use std::time::Instant;
use std::hint::black_box;
use anyhow::Result;
use num_cpus;
use crate::benchmarks::Benchmark;

pub struct CpuBenchmark;

impl CpuBenchmark {
    pub fn new() -> Self {
        Self
    }

    fn single_core_test() -> u64 {
        let iterations = 30_000_000u64;
        let mut acc = 0u64;
        let start = Instant::now();

        for i in 0..iterations {
            acc = black_box(acc.wrapping_add(i));
        }

        let duration = start.elapsed().as_secs_f64();
        (iterations as f64 / duration) as u64
    }

    fn multi_core_test() -> u64 {
        let threads = num_cpus::get();
        let iterations = 20_000_000u64;

        let start = Instant::now();

        let mut handles = Vec::new();

        for _ in 0..threads {
            handles.push(thread::spawn(move || {
                let mut acc = 0u64;
                for i in 0..iterations {
                    acc = black_box(acc.wrapping_add(i));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let duration = start.elapsed().as_secs_f64();
        ((iterations * threads as u64) as f64 / duration) as u64
    }
}

impl Benchmark for CpuBenchmark {
    fn name(&self) -> &'static str {
        "CPU MultiCore"
    }

    fn run(&self) -> Result<u64> {
        let single = Self::single_core_test();
        let multi = Self::multi_core_test();
        Ok((single + multi) / 2)
    }
}