use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BenchScore {
    pub name: String,
    pub raw_score: u64,
    pub weight: u64,
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
    }

    #[test]
    fn cpu_info_default() {
        let cpu = CpuInfo::default();
        assert_eq!(cpu.cores_logical, 0);
        assert!(cpu.vendor.is_none());
        assert!(cpu.model.is_none());
        assert!(cpu.frequency_mhz.is_none());
    }
}
