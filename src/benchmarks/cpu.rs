use rayon::prelude::*;
use std::hint::black_box;
use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
use flate2::{write::ZlibEncoder, Compression};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

use crate::engines::benchmark::Benchmark;
use crate::util::measure::median_of_passes;

const MIN_ELAPSED_SEC: f64 = 1e-9;

/// Mesure reproductible : 1 passe de chauffe + 3 passes mesurées, médiane renvoyée.
/// `sample` reçoit un budget de ~300 ms et doit renvoyer un débit par seconde.
fn measured<F>(sample: F) -> u64
where
    F: FnMut(Duration) -> u64,
{
    median_of_passes(1, 3, sample)
}

/// Débit (iterations / s) d'une boucle tournant pendant `budget`.
fn ops_per_sec<F>(budget: Duration, mut body: F) -> u64
where
    F: FnMut(),
{
    let start = Instant::now();
    let mut count: u64 = 0;
    while start.elapsed() < budget {
        body();
        count += 1;
    }
    let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
    (count as f64 / elapsed) as u64
}

// multi-core generic loop
pub struct CpuMultiCore;
impl Benchmark for CpuMultiCore {
    fn name(&self) -> &str {
        "CPU Multi-Core"
    }

    fn weight(&self) -> u64 {
        3
    }

    fn run(&self) -> Result<u64> {
        Ok(measured(|budget| {
            let mut iterations: u64 = 0;
            let start = Instant::now();
            while start.elapsed() < budget {
                (0u64..1_000_000).into_par_iter().for_each(|i| {
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
            (iterations as f64 / elapsed) as u64
        }))
    }
}

// 1. math entier
pub struct CpuIntMath;
impl Benchmark for CpuIntMath {
    fn name(&self) -> &str {
        "CPU Int Math"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        Ok(measured(|budget| {
            ops_per_sec(budget, || {
                let mut x: u64 = black_box(1);
                x = x.wrapping_mul(123456789).wrapping_add(987654321);
                x = x.wrapping_sub(54321);
                x ^= x.rotate_left(13);
                black_box(x);
            })
        }))
    }
}

// 2. math flottante (vectorielle via accumulé ; mesure le FPU)
pub struct CpuFloatMath;
impl Benchmark for CpuFloatMath {
    fn name(&self) -> &str {
        "CPU Float Math"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        Ok(measured(|budget| {
            // Accumulation vectorisable de produits : mesure le FPU vectoriel
            // plutôt que les fonctions transcendantes (sin/cos) non représentatives.
            let mut acc: f64 = 1.0;
            ops_per_sec(budget, || {
                acc = acc.mul_add(1.000_000_1, 0.000_000_1);
                acc = (acc + 1.5f64).sqrt().max(1e-300);
                black_box(acc);
            })
        }))
    }
}

// 3. calcul des nombres premiers (single-thread, intensif)
pub struct CpuPrimeCalc;
impl Benchmark for CpuPrimeCalc {
    fn name(&self) -> &str {
        "CPU Prime Calc"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        Ok(measured(|budget| {
            let mut count = 0u64;
            let start = Instant::now();
            let mut n = 2u64;
            while start.elapsed() < budget {
                let mut is_prime = true;
                let limit = (n as f64).sqrt() as u64 + 1;
                for i in 2..limit {
                    if n.is_multiple_of(i) {
                        is_prime = false;
                        break;
                    }
                }
                if is_prime {
                    count += 1;
                }
                n = n.wrapping_add(1);
                black_box(is_prime);
            }
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            (count as f64 / elapsed) as u64
        }))
    }
}

// 4. instructions étendues (vector add, auto-vectorisé par LLVM)
pub struct CpuSSE;
impl Benchmark for CpuSSE {
    fn name(&self) -> &str {
        "CPU SSE Ext"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let mut a = vec![1f32; 1_000_000];
        let b = vec![2f32; 1_000_000];
        Ok(measured(|budget| {
            let start = Instant::now();
            let mut count: u64 = 0;
            while start.elapsed() < budget {
                for i in 0..a.len() {
                    a[i] += b[i];
                }
                count += 1;
            }
            let mut checksum: f32 = 0.0;
            for v in &a {
                checksum += *v;
            }
            black_box(checksum);
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            ((a.len() as u64 * count) as f64 / elapsed) as u64
        }))
    }
}

// 5. compression
pub struct CpuCompression;
impl Benchmark for CpuCompression {
    fn name(&self) -> &str {
        "CPU Compression"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let data = vec![0u8; 10_000_000];
        Ok(measured(|budget| {
            let start = Instant::now();
            let mut total_bytes: u64 = 0;
            while start.elapsed() < budget {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&data).ok();
                let compressed = encoder.finish().unwrap_or_default();
                black_box(&compressed);
                total_bytes += data.len() as u64;
            }
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed;
            mb_per_sec as u64
        }))
    }
}

// 6. cryptage (SHA256 loop)
pub struct CpuEncryption;
impl Benchmark for CpuEncryption {
    fn name(&self) -> &str {
        "CPU Encryption"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let data = vec![0u8; 1024 * 1024];
        Ok(measured(|budget| {
            let mut hasher = Sha256::new();
            let start = Instant::now();
            let mut total_bytes: u64 = 0;
            while start.elapsed() < budget {
                hasher.update(&data);
                let digest = hasher.finalize_reset();
                black_box(digest);
                total_bytes += data.len() as u64;
            }
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed;
            mb_per_sec as u64
        }))
    }
}

// 7. simulation physique simple
pub struct CpuPhysics;
impl Benchmark for CpuPhysics {
    fn name(&self) -> &str {
        "CPU Physics"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let mut pos = vec![0f64; 100000];
        let vel = vec![1f64; 100000];
        Ok(measured(|budget| {
            let start = Instant::now();
            let mut count: u64 = 0;
            while start.elapsed() < budget {
                for i in 0..pos.len() {
                    pos[i] += vel[i];
                }
                count += 1;
            }
            let mut energy: f64 = 0.0;
            for p in &pos {
                energy += *p * *p;
            }
            black_box(energy);
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            ((pos.len() as u64 * count) as f64 / elapsed) as u64
        }))
    }
}

// 8. tri
pub struct CpuSorting;
impl Benchmark for CpuSorting {
    fn name(&self) -> &str {
        "CPU Sorting"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let mut rng = StdRng::seed_from_u64(123);
        let mut total_items: u64 = 0;
        let start = Instant::now();
        // Tri sur 3 passes de ~300 ms pour rester reproductible.
        // total_items accumule déjà les 3 passes : on divise donc directement
        // par le temps écoulé (sans multiplier encore par 3).
        for _ in 0..3 {
            let mut v: Vec<u64> = (0..1_000_000).map(|_| rng.gen()).collect();
            v.sort();
            black_box(&v[0..std::cmp::min(16, v.len())]);
            total_items += v.len() as u64;
        }
        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        Ok((total_items as f64 / elapsed) as u64)
    }
}

// 9. uct single thread (dummy loop)
pub struct CpuUCT;
impl Benchmark for CpuUCT {
    fn name(&self) -> &str {
        "CPU UCT Single"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        Ok(measured(|budget| {
            let mut rng = StdRng::seed_from_u64(0xC0FFEE);
            let mut count: u64 = 0;
            let mut value_acc: f64 = 0.0;
            let start = Instant::now();
            while start.elapsed() < budget {
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
            (count as f64 / elapsed) as u64
        }))
    }
}
