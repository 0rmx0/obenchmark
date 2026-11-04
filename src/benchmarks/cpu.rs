use rayon::prelude::*;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use anyhow::Result;
use flate2::{write::ZlibEncoder, Compression};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

use crate::engines::benchmark::Benchmark;
use crate::util::config::{bench_duration, tick_progress};

const MIN_ELAPSED_SEC: f64 = 1e-6;

// multi-core generic loop
pub struct CpuMultiCore;
impl Benchmark for CpuMultiCore {
    fn name(&self) -> &str {
        "CPU Multi-Core"
    }

    fn run(&self) -> Result<u64> {
        let duration = bench_duration();
        let start = Instant::now();
        let mut iterations: u64 = 0;

        while tick_progress(&start, duration) {
            (0u64..1_000_000).into_par_iter().for_each(|i| {
                // Relatively costly integer mix, hard to optimize away.
                let v = i
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1)
                    .rotate_left(17)
                    ^ 0x9E37_79B9_7F4A_7C15;
                black_box(v);
            });
            iterations += 1_000_000;
        }
        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        Ok((iterations as f64 / elapsed) as u64)
    }
}

// 1. integer math
pub struct CpuIntMath;
impl Benchmark for CpuIntMath {
    fn name(&self) -> &str {
        "CPU Int Math"
    }

    fn run(&self) -> Result<u64> {
        let duration = bench_duration();
        let start = Instant::now();
        let mut x: u64 = 1;
        let mut count: u64 = 0;
        let mut acc: u64 = 0;

        while tick_progress(&start, duration) {
            x = x.wrapping_mul(123456789).wrapping_add(987654321);
            x = x.wrapping_sub(54321);
            x ^= x.rotate_left(13);
            acc = acc.wrapping_add(x);
            count += 1;
        }
        black_box(acc);

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        Ok((count as f64 / elapsed) as u64)
    }
}

// 2. floating point math
pub struct CpuFloatMath;
impl Benchmark for CpuFloatMath {
    fn name(&self) -> &str {
        "CPU Float Math"
    }

    fn run(&self) -> Result<u64> {
        let duration = bench_duration();
        let start = Instant::now();
        let mut x: f64 = 1.0;
        let mut count: u64 = 0;
        let mut acc: f64 = 0.0;

        while tick_progress(&start, duration) {
            x = x.mul_add(1.000_000_1, 0.000_000_1);
            x = (x.sin() + x.cos()).tan();
            // Keep the value finite: tan() diverges near its poles.
            if !x.is_finite() {
                x = 1.0;
            }
            acc += x;
            count += 1;
        }
        black_box(acc);

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        Ok((count as f64 / elapsed) as u64)
    }
}

// 3. prime counting
pub struct CpuPrimeCalc;
impl Benchmark for CpuPrimeCalc {
    fn name(&self) -> &str {
        "CPU Prime Calc"
    }

    fn run(&self) -> Result<u64> {
        let duration = bench_duration();
        let start = Instant::now();
        let mut count = 0u64;
        let mut n = 2u64;
        while tick_progress(&start, duration) {
            let mut is_prime = true;
            for i in 2..((n as f64).sqrt() as u64 + 1) {
                if n.is_multiple_of(i) {
                    is_prime = false;
                    break;
                }
            }
            if is_prime {
                count += 1;
            }
            n += 1;
        }
        black_box(count);
        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        // Return primes per second, not a raw 5s count.
        Ok((count as f64 / elapsed) as u64)
    }
}

// 4. vector ops (simulated SIMD)
pub struct CpuSSE;
impl Benchmark for CpuSSE {
    fn name(&self) -> &str {
        "CPU SSE Ext"
    }

    fn run(&self) -> Result<u64> {
        let mut a = vec![1f32; 1_000_000];
        let b = vec![2f32; 1_000_000];
        let duration = bench_duration();
        let start = Instant::now();
        let mut count: u64 = 0;
        while tick_progress(&start, duration) {
            for i in 0..a.len() {
                // Damped update: stays finite over thousands of passes
                // (a plain += would overflow to inf/NaN).
                a[i] = a[i] * 0.5 + b[i];
            }
            count += 1;
        }
        let mut checksum: f32 = 0.0;
        for v in &a {
            checksum += *v;
        }
        black_box(checksum);

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        Ok(((a.len() as u64 * count) as f64 / elapsed) as u64)
    }
}

// 5. compression
pub struct CpuCompression;
impl Benchmark for CpuCompression {
    fn name(&self) -> &str {
        "CPU Compression"
    }

    fn run(&self) -> Result<u64> {
        // Pseudo-random (but reproducible) input: compressing 10 MiB of
        // zeros only measures the trivial case.
        let mut rng = StdRng::seed_from_u64(42);
        let data: Vec<u8> = (0..10_000_000).map(|_| rng.gen::<u8>()).collect();
        let duration = bench_duration();
        let start = Instant::now();
        let mut total_bytes: u64 = 0;

        while tick_progress(&start, duration) {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&data)?;
            let compressed = encoder.finish()?;
            black_box(&compressed);
            total_bytes += data.len() as u64;
        }

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed;
        Ok(mb_per_sec as u64)
    }
}

// 6. hashing (SHA-256 loop)
pub struct CpuEncryption;
impl Benchmark for CpuEncryption {
    fn name(&self) -> &str {
        "CPU Encryption"
    }

    fn run(&self) -> Result<u64> {
        let data = vec![0u8; 1024 * 1024];
        let mut hasher = Sha256::new();
        let duration = bench_duration();
        let start = Instant::now();
        let mut total_bytes: u64 = 0;

        while tick_progress(&start, duration) {
            hasher.update(&data);
            let digest = hasher.finalize_reset();
            black_box(digest);
            total_bytes += data.len() as u64;
        }

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed;
        Ok(mb_per_sec as u64)
    }
}

// 7. simple physics simulation
pub struct CpuPhysics;
impl Benchmark for CpuPhysics {
    fn name(&self) -> &str {
        "CPU Physics"
    }

    fn run(&self) -> Result<u64> {
        let mut pos = vec![0f64; 100000];
        let vel = vec![1f64; 100000];
        let duration = bench_duration();
        let start = Instant::now();
        let mut count: u64 = 0;
        while tick_progress(&start, duration) {
            for i in 0..pos.len() {
                pos[i] = (pos[i] + vel[i]) % 1.0e9; // bounded: no overflow
            }
            count += 1;
        }
        let mut energy: f64 = 0.0;
        for p in &pos {
            energy += *p * *p;
        }
        black_box(energy);

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        Ok(((pos.len() as u64 * count) as f64 / elapsed) as u64)
    }
}

// 8. sorting
pub struct CpuSorting;
impl Benchmark for CpuSorting {
    fn name(&self) -> &str {
        "CPU Sorting"
    }

    fn run(&self) -> Result<u64> {
        let mut rng = StdRng::seed_from_u64(123);
        let duration = bench_duration();
        let start = Instant::now();
        let mut total_items: u64 = 0;

        while tick_progress(&start, duration) {
            let mut v: Vec<u64> = (0..1_000_000).map(|_| rng.gen()).collect();
            v.sort();
            black_box(&v[0..std::cmp::min(16, v.len())]);
            total_items += v.len() as u64;
        }

        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        let items_per_sec = (total_items as f64 / elapsed) as u64;
        Ok(items_per_sec)
    }
}

// 9. single-thread UCT-style loop
pub struct CpuUCT;
impl Benchmark for CpuUCT {
    fn name(&self) -> &str {
        "CPU UCT Single"
    }

    fn run(&self) -> Result<u64> {
        let duration = bench_duration();
        let start = Instant::now();
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        let mut count: u64 = 0;
        let mut value_acc: f64 = 0.0;

        // Simplified Monte-Carlo / UCT simulation.
        while tick_progress(&start, duration) {
            let mut visits = 1.0;
            let mut total_value = 0.0;

            for _ in 0..10_000 {
                let reward: f64 = rng.gen();
                total_value += reward;
                visits += 1.0;
                let uct = total_value / visits + (2.0 * (visits.ln() + 1.0)).sqrt();
                value_acc += uct;
                count += 1;
            }
        }

        black_box(value_acc);
        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        // Operations per second, not a raw count.
        Ok((count as f64 / elapsed) as u64)
    }
}
