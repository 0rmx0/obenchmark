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

    // -----------------------------
    // 1. Math entier (ALU)
    // -----------------------------
    fn int_math() -> u64 {
        let iterations = 200_000_000u64;
        let mut v = 1u64;
        let start = Instant::now();

        for i in 1..iterations {
            v = black_box(v.wrapping_mul(i ^ (i << 1)).wrapping_add(3));
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 2. Nombres premiers
    // -----------------------------
    fn prime_test(limit: u64) -> u64 {
        fn is_prime(n: u64) -> bool {
            if n < 2 { return false; }
            let r = (n as f64).sqrt() as u64;
            for i in 2..=r {
                if n % i == 0 { return false; }
            }
            true
        }

        let start = Instant::now();
        let mut _count = 0u64;

        for n in 2..limit {
            if is_prime(n) { _count += 1; }
        }

        (limit as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 3. Compression simulée (hash avalanche)
    // -----------------------------
    fn compression_sim() -> u64 {
        let size = 50_000_000;
        let mut buf = vec![0u8; size];

        for i in 0..buf.len() {
            buf[i] = (i % 251) as u8;
        }

        let start = Instant::now();
        let mut hash = 0u64;

        for &b in &buf {
            hash = black_box(hash.rotate_left(5) ^ b as u64);
        }

        (size as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 4. Physiques — simulation Newton basique
    // -----------------------------
    fn physics_sim() -> u64 {
        let iterations = 20_000_000;
        let start = Instant::now();
        let mut x = 0.0f64;
        let mut v = 1.0f64;

        for _ in 0..iterations {
            v += (x * 0.00001).sin() * 0.0001;
            x += v;
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 5. UCT Single Thread (Monte Carlo)
    // -----------------------------
    fn uct_single_thread() -> u64 {
        let iterations = 5_000_000;
        let start = Instant::now();
        let mut value = 0f64;

        for i in 1..iterations {
            let uct = ((value / i as f64).abs() + (i as f64).ln().sqrt()) % 1.0;
            value += (uct * i as f64).sin();
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 6. Math virgule flottante (FPU)
    // -----------------------------
    fn float_math() -> u64 {
        let iterations = 100_000_000;
        let start = Instant::now();
        let mut x = 1.0f64;

        for _ in 0..iterations {
            x = (x.sin() * x.cos()).tan();
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 7. Instructions étendues (SSE)
    // -----------------------------
    #[cfg(target_arch = "x86_64")]
    fn simd_test() -> u64 {
        use std::arch::x86_64::*;
        let iterations = 50_000_000;
        let start = Instant::now();

        unsafe {
            let mut a = _mm_set1_ps(1.0);
            let b = _mm_set1_ps(1.0001);

            for _ in 0..iterations {
                a = _mm_mul_ps(a, b);
            }
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn simd_test() -> u64 {
        0
    }

    // -----------------------------
    // 8. Cryptage simulé
    // -----------------------------
    fn crypto_test() -> u64 {
        let iterations = 50_000_000;
        let start = Instant::now();
        let mut key = 0xA5A5A5A5A5A5A5A5u64;

        for i in 0..iterations {
            key = black_box(key.rotate_left(13) ^ (i * 0x9E3779B97F4A7C15));
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }

    // -----------------------------
    // 9. Tirage pseudo-aléatoire (Xorshift)
    // -----------------------------
    fn random_test() -> u64 {
        let iterations = 50_000_000;
        let start = Instant::now();
        let mut x = 123456789u64;

        for _ in 0..iterations {
            x ^= x << 7;
            x ^= x >> 9;
            x ^= x << 8;
        }

        (iterations as f64 / start.elapsed().as_secs_f64()) as u64
    }
}

impl Benchmark for CpuBenchmark {
    fn name(&self) -> &'static str {
        "CPU Advanced"
    }

    fn run(log_tx: &Sender<String>) -> u64 {
    log_tx.send("▶ Math entier…".into()).ok();
    let int_result = Self::int_math();

    log_tx.send("▶ Nombres premiers…".into()).ok();
    let prime_result = Self::prime_test(300_000);

    log_tx.send("▶ Compression simulée…".into()).ok();
    let comp = Self::compression_sim();

    log_tx.send("▶ Physiques…".into()).ok();
    let phys = Self::physics_sim();

    log_tx.send("▶ UCT single thread…".into()).ok();
    let uct = Self::uct_single_thread();

    log_tx.send("▶ Math flottants…".into()).ok();
    let float = Self::float_math();

    log_tx.send("▶ SIMD / SSE…".into()).ok();
    let simd = Self::simd_test();

    log_tx.send("▶ Cryptage…".into()).ok();
    let crypto = Self::crypto_test();

    log_tx.send("▶ Tirage…".into()).ok();
    let rnd = Self::random_test();

    // Score final
    (int_result + prime_result + comp + phys + uct + float + simd + crypto + rnd) / 9
}
}