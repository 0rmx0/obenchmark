use serde::{Deserialize, Serialize};

/// Result of a single benchmark measurement sample
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct SampleResult {
    /// The raw score value
    pub value: u64,
    /// All sample values collected during measurement
    pub samples: Vec<u64>,
    /// Standard deviation of samples (0 if only one sample)
    pub std_dev: f64,
    /// Minimum sample value
    pub min: u64,
    /// Maximum sample value
    pub max: u64,
    /// Number of samples collected
    pub sample_count: usize,
}

impl SampleResult {
    /// Create a new SampleResult from a vector of samples
    pub fn from_samples(samples: Vec<u64>) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        
        let sample_count = samples.len();
        let min = *samples.iter().min().unwrap_or(&0);
        let max = *samples.iter().max().unwrap_or(&0);
        
        let mean: f64 = samples.iter().map(|&x| x as f64).sum::<f64>() / sample_count as f64;
        
        let std_dev = if sample_count > 1 {
            let variance: f64 = samples.iter()
                .map(|&x| (x as f64 - mean).powi(2))
                .sum::<f64>() / (sample_count - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        Self {
            value: mean.round() as u64,
            samples,
            std_dev,
            min,
            max,
            sample_count,
        }
    }
    
    /// Check if standard deviation is within acceptable limits (<= 15% of mean)
    pub fn is_consistent(&self, max_std_dev_percent: f64) -> bool {
        if self.sample_count <= 1 {
            return true; // No deviation with single sample
        }
        let mean = self.value as f64;
        if mean <= 0.0 {
            return true; // Avoid division by zero
        }
        (self.std_dev / mean) * 100.0 <= max_std_dev_percent
    }
    
    /// Consistency ratio (0.0 to 1.0, higher is better)
    pub fn consistency_ratio(&self) -> f64 {
        if self.sample_count <= 1 || self.value == 0 {
            return 1.0;
        }
        let mean = self.value as f64;
        let cv = self.std_dev / mean; // Coefficient of variation
        (1.0 - cv.min(1.0)).max(0.0) // Clamp to [0, 1]
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BenchScore {
    pub name: String,
    pub raw_score: u64,
    pub weight: u64,
    /// Statistical information from sampling
    #[serde(default)]
    pub samples: Option<SampleResult>,
    /// Standard deviation percentage
    #[serde(default)]
    pub std_dev_percent: Option<f64>,
    /// Minimum sample value
    #[serde(default)]
    pub min: Option<u64>,
    /// Maximum sample value
    #[serde(default)]
    pub max: Option<u64>,
}

/// Validation report for benchmark results
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct ValidationReport {
    /// List of validation warnings
    pub warnings: Vec<String>,
    /// List of validation errors
    pub errors: Vec<String>,
    /// Overall validation passed
    pub passed: bool,
    /// Consistency issues found
    #[serde(default)]
    pub consistency_issues: Vec<ConsistencyIssue>,
}

/// Individual consistency issue for a benchmark
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct ConsistencyIssue {
    pub benchmark_name: String,
    pub std_dev_percent: f64,
    pub consistency_ratio: f64,
    pub severity: String, // "low", "medium", "high"
}

/// Single benchmark error information
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct BenchError {
    pub benchmark_name: String,
    pub error_type: String, // "timeout", "execution_error", "validation_error"
    pub message: String,
    pub timestamp: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BenchResult {
    pub scores: Vec<BenchScore>,
    pub final_score: u64,
    #[serde(default)]
    pub cpu_score: u64,
    #[serde(default)]
    pub mem_score: u64,
    #[serde(default)]
    pub disk_score: u64,
    #[serde(default)]
    pub gfx_score: u64,
    pub system_info: Option<SystemInfo>,
    
    // New fields for reliability and validation
    #[serde(default)]
    pub errors: Vec<BenchError>,
    #[serde(default)]
    pub validation: Option<ValidationReport>,
    #[serde(default)]
    pub completed_benchmarks: usize,
    #[serde(default)]
    pub failed_benchmarks: usize,
    #[serde(default)]
    pub skipped_benchmarks: usize,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CpuInfo {
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub cores_logical: usize,
    pub cores_physical: Option<usize>,
    pub frequency_mhz: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct RamInfo {
    pub total_mb: u64,
    #[serde(default)]
    pub ram_type: Option<String>,
    pub modules: Vec<MemoryModule>,
    pub total_readable: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct MemoryModule {
    pub vendor: Option<String>,
    pub part_number: Option<String>,
    pub size_mb: Option<u64>,
    #[serde(default)]
    pub memory_type: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DiskInfo {
    pub name: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub disk_type: Option<String>, // "HDD" | "SSD" | "NVMe" | "Unknown"
    pub mount_point: Option<String>,
    pub total_bytes: Option<u64>,
    pub size_readable: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SystemInfo {
    pub cpu: CpuInfo,
    pub ram: RamInfo,
    pub disks: Vec<DiskInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_score_serialization() {
        let score = BenchScore {
            name: "CPU Multi-Core".to_string(),
            raw_score: 1000,
            weight: 2,
            samples: None,
            std_dev_percent: None,
            min: None,
            max: None,
        };
        
        let json = serde_json::to_string(&score).unwrap();
        let deserialized: BenchScore = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, "CPU Multi-Core");
        assert_eq!(deserialized.raw_score, 1000);
        assert_eq!(deserialized.weight, 2);
    }

    #[test]
    fn bench_result_serialization() {
        let result = BenchResult {
            scores: vec![
                BenchScore {
                    name: "CPU Multi-Core".to_string(),
                    raw_score: 1000,
                    weight: 2,
                },
                BenchScore {
                    name: "Mem Write".to_string(),
                    raw_score: 2000,
                    weight: 2,
                },
            ],
            final_score: 1500,
            cpu_score: 1000,
            mem_score: 2000,
            disk_score: 0,
            system_info: None,
        };
        
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BenchResult = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.final_score, 1500);
        assert_eq!(deserialized.cpu_score, 1000);
        assert_eq!(deserialized.mem_score, 2000);
        assert_eq!(deserialized.disk_score, 0);
        assert_eq!(deserialized.scores.len(), 2);
    }

    #[test]
    fn cpu_info_serialization() {
        let cpu = CpuInfo {
            vendor: Some("Intel".to_string()),
            model: Some("Core i7-1185G7".to_string()),
            cores_logical: 8,
            cores_physical: Some(4),
            frequency_mhz: Some(2800),
        };
        
        let json = serde_json::to_string(&cpu).unwrap();
        let deserialized: CpuInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.vendor, Some("Intel".to_string()));
        assert_eq!(deserialized.model, Some("Core i7-1185G7".to_string()));
        assert_eq!(deserialized.cores_logical, 8);
        assert_eq!(deserialized.cores_physical, Some(4));
        assert_eq!(deserialized.frequency_mhz, Some(2800));
    }

    #[test]
    fn ram_info_serialization() {
        let ram = RamInfo {
            total_mb: 16384,
            ram_type: Some("DDR4".to_string()),
            modules: vec![
                MemoryModule {
                    vendor: Some("Samsung".to_string()),
                    part_number: Some("M471A1K43DB1".to_string()),
                    size_mb: Some(8192),
                    memory_type: Some("DDR4".to_string()),
                },
            ],
            total_readable: Some("16.00 GB".to_string()),
        };
        
        let json = serde_json::to_string(&ram).unwrap();
        let deserialized: RamInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.total_mb, 16384);
        assert_eq!(deserialized.ram_type, Some("DDR4".to_string()));
        assert_eq!(deserialized.modules.len(), 1);
    }

    #[test]
    fn disk_info_serialization() {
        let disk = DiskInfo {
            name: "/dev/sda".to_string(),
            vendor: Some("Samsung".to_string()),
            model: Some("870 EVO".to_string()),
            disk_type: Some("SSD".to_string()),
            mount_point: Some("/".to_string()),
            total_bytes: Some(500_000_000_000),
            size_readable: Some("465.76 GB".to_string()),
        };
        
        let json = serde_json::to_string(&disk).unwrap();
        let deserialized: DiskInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, "/dev/sda");
        assert_eq!(deserialized.vendor, Some("Samsung".to_string()));
        assert_eq!(deserialized.disk_type, Some("SSD".to_string()));
    }

    #[test]
    fn system_info_serialization() {
        let system = SystemInfo {
            cpu: CpuInfo::default(),
            ram: RamInfo::default(),
            disks: vec![DiskInfo::default()],
        };
        
        let json = serde_json::to_string(&system).unwrap();
        let deserialized: SystemInfo = serde_json::from_str(&json).unwrap();
        
        assert!(deserialized.disks.len() == 1);
    }

    #[test]
    fn bench_result_with_system_info() {
        let result = BenchResult {
            scores: vec![],
            final_score: 1000,
            cpu_score: 1000,
            mem_score: 1000,
            disk_score: 1000,
            system_info: Some(SystemInfo {
                cpu: CpuInfo {
                    vendor: Some("Intel".to_string()),
                    model: Some("Test".to_string()),
                    cores_logical: 4,
                    cores_physical: Some(2),
                    frequency_mhz: Some(2000),
                },
                ram: RamInfo {
                    total_mb: 8192,
                    ram_type: Some("DDR3".to_string()),
                    modules: vec![],
                    total_readable: Some("8.00 GB".to_string()),
                },
                disks: vec![
                    DiskInfo {
                        name: "/dev/sda".to_string(),
                        vendor: Some("Test".to_string()),
                        model: Some("Test".to_string()),
                        disk_type: Some("HDD".to_string()),
                        mount_point: Some("/".to_string()),
                        total_bytes: Some(100_000_000_000),
                        size_readable: Some("93.13 GB".to_string()),
                    },
                ],
            }),
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("Intel"));
        assert!(json.contains("DDR3"));
        
        let deserialized: BenchResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.system_info.is_some());
        assert_eq!(deserialized.system_info.as_ref().unwrap().disks.len(), 1);
    }

    #[test]
    fn bench_score_default() {
        let score = BenchScore::default();
        assert_eq!(score.name, "");
        assert_eq!(score.raw_score, 0);
        assert_eq!(score.weight, 0);
        assert!(score.samples.is_none());
    }

    #[test]
    fn cpu_info_default() {
        let cpu = CpuInfo::default();
        assert_eq!(cpu.cores_logical, 0);
        assert!(cpu.vendor.is_none());
        assert!(cpu.model.is_none());
        assert!(cpu.frequency_mhz.is_none());
    }

    // New tests for SampleResult
    #[test]
    fn sample_result_from_samples() {
        let samples = vec![100, 110, 90, 105, 95];
        let result = SampleResult::from_samples(samples.clone());
        
        assert_eq!(result.sample_count, 5);
        assert_eq!(result.min, 90);
        assert_eq!(result.max, 110);
        assert!(result.std_dev > 0.0);
        assert!(result.value > 0);
    }

    #[test]
    fn sample_result_single_sample() {
        let samples = vec![100];
        let result = SampleResult::from_samples(samples);
        
        assert_eq!(result.sample_count, 1);
        assert_eq!(result.min, 100);
        assert_eq!(result.max, 100);
        assert_eq!(result.std_dev, 0.0);
        assert_eq!(result.value, 100);
    }

    #[test]
    fn sample_result_empty_samples() {
        let samples = vec![];
        let result = SampleResult::from_samples(samples);
        
        assert_eq!(result.sample_count, 0);
        assert_eq!(result.value, 0);
        assert_eq!(result.std_dev, 0.0);
    }

    #[test]
    fn sample_result_consistency_check() {
        // Low deviation samples - should be consistent
        let consistent_samples = vec![100, 101, 99, 100, 100]; // ~1% std dev
        let result = SampleResult::from_samples(consistent_samples);
        assert!(result.is_consistent(15.0));
        
        // High deviation samples - should be inconsistent
        let inconsistent_samples = vec![100, 200, 50, 150, 75]; // ~50% std dev
        let result = SampleResult::from_samples(inconsistent_samples);
        assert!(!result.is_consistent(15.0));
    }

    #[test]
    fn sample_result_consistency_ratio() {
        // Perfect consistency
        let perfect_samples = vec![100, 100, 100, 100];
        let result = SampleResult::from_samples(perfect_samples);
        assert_eq!(result.consistency_ratio(), 1.0);
        
        // Some variation
        let varied_samples = vec![100, 110, 90];
        let result = SampleResult::from_samples(varied_samples);
        assert!(result.consistency_ratio() > 0.0);
        assert!(result.consistency_ratio() < 1.0);
    }

    // Tests for new error and validation structures
    #[test]
    fn bench_error_serialization() {
        let error = BenchError {
            benchmark_name: "CPU Multi-Core".to_string(),
            error_type: "timeout".to_string(),
            message: "Benchmark timed out after 30s".to_string(),
            timestamp: Some("2024-01-01T12:00:00Z".to_string()),
        };
        
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: BenchError = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.benchmark_name, "CPU Multi-Core");
        assert_eq!(deserialized.error_type, "timeout");
        assert_eq!(deserialized.message, "Benchmark timed out after 30s");
    }

    #[test]
    fn validation_report_serialization() {
        let report = ValidationReport {
            warnings: vec!["High std dev on CPU test".to_string()],
            errors: vec![],
            passed: true,
            consistency_issues: vec![],
        };
        
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ValidationReport = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.warnings.len(), 1);
        assert!(deserialized.passed);
    }

    #[test]
    fn consistency_issue_serialization() {
        let issue = ConsistencyIssue {
            benchmark_name: "CPU Multi-Core".to_string(),
            std_dev_percent: 18.5,
            consistency_ratio: 0.815,
            severity: "high".to_string(),
        };
        
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: ConsistencyIssue = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.benchmark_name, "CPU Multi-Core");
        assert!((deserialized.std_dev_percent - 18.5).abs() < 0.01);
    }
}
