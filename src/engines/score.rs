use crate::model::result::BenchScore;

/// Baselines pour score normalisé
/// ~1000 = machine de référence
const CPU_BASELINE: u64 = 50_000_000;
const MEM_BASELINE: u64 = 5000;
const DISK_BASELINE: u64 = 1000;

#[derive(Clone, Copy, Debug, Default)]
pub struct AggregatedScores {
    pub global: u64,
    pub cpu: u64,
    pub mem: u64,
    pub disk: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchClass {
    Cpu,
    Mem,
    Disk,
    Other,
}

/// Baseline plus fine par bench individuel, pour garder des scores
/// comparables malgré des unités et ordres de grandeur différents.
fn per_bench_baseline(name: &str) -> Option<u64> {
    match name {
        #[cfg(test)]
        "Test Zero Baseline" => Some(0),
        // CPU (ops/s ou dérivés)
        "CPU Multi-Core" => Some(80_000_000),
        "CPU Int Math" => Some(50_000_000),
        "CPU Float Math" => Some(10_000_000),
        "CPU Prime Calc" => Some(2_000_000),
        "CPU SSE Ext" => Some(50_000_000),
        // CPU en MB/s
        "CPU Compression" => Some(500),
        "CPU Encryption" => Some(500),
        // CPU divers
        "CPU Physics" => Some(100_000_000),
        "CPU Sorting" => Some(50_000_000),
        "CPU UCT Single" => Some(10_000_000),

        // Mémoire
        "Mem DB Ops" => Some(10_000_000),    // ops/s approx
        "Mem Cached Read" => Some(50_000),   // MB/s
        "Mem Uncached Read" => Some(20_000), // MB/s
        "Mem Write" => Some(20_000),         // MB/s
        "Mem Available" => Some(8 * 1024),   // 8 GiB en MB
        "Mem Latency" => Some(50_000_000),   // accès/s
        "Mem Threaded" => Some(50_000),      // MB/s agrégés

        // Disque
        "Disk Seq Read" => Some(500),  // MB/s
        "Disk Seq Write" => Some(400), // MB/s
        "Disk IOPS 32K QD20" => Some(50_000),
        "Disk IOPS 4K QD1" => Some(10_000),

        _ => None,
    }
}

pub fn normalize(name: &str, raw_score: u64) -> u64 {
    let lower = name.to_lowercase();

    // Baseline spécifique au bench si connue, sinon fallback par famille.
    let baseline = per_bench_baseline(name).unwrap_or_else(|| {
        if lower.contains("cpu") {
            CPU_BASELINE
        } else if lower.contains("mem") || lower.contains("memory") {
            MEM_BASELINE
        } else if lower.contains("disk") || lower.contains("iops") {
            DISK_BASELINE
        } else {
            1000
        }
    });

    if baseline == 0 {
        return 0;
    }

    // Normaliser autour de 1000 par rapport à la baseline
    let mut norm = ((raw_score as f64 / baseline as f64) * 1000.0) as u64;
    // Autoriser un écart plus important entre machines avant saturation
    const PER_BENCH_MAX: u64 = 100_000;
    if norm > PER_BENCH_MAX {
        norm = PER_BENCH_MAX;
    }
    norm
}

fn classify(name: &str) -> BenchClass {
    match name {
        // CPU
        "CPU Multi-Core" | "CPU Int Math" | "CPU Float Math" | "CPU Prime Calc" | "CPU SSE Ext"
        | "CPU Compression" | "CPU Encryption" | "CPU Physics" | "CPU Sorting"
        | "CPU UCT Single" => BenchClass::Cpu,

        // Mémoire
        "Mem DB Ops" | "Mem Cached Read" | "Mem Uncached Read" | "Mem Write" | "Mem Available"
        | "Mem Latency" | "Mem Threaded" => BenchClass::Mem,

        // Disque
        "Disk Seq Read" | "Disk Seq Write" | "Disk IOPS 32K QD20" | "Disk IOPS 4K QD1" => {
            BenchClass::Disk
        }

        // Fallback pour d'éventuels nouveaux noms
        _ => {
            let lower = name.to_lowercase();
            if lower.contains("cpu") {
                BenchClass::Cpu
            } else if lower.contains("mem") || lower.contains("memory") {
                BenchClass::Mem
            } else if lower.contains("disk") || lower.contains("iops") {
                BenchClass::Disk
            } else {
                BenchClass::Other
            }
        }
    }
}

pub fn compute_aggregated_scores(scores: &[BenchScore]) -> AggregatedScores {
    // Utiliser u128 pour éviter les débordements intermédiaires
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
        #[cfg(debug_assertions)]
        eprintln!(
            "[score] {} -> normalized={} weight={}",
            s.name, normalized, s.weight
        );

        // global
        total_score_global = total_score_global.saturating_add(normalized.saturating_mul(weight));
        total_weight_global = total_weight_global.saturating_add(weight);

        // par catégorie
        match classify(&s.name) {
            BenchClass::Cpu => {
                total_score_cpu = total_score_cpu.saturating_add(normalized.saturating_mul(weight));
                total_weight_cpu = total_weight_cpu.saturating_add(weight);
            }
            BenchClass::Mem => {
                total_score_mem = total_score_mem.saturating_add(normalized.saturating_mul(weight));
                total_weight_mem = total_weight_mem.saturating_add(weight);
            }
            BenchClass::Disk => {
                total_score_disk =
                    total_score_disk.saturating_add(normalized.saturating_mul(weight));
                total_weight_disk = total_weight_disk.saturating_add(weight);
            }
            BenchClass::Other => {}
        }
    }

    let compute_avg = |total_score: u128, total_weight: u128| -> u64 {
        if total_weight == 0 {
            0
        } else {
            // Moyenne pondérée avec arrondi au plus proche.
            let averaged = (total_score + total_weight / 2) / total_weight;
            #[cfg(debug_assertions)]
            eprintln!(
                "[score] category total_score={} total_weight={} averaged={}",
                total_score, total_weight, averaged
            );
            let capped = if averaged > 999_999u128 {
                999_999u128
            } else {
                averaged
            };
            capped as u64
        }
    };

    AggregatedScores {
        global: compute_avg(total_score_global, total_weight_global),
        cpu: compute_avg(total_score_cpu, total_weight_cpu),
        mem: compute_avg(total_score_mem, total_weight_mem),
        disk: compute_avg(total_score_disk, total_weight_disk),
    }
}

pub fn compute_final_score(scores: &[BenchScore]) -> u64 {
    compute_aggregated_scores(scores).global
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

    #[test]
    fn normalize_cpu_baseline_hits_reference() {
        assert_eq!(normalize("CPU Multi-Core", 80_000_000), 1000);
    }

    #[test]
    fn normalize_fallback_uses_family_baseline() {
        let value = normalize("Custom CPU Weird", 100_000_000);
        assert!(value >= 1000);
    }

    #[test]
    fn normalize_handles_zero_baseline() {
        assert_eq!(normalize("Test Zero Baseline", 123), 0);
    }

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
}
