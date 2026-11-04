use crate::benchmarks::{cpu, disk, memory};
use crate::engines::benchmark::Benchmark;

/// Describes a benchmark that can be instantiated on demand.
/// Weight lives here (not on the trait) so all metadata for a benchmark
/// is declared in one place.
#[derive(Clone, Copy)]
pub struct BenchSpec {
    pub name: &'static str,
    pub weight: u64,
    pub builder: fn() -> Box<dyn Benchmark>,
}

impl BenchSpec {
    /// Instantiate a new boxed benchmark.
    pub fn build(self) -> Box<dyn Benchmark> {
        (self.builder)()
    }
}

pub const AVAILABLE_BENCHMARKS: &[BenchSpec] = &[
    BenchSpec {
        name: "CPU Multi-Core",
        weight: 3,
        builder: build_cpu_multi_core,
    },
    BenchSpec {
        name: "CPU Int Math",
        weight: 2,
        builder: build_cpu_int_math,
    },
    BenchSpec {
        name: "CPU Float Math",
        weight: 2,
        builder: build_cpu_float_math,
    },
    BenchSpec {
        name: "CPU Prime Calc",
        weight: 2,
        builder: build_cpu_prime_calc,
    },
    BenchSpec {
        name: "CPU SSE Ext",
        weight: 2,
        builder: build_cpu_sse,
    },
    BenchSpec {
        name: "CPU Compression",
        weight: 2,
        builder: build_cpu_compression,
    },
    BenchSpec {
        name: "CPU Encryption",
        weight: 2,
        builder: build_cpu_encryption,
    },
    BenchSpec {
        name: "CPU Physics",
        weight: 2,
        builder: build_cpu_physics,
    },
    BenchSpec {
        name: "CPU Sorting",
        weight: 2,
        builder: build_cpu_sorting,
    },
    BenchSpec {
        name: "CPU UCT Single",
        weight: 2,
        builder: build_cpu_uct,
    },
    BenchSpec {
        name: "Mem DB Ops",
        weight: 2,
        builder: build_mem_db_ops,
    },
    BenchSpec {
        name: "Mem Cached Read",
        weight: 2,
        builder: build_mem_cached_read,
    },
    BenchSpec {
        name: "Mem Uncached Read",
        weight: 2,
        builder: build_mem_uncached_read,
    },
    BenchSpec {
        name: "Mem Write",
        weight: 2,
        builder: build_mem_write,
    },
    BenchSpec {
        name: "Mem Available",
        weight: 1,
        builder: build_mem_available,
    },
    BenchSpec {
        name: "Mem Latency",
        weight: 2,
        builder: build_mem_latency,
    },
    BenchSpec {
        name: "Mem Threaded",
        weight: 2,
        builder: build_mem_threaded,
    },
    BenchSpec {
        name: "Disk Seq Read",
        weight: 2,
        builder: build_disk_seq_read,
    },
    BenchSpec {
        name: "Disk Seq Write",
        weight: 2,
        builder: build_disk_seq_write,
    },
    BenchSpec {
        name: "Disk IOPS 32K QD20",
        weight: 2,
        builder: build_disk_iops_32k,
    },
    BenchSpec {
        name: "Disk IOPS 4K QD1",
        weight: 2,
        builder: build_disk_iops_4k,
    },
    BenchSpec {
        name: "Disk IOPS 32K QD20 Write",
        weight: 2,
        builder: build_disk_iops_32k_write,
    },
    BenchSpec {
        name: "Disk IOPS 4K QD1 Write",
        weight: 2,
        builder: build_disk_iops_4k_write,
    },
];

pub fn default_suite() -> Vec<BenchSpec> {
    AVAILABLE_BENCHMARKS.to_vec()
}

fn build_cpu_multi_core() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuMultiCore)
}

fn build_cpu_int_math() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuIntMath)
}

fn build_cpu_float_math() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuFloatMath)
}

fn build_cpu_prime_calc() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuPrimeCalc)
}

fn build_cpu_sse() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuSSE)
}

fn build_cpu_compression() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuCompression)
}

fn build_cpu_encryption() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuEncryption)
}

fn build_cpu_physics() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuPhysics)
}

fn build_cpu_sorting() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuSorting)
}

fn build_cpu_uct() -> Box<dyn Benchmark> {
    Box::new(cpu::CpuUCT)
}

fn build_mem_db_ops() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryDBOps)
}

fn build_mem_cached_read() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryCachedRead)
}

fn build_mem_uncached_read() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryUncachedRead)
}

fn build_mem_write() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryWrite)
}

fn build_mem_available() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryAvailable)
}

fn build_mem_latency() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryLatency)
}

fn build_mem_threaded() -> Box<dyn Benchmark> {
    Box::new(memory::MemoryThreaded)
}

fn build_disk_seq_read() -> Box<dyn Benchmark> {
    Box::new(disk::DiskSequentialRead)
}

fn build_disk_seq_write() -> Box<dyn Benchmark> {
    Box::new(disk::DiskSequentialWrite)
}

fn build_disk_iops_32k() -> Box<dyn Benchmark> {
    Box::new(disk::DiskRandomIOPS32K)
}

fn build_disk_iops_4k() -> Box<dyn Benchmark> {
    Box::new(disk::DiskRandomIOPS4K)
}

fn build_disk_iops_32k_write() -> Box<dyn Benchmark> {
    Box::new(disk::DiskRandomWrite32K)
}

fn build_disk_iops_4k_write() -> Box<dyn Benchmark> {
    Box::new(disk::DiskRandomWrite4K)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_benchmarks_have_unique_names() {
        let mut names = std::collections::HashSet::new();
        for spec in AVAILABLE_BENCHMARKS {
            assert!(
                names.insert(spec.name),
                "Duplicate benchmark name: {}",
                spec.name
            );
        }
    }

    #[test]
    fn all_benchmarks_have_positive_weight() {
        for spec in AVAILABLE_BENCHMARKS {
            assert!(spec.weight > 0, "Benchmark {} has zero weight", spec.name);
        }
    }

    #[test]
    fn all_benchmarks_have_non_empty_name() {
        for spec in AVAILABLE_BENCHMARKS {
            assert!(!spec.name.is_empty(), "Benchmark has empty name");
        }
    }

    #[test]
    fn default_suite_returns_all_benchmarks() {
        let suite = default_suite();
        assert_eq!(suite.len(), AVAILABLE_BENCHMARKS.len());
    }

    #[test]
    fn build_returns_correct_type() {
        for spec in AVAILABLE_BENCHMARKS {
            let bench = spec.build();
            assert_eq!(bench.name(), spec.name);
        }
    }

    #[test]
    fn spec_weights_match_expected() {
        let spec = |name: &str| {
            AVAILABLE_BENCHMARKS
                .iter()
                .find(|s| s.name == name)
                .unwrap()
        };
        assert_eq!(spec("CPU Multi-Core").weight, 3);
        assert_eq!(spec("CPU Int Math").weight, 2);
        assert_eq!(spec("CPU Float Math").weight, 2);
        assert_eq!(spec("CPU Prime Calc").weight, 2);
        assert_eq!(spec("CPU SSE Ext").weight, 2);
        assert_eq!(spec("CPU Compression").weight, 2);
        assert_eq!(spec("CPU Encryption").weight, 2);
        assert_eq!(spec("CPU Physics").weight, 2);
        assert_eq!(spec("CPU Sorting").weight, 2);
        assert_eq!(spec("CPU UCT Single").weight, 2);
        assert_eq!(spec("Mem DB Ops").weight, 2);
        assert_eq!(spec("Mem Cached Read").weight, 2);
        assert_eq!(spec("Mem Uncached Read").weight, 2);
        assert_eq!(spec("Mem Write").weight, 2);
        assert_eq!(spec("Mem Available").weight, 1);
        assert_eq!(spec("Mem Latency").weight, 2);
        assert_eq!(spec("Mem Threaded").weight, 2);
        assert_eq!(spec("Disk Seq Read").weight, 2);
        assert_eq!(spec("Disk Seq Write").weight, 2);
        assert_eq!(spec("Disk IOPS 32K QD20").weight, 2);
        assert_eq!(spec("Disk IOPS 4K QD1").weight, 2);
        assert_eq!(spec("Disk IOPS 32K QD20 Write").weight, 2);
        assert_eq!(spec("Disk IOPS 4K QD1 Write").weight, 2);
    }

    #[test]
    fn test_benchmark_count() {
        // CPU: 10, Memory: 7, Disk: 6 = 23 total
        assert_eq!(AVAILABLE_BENCHMARKS.len(), 23);
    }

    #[test]
    fn test_cpu_benchmarks_count() {
        let cpu_benchmarks: Vec<_> = AVAILABLE_BENCHMARKS
            .iter()
            .filter(|s| s.name.starts_with("CPU"))
            .collect();
        assert_eq!(cpu_benchmarks.len(), 10);
    }

    #[test]
    fn test_memory_benchmarks_count() {
        let mem_benchmarks: Vec<_> = AVAILABLE_BENCHMARKS
            .iter()
            .filter(|s| s.name.starts_with("Mem"))
            .collect();
        assert_eq!(mem_benchmarks.len(), 7);
    }

    #[test]
    fn test_disk_benchmarks_count() {
        let disk_benchmarks: Vec<_> = AVAILABLE_BENCHMARKS
            .iter()
            .filter(|s| s.name.starts_with("Disk"))
            .collect();
        assert_eq!(disk_benchmarks.len(), 6);
    }
}
