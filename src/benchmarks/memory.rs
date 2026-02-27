use std::time::Instant;
use std::thread;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use anyhow::Result;
use crate::benchmarks::Benchmark;

pub struct MemoryBenchmark;

impl MemoryBenchmark {

    /// Lecture mémoire en cache (accès séquentiels)
    fn cached_read(size: usize) -> u64 {
        let data = vec![1u8; size];
        let start = Instant::now();
        let mut sum: u64 = 0;
        for v in &data {
            sum += *v as u64;
        }
        (size as f64 / start.elapsed().as_secs_f64()) as u64
    }

    /// Lecture mémoire non-cachée (accès aléatoires)
    fn uncached_read(size: usize) -> u64 {
        let data = vec![1u8; size];
        let mut rng = fastrand::Rng::new();
        let start = Instant::now();
        let mut sum = 0u64;

        for _ in 0..size {
            let idx = rng.usize(..size);
            sum += data[idx] as u64;
        }
        (size as f64 / start.elapsed().as_secs_f64()) as u64
    }

    /// Écriture mémoire
    fn memory_write(size: usize) -> u64 {
        let mut data = vec![0u8; size];
        let start = Instant::now();
        for i in 0..size {
            data[i] = (i % 255) as u8;
        }
        (size as f64 / start.elapsed().as_secs_f64()) as u64
    }

    /// Mémoire disponible (approx via système)
    fn available_ram() -> u64 {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();
    sys.available_memory() * 1024
}

    /// Latence mémoire : mesure l'accès à un seul élément
    fn memory_latency() -> u64 {
        let data = vec![0u8; 1024 * 1024];
        let start = Instant::now();
        let mut cnt = 0u64;

        for _ in 0..2_000_000 {
            cnt += data[fastrand::usize(..data.len())] as u64;
        }

        let nanos = start.elapsed().as_nanos() as f64 / 2_000_000f64;
        nanos as u64
    }

    /// Mémoire filée (multithread)
    fn threaded_memory(size: usize, threads: usize) -> u64 {
        let block = size / threads;
        let mut handles = vec![];
        let total = Arc::new(AtomicU64::new(0));

        for _ in 0..threads {
            let total = total.clone();
            handles.push(thread::spawn(move || {
                let mut data = vec![1u8; block];
                let mut sum = 0u64;
                for v in &data {
                    sum += *v as u64;
                }
                total.fetch_add(sum, Ordering::Relaxed);
            }));
        }

        let start = Instant::now();
        for h in handles { h.join().unwrap(); }

        (size as f64 / start.elapsed().as_secs_f64()) as u64
    }

    /// Simulation de workload type “base de données”
    /// → Accès aléatoires sur une grande structure
    fn database_ops(size: usize) -> u64 {
        let data = vec![1u8; size];
        let mut rng = fastrand::Rng::new();

        let start = Instant::now();
        let mut ops = 0;

        for _ in 0..(size / 16) {
            let idx = rng.usize(..size);
            let _ = data[idx];
            ops += 1;
        }

        (ops as f64 / start.elapsed().as_secs_f64()) as u64
    }
}

impl Benchmark for MemoryBenchmark {
    fn name(&self) -> &'static str {
        "Memory"
    }

    fn run(&self) -> Result<u64> {
        let size = 200_000_000; // 200 MB

        println!("▶ Lecture cache…");
        let cached = Self::cached_read(size);
        println!("   → {} bytes/s", cached);

        println!("▶ Lecture non cache…");
        let uncached = Self::uncached_read(size);
        println!("   → {} bytes/s", uncached);

        println!("▶ Écriture mémoire…");
        let write = Self::memory_write(size);
        println!("   → {} bytes/s", write);

        println!("▶ Latence mémoire…");
        let latency = Self::memory_latency();
        println!("   → {} ns / accès", latency);

        println!("▶ Mémoire multi-thread…");
        let threaded = Self::threaded_memory(size, num_cpus::get());
        println!("   → {} bytes/s", threaded);

        println!("▶ Workload type BDD…");
        let db_ops = Self::database_ops(size);
        println!("   → {} ops/s", db_ops);

        println!("▶ RAM disponible…");
        let ram = Self::available_ram();
        println!("   → {} bytes", ram);

        Ok(cached)
    }
}
