/// Format a byte count as a human-readable string (binary units).
pub fn human_bytes(bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut bytes = bytes;
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
        assert_eq!(human_bytes(1536.0), "1.50 KB");
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
        assert_eq!(
            human_bytes(1024.0 * 1024.0 * 1024.0 * 1024.0 * 2.5),
            "2.50 TB"
        );
    }
}
