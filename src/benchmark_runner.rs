//! Orchestrateur du benchmark.
//! Exécute les tests sélectionnés et calcule le score global.

use crate::benchmarks::{cpu, memory, disk, gpu};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration transmise depuis l’UI.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TestConfig {
    pub cpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub gpu_backends: Vec<String>,
}

/// Résultat d’un benchmark.
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchResult {
    pub scores: Vec<(String, u64)>,
    pub global_score: u64,
}

/// Exécute les tests demandés et renvoie le résultat.
pub fn run_selected_tests(config: TestConfig) -> BenchResult {
    let mut scores: Vec<(String, u64)> = Vec::new();

    // --- CPU ---
    if config.cpu {
        let sc = cpu::cpu_test();
        scores.push(("CPU".into(), sc));
    }

    // --- Mémoire ---
    if config.memory {
        let sc = memory::memory_test();
        scores.push(("RAM".into(), sc));
    }

    // --- Disque ---
    if config.disk {
        let sc = disk::disk_test();
        scores.push(("DISK".into(), sc));
    }

    // --- GPU (un ou plusieurs backends) ---
    for backend in config.gpu_backends.iter() {
        let sc = gpu::run_gpu_backend(backend);
        scores.push((format!("GPU ({})", backend), sc));
    }

    // Calcul du score global (moyenne arithmétique)
    let global = if scores.is_empty() {
        0
    } else {
        let sum: u64 = scores.iter().map(|(_, s)| *s).sum();
        sum / scores.len() as u64
    };

    BenchResult {
        scores,
        global_score: global,
    }
}
