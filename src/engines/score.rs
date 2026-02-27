use crate::model::result::BenchScore;

/// Baselines pour score normalisé
/// 1000 = machine de base
const CPU_BASELINE: u64 = 50_000_000;
const MEM_BASELINE: u64 = 5000;
const DISK_BASELINE: u64 = 1000;

pub fn normalize(name: &str, raw_score: u64) -> u64 {
    let baseline = match name {
        "CPU Multi-Core" => CPU_BASELINE,
        "Memory Bandwidth" => MEM_BASELINE,
        "Disk Sequential" => DISK_BASELINE,
        _ => 1000,
    };

    ((raw_score as f64 / baseline as f64) * 1000.0) as u64
}

pub fn compute_final_score(scores: &[BenchScore]) -> u64 {
    let mut total_weight = 0;
    let mut total_score = 0;

    for s in scores {
        let normalized = normalize(&s.name, s.raw_score);
        total_score += normalized * s.weight;
        total_weight += s.weight;
    }

    if total_weight == 0 { 0 } else { total_score / total_weight }
}