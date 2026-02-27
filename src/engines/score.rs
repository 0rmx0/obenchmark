use crate::model::result::BenchScore;

/// Baselines pour score normalisé
/// 1000 = machine de base
const CPU_BASELINE: u64 = 50_000_000;
const MEM_BASELINE: u64 = 5000;
const DISK_BASELINE: u64 = 1000;

pub fn normalize(name: &str, raw_score: u64) -> u64 {
    // Map many benchmark names to coarse categories so baselines stay meaningful
    let baseline = if name.contains("CPU") {
        CPU_BASELINE
    } else if name.to_lowercase().contains("mem") || name.to_lowercase().contains("memory") {
        MEM_BASELINE
    } else if name.to_lowercase().contains("disk") || name.to_lowercase().contains("iops") {
        DISK_BASELINE
    } else {
        1000
    };

    // Normalize to a 0-1000 scale relative to the baseline
    let mut norm = ((raw_score as f64 / baseline as f64) * 1000.0) as u64;
    // cap per-benchmark to avoid huge outliers that saturate final average
    const PER_BENCH_MAX: u64 = 10_000;
    if norm > PER_BENCH_MAX { norm = PER_BENCH_MAX; }
    norm
}

pub fn compute_final_score(scores: &[BenchScore]) -> u64 {
    // Use wider arithmetic (u128) for intermediate sums to avoid overflow
    let mut total_weight: u128 = 0;
    let mut total_score: u128 = 0;

    for s in scores {
        let normalized = normalize(&s.name, s.raw_score) as u128;
        let weight = s.weight as u128;
        // debug: print each normalized score and weight
        eprintln!("[score] {} -> normalized={} weight={}", s.name, normalized, s.weight);
        total_score = total_score.saturating_add(normalized.saturating_mul(weight));
        total_weight = total_weight.saturating_add(weight);
    }

    let score_u64 = if total_weight == 0 {
        0u64
    } else {
        // divide in u128 then clamp to u64
        let averaged = (total_score / total_weight) as u128;
        eprintln!("[score] total_score={} total_weight={} averaged={}", total_score, total_weight, averaged);
        let capped = if averaged > 99_999u128 { 99_999u128 } else { averaged };
        capped as u64
    };

    score_u64
}