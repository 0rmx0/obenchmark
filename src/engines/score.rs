use crate::model::result::BenchScore;

/// Baselines pour score normalisé
/// ~1000 = machine de référence
const CPU_BASELINE: u64 = 50_000_000;
const MEM_BASELINE: u64 = 5000;
const DISK_BASELINE: u64 = 1000;
const GFX_BASELINE: u64 = 200;

#[derive(Clone, Copy, Debug, Default)]
pub struct AggregatedScores {
    pub global: u64,
    pub cpu: u64,
    pub mem: u64,
    pub disk: u64,
    pub gfx: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchClass {
    Cpu,
    Mem,
    Disk,
    Gfx,
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

        // Graphisme (2D: mégapixels/s, 3D: triangles/s)
        "GFX 2D Raster" => Some(300),
        "GFX 3D Raster" => Some(3_000_000),

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
        } else if lower.contains("gfx") || lower.contains("graphic") {
            GFX_BASELINE
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

        // Graphisme (2D/3D)
        "GFX 2D Raster" | "GFX 3D Raster" => BenchClass::Gfx,

        // Fallback pour d'éventuels nouveaux noms
        _ => {
            let lower = name.to_lowercase();
            if lower.contains("cpu") {
                BenchClass::Cpu
            } else if lower.contains("mem") || lower.contains("memory") {
                BenchClass::Mem
            } else if lower.contains("disk") || lower.contains("iops") {
                BenchClass::Disk
            } else if lower.contains("gfx") || lower.contains("graphic") {
                BenchClass::Gfx
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

    let mut total_weight_gfx: u128 = 0;
    let mut total_score_gfx: u128 = 0;

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
            BenchClass::Gfx => {
                total_score_gfx = total_score_gfx.saturating_add(normalized.saturating_mul(weight));
                total_weight_gfx = total_weight_gfx.saturating_add(weight);
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
        gfx: compute_avg(total_score_gfx, total_weight_gfx),
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
    fn normalize_handles_zero_baseline() {
        assert_eq!(normalize("Test Zero Baseline", 123), 0);
    }

    #[test]
    fn normalize_caps_at_max() {
        // Score très élevé devrait être plafonné à 100_000
        let value = normalize("CPU Multi-Core", 80_000_000 * 100);
        assert_eq!(value, 100_000);
    }

    #[test]
    fn normalize_handles_zero_raw_score() {
        assert_eq!(normalize("CPU Multi-Core", 0), 0);
    }

    // Tests pour classify()
    #[test]
    fn classify_cpu_benchmarks() {
        assert_eq!(classify("CPU Multi-Core"), BenchClass::Cpu);
        assert_eq!(classify("CPU Int Math"), BenchClass::Cpu);
        assert_eq!(classify("CPU Float Math"), BenchClass::Cpu);
        assert_eq!(classify("CPU Prime Calc"), BenchClass::Cpu);
        assert_eq!(classify("CPU SSE Ext"), BenchClass::Cpu);
        assert_eq!(classify("CPU Compression"), BenchClass::Cpu);
        assert_eq!(classify("CPU Encryption"), BenchClass::Cpu);
        assert_eq!(classify("CPU Physics"), BenchClass::Cpu);
        assert_eq!(classify("CPU Sorting"), BenchClass::Cpu);
        assert_eq!(classify("CPU UCT Single"), BenchClass::Cpu);
    }

    #[test]
    fn classify_memory_benchmarks() {
        assert_eq!(classify("Mem DB Ops"), BenchClass::Mem);
        assert_eq!(classify("Mem Cached Read"), BenchClass::Mem);
        assert_eq!(classify("Mem Uncached Read"), BenchClass::Mem);
        assert_eq!(classify("Mem Write"), BenchClass::Mem);
        assert_eq!(classify("Mem Available"), BenchClass::Mem);
        assert_eq!(classify("Mem Latency"), BenchClass::Mem);
        assert_eq!(classify("Mem Threaded"), BenchClass::Mem);
    }

    #[test]
    fn classify_disk_benchmarks() {
        assert_eq!(classify("Disk Seq Read"), BenchClass::Disk);
        assert_eq!(classify("Disk Seq Write"), BenchClass::Disk);
        assert_eq!(classify("Disk IOPS 32K QD20"), BenchClass::Disk);
        assert_eq!(classify("Disk IOPS 4K QD1"), BenchClass::Disk);
    }

    #[test]
    fn classify_gfx_benchmarks() {
        assert_eq!(classify("GFX 2D Raster"), BenchClass::Gfx);
        assert_eq!(classify("GFX 3D Raster"), BenchClass::Gfx);
    }

    #[test]
    fn normalize_gfx_baseline_reference() {
        assert_eq!(normalize("GFX 2D Raster", 300), 1000);
        assert_eq!(normalize("GFX 3D Raster", 3_000_000), 1000);
    }

    #[test]
    fn aggregated_scores_include_gfx_category() {
        let scores = vec![
            bench("GFX 2D Raster", 300, 3),
            bench("GFX 3D Raster", 3_000_000, 3),
        ];
        let aggregated = compute_aggregated_scores(&scores);
        assert_eq!(aggregated.gfx, 1000);
        assert_eq!(aggregated.cpu, 0);
    }

    #[test]
    fn classify_fallback_to_other() {
        assert_eq!(classify("Unknown Benchmark"), BenchClass::Other);
    }

    #[test]
    fn classify_fallback_by_content() {
        assert_eq!(classify("My CPU Test"), BenchClass::Cpu);
        assert_eq!(classify("Memory Speed"), BenchClass::Mem);
        assert_eq!(classify("Disk Performance"), BenchClass::Disk);
    }

    // Tests pour per_bench_baseline()
    #[test]
    fn per_bench_baseline_cpu_tests() {
        assert_eq!(per_bench_baseline("CPU Multi-Core"), Some(80_000_000));
        assert_eq!(per_bench_baseline("CPU Int Math"), Some(50_000_000));
        assert_eq!(per_bench_baseline("CPU Float Math"), Some(10_000_000));
        assert_eq!(per_bench_baseline("CPU Prime Calc"), Some(2_000_000));
        assert_eq!(per_bench_baseline("CPU SSE Ext"), Some(50_000_000));
    }

    #[test]
    fn per_bench_baseline_memory_tests() {
        assert_eq!(per_bench_baseline("Mem DB Ops"), Some(10_000_000));
        assert_eq!(per_bench_baseline("Mem Cached Read"), Some(50_000));
        assert_eq!(per_bench_baseline("Mem Uncached Read"), Some(20_000));
        assert_eq!(per_bench_baseline("Mem Write"), Some(20_000));
    }

    #[test]
    fn per_bench_baseline_disk_tests() {
        assert_eq!(per_bench_baseline("Disk Seq Read"), Some(500));
        assert_eq!(per_bench_baseline("Disk Seq Write"), Some(400));
        assert_eq!(per_bench_baseline("Disk IOPS 32K QD20"), Some(50_000));
        assert_eq!(per_bench_baseline("Disk IOPS 4K QD1"), Some(10_000));
    }

    #[test]
    fn per_bench_baseline_unknown_returns_none() {
        assert_eq!(per_bench_baseline("Unknown Benchmark"), None);
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
        // Tous devraient être à 1000 car ils correspondent aux baselines
        assert_eq!(aggregated.cpu, 1000);
        assert_eq!(aggregated.mem, 1000);
        assert_eq!(aggregated.disk, 1000);
        assert!(aggregated.global > 0);
    }

    #[test]
    fn aggregated_scores_weighted_average() {
        // Deux benchmarks CPU avec des poids différents
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000, 3), // normalized: 1000, weight: 3 -> 3000
            bench("CPU Int Math", 50_000_000, 2),   // normalized: 1000, weight: 2 -> 2000
        ];
        let aggregated = compute_aggregated_scores(&scores);
        // (3000 + 2000) / (3 + 2) = 5000 / 5 = 1000
        assert_eq!(aggregated.cpu, 1000);
    }

    #[test]
    fn aggregated_scores_capped_at_max() {
        // Score très élevé devrait être plafonné
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000 * 1000, 2), // Très haut score
        ];
        let aggregated = compute_aggregated_scores(&scores);
        // Doit être plafonné à 999_999
        assert!(aggregated.global <= 999_999);
        assert!(aggregated.cpu <= 999_999);
    }

    #[test]
    fn aggregated_scores_handles_mixed_categories() {
        let scores = vec![
            bench("CPU Multi-Core", 80_000_000, 2),
            bench("CPU Int Math", 25_000_000, 2),  // 500 normalisé
            bench("Mem Write", 20_000, 2),
            bench("Disk Seq Read", 500, 2),
        ];
        let aggregated = compute_aggregated_scores(&scores);
        // CPU: (1000*2 + 500*2) / (2+2) = (2000 + 1000) / 4 = 750
        assert_eq!(aggregated.cpu, 750);
        assert_eq!(aggregated.mem, 1000);
        assert_eq!(aggregated.disk, 1000);
    }
}
