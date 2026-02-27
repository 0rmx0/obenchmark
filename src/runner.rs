use std::sync::mpsc::{self, Sender};
use std::thread;
use anyhow::Result;
use crate::benchmarks::Benchmark;

pub struct BenchResult {
    pub scores: Vec<(String, u64)>,
    pub global_score: u64,
}

#[derive(Debug, Clone)]
pub enum ProgressUpdate {
    Global(f32),                 // 0.0 → 1.0
    Step(f32, String),           // progression du test en cours
    StepStart(String),           // nom du test
    StepEnd(String),             // fin du test
}

fn weighted_score(scores: &[(String, u64)]) -> u64 {
    let mut total = 0u64;
    let mut weight_sum = 0u64;

    for (name, score) in scores {
        let weight = match name.as_str() {
            "CPU Advanced" => 4,
            "Memory" => 2,
            "Disk" => 1,
            _ => 1,
        };
        total += *score * weight;
        weight_sum += weight;
    }

    if weight_sum == 0 { 0 } else { total / weight_sum }
}

pub fn run_async(
    benches: Vec<Box<dyn Benchmark + Send>>,
    log_tx: Sender<String>,
    progress_tx: Sender<ProgressUpdate>,
) -> mpsc::Receiver<Result<BenchResult>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let total = benches.len() as f32;
        let mut index = 0f32;
        let mut scores = Vec::new();

        log_tx.send("▶ Début du benchmark…".into()).ok();

        for bench in benches {
            let name = bench.name().to_string();

            // Step start
            progress_tx.send(ProgressUpdate::StepStart(name.clone())).ok();
            progress_tx.send(ProgressUpdate::Step(0.0, name.clone())).ok();
            log_tx.send(format!("— [{}] Démarrage…", name)).ok();

            // Exécution
            match bench.run() {
                Ok(score) => {
                    scores.push((name.clone(), score));
                    progress_tx.send(ProgressUpdate::Step(1.0, name.clone())).ok();
                    progress_tx.send(ProgressUpdate::StepEnd(name.clone())).ok();
                    log_tx.send(format!("✓ [{}] Terminé — Score {}", name, score)).ok();
                }
                Err(e) => {
                    log_tx.send(format!("✗ [{}] Erreur {:?}", name, e)).ok();
                    let _ = tx.send(Err(e));
                    return;
                }
            }

            // Progression globale
            index += 1.0;
            let global_pct = index / total;
            progress_tx.send(ProgressUpdate::Global(global_pct)).ok();
        }

        let global = weighted_score(&scores);

        log_tx.send(format!("🏁 Benchmarks terminés — Score global: {}", global))
            .ok();

        let _ = tx.send(Ok(BenchResult {
            scores,
            global_score: global,
        }));
    });

    rx
}