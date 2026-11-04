use anyhow::Result;

/// A single benchmark. Metadata (unit, category, baseline, weight) lives in
/// `BenchSpec` / `bench_meta`, not here, so there is a single source of truth.
pub trait Benchmark: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self) -> Result<u64>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchCategory {
    Cpu,
    Mem,
    Disk,
    Other,
}

/// Static metadata attached to a benchmark name.
#[derive(Clone, Copy, Debug)]
pub struct BenchMeta {
    pub unit: &'static str,
    pub category: BenchCategory,
    pub baseline: u64,
}

/// Family baselines for unknown benchmark names (~1000 = reference machine).
const CPU_BASELINE: u64 = 50_000_000;
const MEM_BASELINE: u64 = 5_000;
const DISK_BASELINE: u64 = 1_000;
const FALLBACK_BASELINE: u64 = 1_000;

/// Central metadata table. Add new benchmarks here (and in the suite) and
/// scoring, CLI units and GUI display pick them up automatically.
pub fn bench_meta(name: &str) -> BenchMeta {
    let meta = match name {
        // CPU — ops/s
        "CPU Multi-Core" => (80_000_000, "ops/s", BenchCategory::Cpu),
        "CPU Int Math" => (50_000_000, "ops/s", BenchCategory::Cpu),
        "CPU Float Math" => (10_000_000, "ops/s", BenchCategory::Cpu),
        "CPU Prime Calc" => (400_000, "ops/s", BenchCategory::Cpu),
        "CPU SSE Ext" => (50_000_000, "ops/s", BenchCategory::Cpu),
        "CPU Physics" => (100_000_000, "ops/s", BenchCategory::Cpu),
        "CPU Sorting" => (50_000_000, "ops/s", BenchCategory::Cpu),
        "CPU UCT Single" => (2_000_000, "ops/s", BenchCategory::Cpu),
        // CPU — MB/s
        "CPU Compression" => (500, "MB/s", BenchCategory::Cpu),
        "CPU Encryption" => (500, "MB/s", BenchCategory::Cpu),
        // Memory
        "Mem DB Ops" => (10_000_000, "ops/s", BenchCategory::Mem),
        "Mem Cached Read" => (50_000, "MB/s", BenchCategory::Mem),
        "Mem Uncached Read" => (20_000, "MB/s", BenchCategory::Mem),
        "Mem Write" => (20_000, "MB/s", BenchCategory::Mem),
        "Mem Available" => (8 * 1024, "MB", BenchCategory::Mem),
        "Mem Latency" => (50_000_000, "accès/s", BenchCategory::Mem),
        "Mem Threaded" => (50_000, "MB/s", BenchCategory::Mem),
        // Disk
        "Disk Seq Read" => (500, "MB/s", BenchCategory::Disk),
        "Disk Seq Write" => (400, "MB/s", BenchCategory::Disk),
        "Disk IOPS 32K QD20" => (50_000, "IOPS", BenchCategory::Disk),
        "Disk IOPS 4K QD1" => (10_000, "IOPS", BenchCategory::Disk),
        "Disk IOPS 32K QD20 Write" => (30_000, "IOPS", BenchCategory::Disk),
        "Disk IOPS 4K QD1 Write" => (5_000, "IOPS", BenchCategory::Disk),
        _ => {
            let lower = name.to_lowercase();
            if lower.contains("cpu") {
                (CPU_BASELINE, "ops/s", BenchCategory::Cpu)
            } else if lower.contains("mem") || lower.contains("memory") {
                (MEM_BASELINE, "MB/s", BenchCategory::Mem)
            } else if lower.contains("disk") || lower.contains("iops") {
                (DISK_BASELINE, "IOPS", BenchCategory::Disk)
            } else {
                (FALLBACK_BASELINE, "", BenchCategory::Other)
            }
        }
    };
    BenchMeta {
        baseline: meta.0,
        unit: meta.1,
        category: meta.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_cpu_reference_values() {
        assert_eq!(bench_meta("CPU Multi-Core").baseline, 80_000_000);
        assert_eq!(bench_meta("CPU Multi-Core").unit, "ops/s");
        assert_eq!(bench_meta("CPU Multi-Core").category, BenchCategory::Cpu);
        assert_eq!(bench_meta("CPU Int Math").baseline, 50_000_000);
        assert_eq!(bench_meta("CPU Float Math").baseline, 10_000_000);
        assert_eq!(bench_meta("CPU Prime Calc").baseline, 400_000);
        assert_eq!(bench_meta("CPU SSE Ext").baseline, 50_000_000);
        assert_eq!(bench_meta("CPU Compression").unit, "MB/s");
        assert_eq!(bench_meta("CPU Encryption").baseline, 500);
        assert_eq!(bench_meta("CPU Physics").baseline, 100_000_000);
        assert_eq!(bench_meta("CPU Sorting").baseline, 50_000_000);
        assert_eq!(bench_meta("CPU UCT Single").baseline, 2_000_000);
    }

    #[test]
    fn meta_memory_values() {
        assert_eq!(bench_meta("Mem DB Ops").category, BenchCategory::Mem);
        assert_eq!(bench_meta("Mem DB Ops").unit, "ops/s");
        assert_eq!(bench_meta("Mem Cached Read").baseline, 50_000);
        assert_eq!(bench_meta("Mem Uncached Read").baseline, 20_000);
        assert_eq!(bench_meta("Mem Write").baseline, 20_000);
        assert_eq!(bench_meta("Mem Available").unit, "MB");
        assert_eq!(bench_meta("Mem Available").baseline, 8 * 1024);
        assert_eq!(bench_meta("Mem Latency").unit, "accès/s");
        assert_eq!(bench_meta("Mem Threaded").baseline, 50_000);
    }

    #[test]
    fn meta_disk_values() {
        assert_eq!(bench_meta("Disk Seq Read").baseline, 500);
        assert_eq!(bench_meta("Disk Seq Read").category, BenchCategory::Disk);
        assert_eq!(bench_meta("Disk Seq Write").baseline, 400);
        assert_eq!(bench_meta("Disk IOPS 32K QD20").baseline, 50_000);
        assert_eq!(bench_meta("Disk IOPS 32K QD20").unit, "IOPS");
        assert_eq!(bench_meta("Disk IOPS 4K QD1").baseline, 10_000);
        assert_eq!(bench_meta("Disk IOPS 32K QD20 Write").baseline, 30_000);
        assert_eq!(bench_meta("Disk IOPS 4K QD1 Write").baseline, 5_000);
    }

    #[test]
    fn meta_fallback_by_substring() {
        assert_eq!(bench_meta("My CPU Test").category, BenchCategory::Cpu);
        assert_eq!(bench_meta("My CPU Test").baseline, CPU_BASELINE);
        assert_eq!(bench_meta("Memory Speed").category, BenchCategory::Mem);
        assert_eq!(bench_meta("Disk Performance").category, BenchCategory::Disk);
        assert_eq!(
            bench_meta("Unknown Benchmark").category,
            BenchCategory::Other
        );
        assert_eq!(bench_meta("Unknown Benchmark").baseline, FALLBACK_BASELINE);
    }
}
