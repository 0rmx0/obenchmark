use crate::benchmarks::{cpu, disk, graphics, memory};
use crate::engines::benchmark::Benchmark;

/// Describes a benchmark that can be instantiated on demand.
#[derive(Clone, Copy)]
pub struct BenchSpec {
    pub name: &'static str,
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
        builder: build_cpu_multi_core,
    },
    BenchSpec {
        name: "CPU Int Math",
        builder: build_cpu_int_math,
    },
    BenchSpec {
        name: "CPU Float Math",
        builder: build_cpu_float_math,
    },
    BenchSpec {
        name: "CPU Prime Calc",
        builder: build_cpu_prime_calc,
    },
    BenchSpec {
        name: "CPU SSE Ext",
        builder: build_cpu_sse,
    },
    BenchSpec {
        name: "CPU Compression",
        builder: build_cpu_compression,
    },
    BenchSpec {
        name: "CPU Encryption",
        builder: build_cpu_encryption,
    },
    BenchSpec {
        name: "CPU Physics",
        builder: build_cpu_physics,
    },
    BenchSpec {
        name: "CPU Sorting",
        builder: build_cpu_sorting,
    },
    BenchSpec {
        name: "CPU UCT Single",
        builder: build_cpu_uct,
    },
    BenchSpec {
        name: "Mem DB Ops",
        builder: build_mem_db_ops,
    },
    BenchSpec {
        name: "Mem Cached Read",
        builder: build_mem_cached_read,
    },
    BenchSpec {
        name: "Mem Uncached Read",
        builder: build_mem_uncached_read,
    },
    BenchSpec {
        name: "Mem Write",
        builder: build_mem_write,
    },
    BenchSpec {
        name: "Mem Available",
        builder: build_mem_available,
    },
    BenchSpec {
        name: "Mem Latency",
        builder: build_mem_latency,
    },
    BenchSpec {
        name: "Mem Threaded",
        builder: build_mem_threaded,
    },
    BenchSpec {
        name: "Disk Seq Read",
        builder: build_disk_seq_read,
    },
    BenchSpec {
        name: "Disk Seq Write",
        builder: build_disk_seq_write,
    },
    BenchSpec {
        name: "Disk IOPS 32K QD20",
        builder: build_disk_iops_32k,
    },
    BenchSpec {
        name: "Disk IOPS 4K QD1",
        builder: build_disk_iops_4k,
    },
    BenchSpec {
        name: "GFX 2D Raster",
        builder: build_gfx_2d_raster,
    },
    BenchSpec {
        name: "GFX 3D Raster",
        builder: build_gfx_3d_raster,
    },
];

pub fn default_suite() -> Vec<Box<dyn Benchmark>> {
    AVAILABLE_BENCHMARKS
        .iter()
        .map(|spec| spec.build())
        .collect()
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

fn build_gfx_2d_raster() -> Box<dyn Benchmark> {
    Box::new(graphics::Gfx2DRaster)
}

fn build_gfx_3d_raster() -> Box<dyn Benchmark> {
    Box::new(graphics::Gfx3DRaster)
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
            let bench = spec.build();
            assert!(bench.weight() > 0, "Benchmark {} has zero weight", bench.name());
        }
    }

    #[test]
    fn all_benchmarks_have_non_empty_name() {
        for spec in AVAILABLE_BENCHMARKS {
            assert!(
                !spec.name.is_empty(),
                "Benchmark has empty name"
            );
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

    // Tests pour les builders individuels
    #[test]
    fn test_cpu_multi_core_builder() {
        let bench = build_cpu_multi_core();
        assert_eq!(bench.name(), "CPU Multi-Core");
        assert_eq!(bench.weight(), 3);
    }

    #[test]
    fn test_cpu_int_math_builder() {
        let bench = build_cpu_int_math();
        assert_eq!(bench.name(), "CPU Int Math");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_float_math_builder() {
        let bench = build_cpu_float_math();
        assert_eq!(bench.name(), "CPU Float Math");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_prime_calc_builder() {
        let bench = build_cpu_prime_calc();
        assert_eq!(bench.name(), "CPU Prime Calc");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_sse_builder() {
        let bench = build_cpu_sse();
        assert_eq!(bench.name(), "CPU SSE Ext");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_compression_builder() {
        let bench = build_cpu_compression();
        assert_eq!(bench.name(), "CPU Compression");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_encryption_builder() {
        let bench = build_cpu_encryption();
        assert_eq!(bench.name(), "CPU Encryption");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_physics_builder() {
        let bench = build_cpu_physics();
        assert_eq!(bench.name(), "CPU Physics");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_sorting_builder() {
        let bench = build_cpu_sorting();
        assert_eq!(bench.name(), "CPU Sorting");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_cpu_uct_builder() {
        let bench = build_cpu_uct();
        assert_eq!(bench.name(), "CPU UCT Single");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_mem_db_ops_builder() {
        let bench = build_mem_db_ops();
        assert_eq!(bench.name(), "Mem DB Ops");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_mem_cached_read_builder() {
        let bench = build_mem_cached_read();
        assert_eq!(bench.name(), "Mem Cached Read");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_mem_uncached_read_builder() {
        let bench = build_mem_uncached_read();
        assert_eq!(bench.name(), "Mem Uncached Read");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_mem_write_builder() {
        let bench = build_mem_write();
        assert_eq!(bench.name(), "Mem Write");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_mem_available_builder() {
        let bench = build_mem_available();
        assert_eq!(bench.name(), "Mem Available");
        assert_eq!(bench.weight(), 1);
    }

    #[test]
    fn test_mem_latency_builder() {
        let bench = build_mem_latency();
        assert_eq!(bench.name(), "Mem Latency");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_mem_threaded_builder() {
        let bench = build_mem_threaded();
        assert_eq!(bench.name(), "Mem Threaded");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_disk_seq_read_builder() {
        let bench = build_disk_seq_read();
        assert_eq!(bench.name(), "Disk Seq Read");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_disk_seq_write_builder() {
        let bench = build_disk_seq_write();
        assert_eq!(bench.name(), "Disk Seq Write");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_disk_iops_32k_builder() {
        let bench = build_disk_iops_32k();
        assert_eq!(bench.name(), "Disk IOPS 32K QD20");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_disk_iops_4k_builder() {
        let bench = build_disk_iops_4k();
        assert_eq!(bench.name(), "Disk IOPS 4K QD1");
        assert_eq!(bench.weight(), 2);
    }

    #[test]
    fn test_gfx_2d_raster_builder() {
        let bench = build_gfx_2d_raster();
        assert_eq!(bench.name(), "GFX 2D Raster");
        assert_eq!(bench.weight(), 3);
    }

    #[test]
    fn test_gfx_3d_raster_builder() {
        let bench = build_gfx_3d_raster();
        assert_eq!(bench.name(), "GFX 3D Raster");
        assert_eq!(bench.weight(), 3);
    }

    #[test]
    fn test_all_benchmarks_can_be_built() {
        for spec in AVAILABLE_BENCHMARKS {
            let bench = spec.build();
            // Just verify it can be built without panicking
            assert!(!bench.name().is_empty());
            assert!(bench.weight() > 0);
        }
    }

    #[test]
    fn test_benchmark_count() {
        // CPU: 10, Memory: 7, Disk: 4, GFX: 2 = 23 total
        assert_eq!(AVAILABLE_BENCHMARKS.len(), 23);
    }

    #[test]
    fn test_gfx_benchmarks_count() {
        let gfx_benchmarks: Vec<_> = AVAILABLE_BENCHMARKS
            .iter()
            .filter(|s| s.name.starts_with("GFX"))
            .collect();
        assert_eq!(gfx_benchmarks.len(), 2);
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
        assert_eq!(disk_benchmarks.len(), 4);
    }
}
