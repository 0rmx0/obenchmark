use crate::engines::benchmark::{bench_meta, BenchCategory};
use crate::model::result::BenchScore;
use crate::util::config::verbose;

/// Per-benchmark normalized cap: a machine may outperform the reference
/// baseline, but not skew the average beyond this factor.
const PER_BENCH_MAX: u64 = 10_000;
/// Final aggregated score cap (5 digits, see README).
const AGGREGATED_MAX: u64 = 99_999;

#[derive(Clone, Copy, Debug, Default)]
pub struct AggregatedScores {
    pub global: u64,
    pub cpu: u64,
    pub mem: u64,
    pub disk: u64,
}

/// Normalize a raw score against its per-benchmark baseline (~1000 = reference).
pub fn normalize(name: &str, raw_score: u64) -> u64 {
    let baseline = bench_meta(name).baseline;
    if baseline == 0 {
        return 0;
    }

    let mut norm = ((raw_score as f64 / baseline as f64) * 1000.0) as u64;
    if norm > PER_BENCH_MAX {
        norm = PER_BENCH_MAX;
    }
    norm
}

pub fn compute_aggregated_scores(scores: &[BenchScore]) -> AggregatedScores {
    // u128 accumulators to avoid intermediate overflows.
    let mut total_weight_global: u128 = 0;
    let mut total_score_global: u128 = 0;
    let mut total_weight_cpu: u128 = 0;
    let mut total_score_cpu: u128 = 0;
    let mut total_weight_mem: u128 = 0;
    let mut total_score_mem: u128 = 0;
    let mut total_weight_disk: u128 = 0;
    let mut total_score_disk: u128 = 0;

    for s in scores {
        let normalized = normalize(&s.name, s.raw_score) as u128;
        let weight = s.weight as u128;
        if verbose() {
            eprintln!(
                "[score] {} -> normalized={} weight={}",
                s.name, normalized, s.weight
            );
        }

        total_score_global = total_score_global.saturating_add(normalized.saturating_mul(weight));
        total_weight_global = total_weight_global.saturating_add(weight);

        match bench_meta(&s.name).category {
            BenchCategory::Cpu => {
                total_score_cpu = total_score_cpu.saturating_add(normalized.saturating_mul(weight));
                total_weight_cpu = total_weight_cpu.saturating_add(weight);
            }
            BenchCategory::Mem => {
                total_score_mem = total_score_mem.saturating_add(normalized.saturating_mul(weight));
                total_weight_mem = total_weight_mem.saturating_add(weight);
            }
            BenchCategory::Disk => {
                total_score_disk =
                    total_score_disk.saturating_add(normalized.saturating_mul(weight));
                total_weight_disk = total_weight_disk.saturating_add(weight);
            }
            BenchCategory::Other => {}
        }
    }

    let compute_avg = |total_score: u128, total_weight: u128| -> u64 {
        if total_weight == 0 {
            return 0;
        }
        // Weighted average, rounded to nearest.
        let averaged = (total_score + total_weight / 2) / total_weight;
        if verbose() {
            eprintln!(
                "[score] category total_score={} total_weight={} averaged={}",
                total_score, total_weight, averaged
            );
        }
        let capped = averaged.min(AGGREGATED_MAX as u128);
        capped as u64
    };

    AggregatedScores {
        global: compute_avg(total_score_global, total_weight_global),
        cpu: compute_avg(total_score_cpu, total_weight_cpu),
        mem: compute_avg(total_score_mem, total_weight_mem),
        disk: compute_avg(total_score_disk, total_weight_disk),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bench(name: &str, raw_score: u64, weight: u64) -> BenchScore {
        BenchScore {
            name: name.to_string(),
            raw_score,
            weight,
        }
    }

    // Tests pour normalize()
    #[test]
    fn normalize_cpu_baseline_hits_reference() {
        assert_eq!(normalize("CPU Multi-Core", 80_000_000), 1000);
    }

    #[test]
    fn normalize_cpu_int_math_reference() {
        assert_eq!(normalize("CPU Int Math", 50_000_000), 1000);
    }

    #[test]
    fn normalize_cpu_float_math_reference() {
        assert_eq!(normalize("CPU Float Math", 10_000_000), 1000);
    }

    #[test]
    fn normalize_mem_cached_read_reference() {
        assert_eq!(normalize("Mem Cached Read", 50_000), 1000);
    }

    #[test]
    fn normalize_disk_seq_read_reference() {
        assert_eq!(normalize("Disk Seq Read", 500), 1000);
    }

    #[test]
    fn normalize_fallback_uses_family_baseline() {
        let value = normalize("Custom CPU Weird", 100_000_000);
        assert!(value >= 1000);
    }

    #[test]
    fn normalize_caps_at_max() {
        // Very high score must be capped at PER_BENCH_MAX (10_000).
        let value = normalize("CPU Multi-Core", 80_000_000 * 100);
        assert_eq!(value, 10_000);
    }

    #[test]
    fn normalize_handles_zero_raw_score() {
        assert_eq!(normalize("CPU Multi-Core", 0), 0);
    }

    // Tests pour compute_aggregated_scores()
    #[test]
    fn aggregated_scores_respect_weights_and_categories() {
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000, 2),
            bench("Mem Write", 20_000, 3),
        ];
        let aggregated = compute_aggregated_scores(&scores);
        assert_eq!(aggregated.global, 1000);
        assert_eq!(aggregated.cpu, 1000);
        assert_eq!(aggregated.mem, 1000);
        assert_eq!(aggregated.disk, 0);
    }

    #[test]
    fn aggregated_scores_empty_are_zero() {
        let aggregated = compute_aggregated_scores(&[]);
        assert_eq!(aggregated.global, 0);
        assert_eq!(aggregated.cpu, 0);
        assert_eq!(aggregated.mem, 0);
        assert_eq!(aggregated.disk, 0);
    }

    #[test]
    fn aggregated_scores_all_categories() {
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000, 2),
            bench("Mem Cached Read", 50_000, 2),
            bench("Disk Seq Read", 500, 2),
        ];
        let aggregated = compute_aggregated_scores(&scores);
        assert_eq!(aggregated.cpu, 1000);
        assert_eq!(aggregated.mem, 1000);
        assert_eq!(aggregated.disk, 1000);
        assert!(aggregated.global > 0);
    }

    #[test]
    fn aggregated_scores_weighted_average() {
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000, 3),
            bench("CPU Int Math", 50_000_000, 2),
        ];
        let aggregated = compute_aggregated_scores(&scores);
        // (1000*3 + 1000*2) / 5 = 1000
        assert_eq!(aggregated.cpu, 1000);
    }

    #[test]
    fn aggregated_scores_capped_at_max() {
        let scores = vec![bench("CPU Multi-Core", 80_000_000 * 1000, 2)];
        let aggregated = compute_aggregated_scores(&scores);
        assert!(aggregated.global <= 99_999);
        assert!(aggregated.cpu <= 99_999);
    }

    #[test]
    fn aggregated_scores_handles_mixed_categories() {
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000, 2),
            bench("CPU Int Math", 25_000_000, 2), // normalized 500
            bench("Mem Write", 20_000, 2),
            bench("Disk Seq Read", 500, 2),
        ];
        let aggregated = compute_aggregated_scores(&scores);
        // CPU: (1000*2 + 500*2) / 4 = 750
        assert_eq!(aggregated.cpu, 750);
        assert_eq!(aggregated.mem, 1000);
        assert_eq!(aggregated.disk, 1000);
    }
}
