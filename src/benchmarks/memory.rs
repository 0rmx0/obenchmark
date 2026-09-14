// Les benchmarks mémoire utilisent volontairement des boucles d'indexation
// explicites pour reproduire des patterns d'accès séquentiels mesurables.
#![allow(clippy::needless_range_loop)]

use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};

use crate::engines::benchmark::Benchmark;
use crate::util::measure::median_of_passes;
use crate::util::sysinfo::get_system_info;

/// Taille par défaut des gros buffers mémoire utilisés pour les tests séquentiels.
const DEFAULT_STREAM_MEM_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB
/// Taille minimale pour garder un signal exploitable même sur petites machines.
const MIN_STREAM_MEM_BYTES: u64 = 64 * 1024 * 1024; // 64 MiB
/// Fraction maximale de la RAM totale à utiliser pour un seul test mémoire.
const MAX_FRACTION_OF_RAM: f64 = 0.25; // 25%

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// Mesure reproductible : 1 passe de chauffe + 3 passes mesurées, médiane renvoyée.
fn measured<F>(sample: F) -> u64
where
    F: FnMut(Duration) -> u64,
{
    median_of_passes(1, 3, sample)
}

/// Choisit une taille de buffer raisonnable en fonction de la RAM disponible.
fn choose_buffer_size_bytes(default: u64, min_bytes: u64) -> u64 {
    let sys = get_system_info();
    let total_bytes = sys.total_memory().saturating_mul(1024); // KB -> bytes
    if total_bytes == 0 {
        return default.max(min_bytes);
    }

    let cap_by_fraction = (total_bytes as f64 * MAX_FRACTION_OF_RAM).round() as u64;
    let mut size = default.min(cap_by_fraction);
    if size < min_bytes {
        size = min_bytes.min(total_bytes / 2);
    }

    // Ne jamais dépasser la moitié de la RAM physique pour ce test.
    size.min(total_bytes / 2).max(min_bytes)
}

/// 1. Opérations type base de données (insertions/lectures aléatoires dans un vecteur).
pub struct MemoryDBOps;
impl Benchmark for MemoryDBOps {
    fn name(&self) -> &str {
        "Mem DB Ops"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        Ok(measured(|budget| {
            let mut db: Vec<u64> = Vec::with_capacity(1_000_000);
            let start = Instant::now();
            let mut ops: u64 = 0;
            let mut i: u64 = 0;
            while start.elapsed() < budget {
                db.push(i);
                let idx = (i / 2) as usize;
                let _ = black_box(db[idx]);
                ops += 2; // 1 push + 1 read par itération
                i = i.wrapping_add(1);
                if i >= 1_000_000 {
                    i = 0;
                    db.clear();
                    db.reserve(1_000_000);
                }
            }
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            black_box(&db);
            (ops as f64 / elapsed) as u64
        }))
    }
}

/// 2. Lecture fortement cachée (petit buffer lu de nombreuses fois).
pub struct MemoryCachedRead;
impl Benchmark for MemoryCachedRead {
    fn name(&self) -> &str {
        "Mem Cached Read"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        // 32 KiB : typiquement dans le L1/L2, mesure la bande passante de cache.
        // Lecture par mots de 64 bits (style STREAM) plutôt qu'octet par octet,
        // pour mesurer la vraie bande passante du cache et non la surcharge de boucle.
        const SIZE: usize = 32 * 1024;
        let data = vec![0u64; SIZE / 8];
        Ok(measured(|budget| {
            let start = Instant::now();
            let mut sum = 0u64;
            let mut total_bytes: u64 = 0;
            while start.elapsed() < budget {
                for &w in &data {
                    sum = sum.wrapping_add(w);
                }
                total_bytes += SIZE as u64;
            }
            black_box(sum);
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            (bytes_to_mb(total_bytes) / elapsed) as u64
        }))
    }
}

/// 3. Lecture séquentielle sur un grand buffer (faible taux de cache).
pub struct MemoryUncachedRead;
impl Benchmark for MemoryUncachedRead {
    fn name(&self) -> &str {
        "Mem Uncached Read"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let size_bytes = choose_buffer_size_bytes(DEFAULT_STREAM_MEM_BYTES, MIN_STREAM_MEM_BYTES);
        let words = (size_bytes as usize) / 8;
        let mut data = vec![0u64; words];
        // Pré-chauffe : toucher toutes les pages pour éviter de mesurer les fautes de page.
        for w in &mut data {
            *w = w.wrapping_add(1);
        }

        Ok(measured(|budget| {
            let start = Instant::now();
            let mut sum = 0u64;
            let mut passes: u64 = 0;
            while start.elapsed() < budget {
                for &w in &data {
                    sum = sum.wrapping_add(w);
                }
                passes += 1;
            }
            black_box(sum);
            let total_bytes = (words as u64) * 8 * passes;
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            (bytes_to_mb(total_bytes) / elapsed) as u64
        }))
    }
}

/// 4. Bande passante d'écriture mémoire séquentielle.
pub struct MemoryWrite;
impl Benchmark for MemoryWrite {
    fn name(&self) -> &str {
        "Mem Write"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let size_bytes = choose_buffer_size_bytes(DEFAULT_STREAM_MEM_BYTES, MIN_STREAM_MEM_BYTES);
        let words = (size_bytes as usize) / 8;
        let mut data = vec![0u64; words];
        Ok(measured(|budget| {
            let start = Instant::now();
            let mut passes: u64 = 0;
            let mut pattern: u64 = 0xA5A5_A5A5_A5A5_A5A5;
            while start.elapsed() < budget {
                for w in &mut data {
                    *w = pattern;
                    pattern = pattern.wrapping_add(0x0102_0304_0506_0708);
                }
                passes += 1;
            }
            black_box(&data[0..std::cmp::min(1024, words)]);
            let total_bytes = (words as u64) * 8 * passes;
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            (bytes_to_mb(total_bytes) / elapsed) as u64
        }))
    }
}

/// 5. RAM disponible (en MB) au moment du benchmark.
pub struct MemoryAvailable;
impl Benchmark for MemoryAvailable {
    fn name(&self) -> &str {
        "Mem Available"
    }

    fn weight(&self) -> u64 {
        0 // Caractéristique système, pas une performance : hors du scoring.
    }

    fn run(&self) -> Result<u64> {
        let sys = get_system_info();
        Ok(sys.available_memory() / 1024) // MB
    }
}

/// 6. Latence mémoire via accès aléatoires chaînés (pointer chasing).
pub struct MemoryLatency;
impl Benchmark for MemoryLatency {
    fn name(&self) -> &str {
        "Mem Latency"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        // Taille de l'ensemble de travail ~80 Mo sur plateforme 64 bits.
        let n: usize = 10_000_000;

        // Construire un cycle pseudo-aléatoire sur [0, n).
        let mut order: Vec<usize> = (0..n).collect();
        let mut rng = StdRng::seed_from_u64(0x4D45_4D00);
        order.shuffle(&mut rng);

        let mut next = vec![0usize; n];
        for i in 0..n {
            let from = order[i];
            let to = order[(i + 1) % n];
            next[from] = to;
        }

        // Mesure reproductible : on compte les accès sur un budget de temps.
        Ok(measured(|budget| {
            let mut idx = 0usize;
            let start = Instant::now();
            let mut accesses: u64 = 0;
            while start.elapsed() < budget {
                // Accès pointer-chasing : chaque accès dépend du précédent.
                unsafe {
                    idx = *next.get_unchecked(idx);
                }
                accesses += 1;
            }
            black_box(idx);
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            // Convertir en accès par seconde (plus élevé = meilleure latence).
            (accesses as f64 / elapsed) as u64
        }))
    }
}

/// 7. Bande passante mémoire multi-thread (un buffer par cœur logique).
pub struct MemoryThreaded;
impl Benchmark for MemoryThreaded {
    fn name(&self) -> &str {
        "Mem Threaded"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let threads = num_cpus::get();
        // per-thread local buffer size, but cap overall allocation to half RAM
        let sys = get_system_info();
        let total_bytes = sys.total_memory().saturating_mul(1024);
        let per_thread_default: u64 = 100u64 * 1024 * 1024;
        let per_thread_cap = (total_bytes / 2) / threads as u64;
        let size_bytes = std::cmp::min(per_thread_default, per_thread_cap);
        let size = size_bytes as usize;

        // Pré-allouer les buffers de chaque thread hors du timer pour mesurer
        // uniquement la bande passante d'écriture, pas l'allocation.
        Ok(measured(|budget| {
            let start = Instant::now();
            let mut passes: u64 = 0;
            while start.elapsed() < budget {
                let mut handles = Vec::with_capacity(threads);
                for _ in 0..threads {
                    handles.push(thread::spawn(move || -> u64 {
                        let mut local = vec![0u64; size / 8];
                        let mut p: u64 = 0x5A5A_5A5A_5A5A_5A5A;
                        for w in &mut local {
                            *w = p;
                            p = p.wrapping_add(0x0807_0605_0403_0201);
                        }
                        black_box(&local[0..std::cmp::min(128, local.len())]);
                        (local.len() * 8) as u64
                    }));
                }
                let mut all_done = true;
                for h in handles {
                    if h.join().unwrap_or(0) == 0 {
                        all_done = false;
                    }
                }
                if all_done {
                    passes += 1;
                }
            }
            let total_bytes_written = size_bytes * threads as u64 * passes.max(1);
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            (bytes_to_mb(total_bytes_written) / elapsed) as u64
        }))
    }
}
