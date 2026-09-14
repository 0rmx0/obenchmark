/// Convertit un nombre d'octets en chaîne lisible (B, KB, MB, GB, TB).
pub fn human_bytes(mut bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut i = 0usize;
    while bytes >= 1024.0 && i < units.len() - 1 {
        bytes /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes as u64, units[i])
    } else {
        format!("{:.2} {}", bytes, units[i])
    }
}

/// Unité affichée pour un benchmark donné, à partir de son nom canonique.
pub fn unit_for_bench(name: &str) -> &'static str {
    match name {
        "CPU Multi-Core" | "CPU Int Math" | "CPU Float Math" | "CPU Prime Calc" | "CPU SSE Ext"
        | "CPU Physics" | "CPU Sorting" | "CPU UCT Single" => "ops/s",
        "CPU Compression" | "CPU Encryption" => "MB/s",
        "Mem DB Ops" => "ops/s",
        "Mem Cached Read" | "Mem Uncached Read" | "Mem Write" | "Mem Threaded" => "MB/s",
        "Mem Available" => "MB",
        "Mem Latency" => "accès/s",
        "Disk Seq Read" | "Disk Seq Write" => "MB/s",
        "Disk IOPS 32K QD20" | "Disk IOPS 4K QD1" => "IOPS",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_zero() {
        assert_eq!(human_bytes(0.0), "0 B");
    }

    #[test]
    fn human_bytes_small_values() {
        assert_eq!(human_bytes(100.0), "100 B");
        assert_eq!(human_bytes(1023.0), "1023 B");
    }

    #[test]
    fn human_bytes_kilobytes() {
        assert_eq!(human_bytes(1024.0), "1.00 KB");
        assert_eq!(human_bytes(2048.0), "2.00 KB");
    }

    #[test]
    fn human_bytes_megabytes() {
        assert_eq!(human_bytes(1024.0 * 1024.0), "1.00 MB");
        assert_eq!(human_bytes(1024.0 * 1024.0 * 2.5), "2.50 MB");
    }

    #[test]
    fn human_bytes_gigabytes() {
        assert_eq!(human_bytes(1024.0 * 1024.0 * 1024.0), "1.00 GB");
        assert_eq!(human_bytes(1024.0 * 1024.0 * 1024.0 * 5.5), "5.50 GB");
    }

    #[test]
    fn human_bytes_terabytes() {
        assert_eq!(human_bytes(1024.0 * 1024.0 * 1024.0 * 1024.0), "1.00 TB");
    }

    #[test]
    fn human_bytes_very_large() {
        let bytes = 1024.0 * 1024.0 * 1024.0 * 1024.0 * 2.5;
        assert_eq!(human_bytes(bytes), "2.50 TB");
    }

    #[test]
    fn human_bytes_fractional() {
        assert_eq!(human_bytes(1536.0), "1.50 KB");
    }

    #[test]
    fn unit_for_bench_cpu_ops() {
        assert_eq!(unit_for_bench("CPU Multi-Core"), "ops/s");
        assert_eq!(unit_for_bench("CPU Int Math"), "ops/s");
        assert_eq!(unit_for_bench("CPU Float Math"), "ops/s");
        assert_eq!(unit_for_bench("CPU Prime Calc"), "ops/s");
        assert_eq!(unit_for_bench("CPU SSE Ext"), "ops/s");
        assert_eq!(unit_for_bench("CPU Physics"), "ops/s");
        assert_eq!(unit_for_bench("CPU Sorting"), "ops/s");
        assert_eq!(unit_for_bench("CPU UCT Single"), "ops/s");
    }

    #[test]
    fn unit_for_bench_cpu_mb() {
        assert_eq!(unit_for_bench("CPU Compression"), "MB/s");
        assert_eq!(unit_for_bench("CPU Encryption"), "MB/s");
    }

    #[test]
    fn unit_for_bench_memory() {
        assert_eq!(unit_for_bench("Mem DB Ops"), "ops/s");
        assert_eq!(unit_for_bench("Mem Cached Read"), "MB/s");
        assert_eq!(unit_for_bench("Mem Uncached Read"), "MB/s");
        assert_eq!(unit_for_bench("Mem Write"), "MB/s");
        assert_eq!(unit_for_bench("Mem Available"), "MB");
        assert_eq!(unit_for_bench("Mem Latency"), "accès/s");
        assert_eq!(unit_for_bench("Mem Threaded"), "MB/s");
    }

    #[test]
    fn unit_for_bench_disk() {
        assert_eq!(unit_for_bench("Disk Seq Read"), "MB/s");
        assert_eq!(unit_for_bench("Disk Seq Write"), "MB/s");
        assert_eq!(unit_for_bench("Disk IOPS 32K QD20"), "IOPS");
        assert_eq!(unit_for_bench("Disk IOPS 4K QD1"), "IOPS");
    }

    #[test]
    fn unit_for_bench_unknown() {
        assert_eq!(unit_for_bench("Unknown"), "");
    }
}
