use crate::benchmarks::{cpu, disk, memory};
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
