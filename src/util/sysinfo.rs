use sysinfo::System;

use crate::model::result::{CpuInfo, DiskInfo, MemoryModule, SystemInfo};
use std::collections::HashMap;

/// Convert bytes to human-readable format
fn human_bytes(mut bytes: f64) -> String {
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

/// Get basic system info from sysinfo crate
pub fn get_system_info() -> System {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys
}

/// Get detailed system information with cross-platform support
pub fn get_detailed_system_info() -> SystemInfo {
    let mut sysinfo = SystemInfo::default();

    // Get CPU information
    get_cpu_info(&mut sysinfo);

    // Get RAM information
    get_ram_info(&mut sysinfo);

    // Get disk information
    get_disk_info(&mut sysinfo);

    sysinfo
}

/// Get CPU information with platform-specific enhancements
fn get_cpu_info(sysinfo: &mut SystemInfo) {
    let mut cpu = CpuInfo::default();
    
    // Always get logical cores from num_cpus as fallback
    cpu.cores_logical = num_cpus::get();

    // Try to get info from sysinfo first
    let s = get_system_info();
    let g = s.global_cpu_info();

    // Get CPU brand/model
    let brand = g.brand().trim();
    if !brand.is_empty() {
        cpu.model = Some(brand.to_string());
    }

    // Get CPU vendor
    let vendor = g.vendor_id().trim();
    if !vendor.is_empty() {
        cpu.vendor = Some(vendor.to_string());
    }

    // Get CPU frequency
    let freq = g.frequency();
    if freq > 0 {
        cpu.frequency_mhz = Some(freq as u64);
    }

    // Platform-specific enhancements
    #[cfg(target_os = "windows")]
    {
        get_windows_cpu_info(&mut cpu);
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_cpu_info(&mut cpu);
    }

    #[cfg(target_os = "macos")]
    {
        // sysinfo usually works well on macOS
    }

    sysinfo.cpu = cpu;
}

/// Get RAM information with platform-specific enhancements
fn get_ram_info(sysinfo: &mut SystemInfo) {
    let s = get_system_info();
    let total_kb = s.total_memory();
    let total_mb = total_kb / 1024;

    sysinfo.ram.total_mb = total_mb;
    sysinfo.ram.total_readable = Some(if total_mb >= 1024 {
        format!("{:.2} GB", total_mb as f64 / 1024.0)
    } else {
        format!("{} MB", total_mb)
    });

    // Platform-specific RAM details
    #[cfg(target_os = "windows")]
    {
        get_windows_ram_info(sysinfo);
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_ram_info(sysinfo);
    }
}

/// Get disk information with platform-specific enhancements
fn get_disk_info(sysinfo: &mut SystemInfo) {
    let mut disks = vec![];

    // Get basic disk info from sysinfo
    let sd = sysinfo::Disks::new_with_refreshed_list();
    for d in sd.list() {
        let total = d.total_space();

        disks.push(DiskInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: Some(d.mount_point().to_string_lossy().to_string()),
            total_bytes: Some(total),
            size_readable: Some(human_bytes(total as f64)),
            vendor: None,
            model: None,
            disk_type: None,
        });
    }

    // Platform-specific disk enhancements
    #[cfg(target_os = "windows")]
    {
        get_windows_disk_info(&mut disks);
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_disk_info(&mut disks);
    }

    #[cfg(target_os = "macos")]
    {
        get_macos_disk_info(&mut disks);
    }

    sysinfo.disks = disks;
}

// ============================================================================
// WINDOWS-SPECIFIC IMPLEMENTATIONS
// ============================================================================

#[cfg(target_os = "windows")]
fn get_windows_cpu_info(cpu: &mut CpuInfo) {
    // Try to get CPU info from WMI as fallback
    let wmi_result = wmi::WMIConnection::new();
    
    if let Ok(wmi) = wmi_result {
        // Query for processor information
        if let Ok(procs) = wmi.raw_query::<Win32Processor>(
            "SELECT Name, Manufacturer, MaxClockSpeed, CurrentClockSpeed FROM Win32_Processor"
        ) {
            if let Some(p) = procs.into_iter().next() {
                // Update model if not already set or empty
                if cpu.model.as_ref().map_or(true, |m| m.is_empty()) {
                    if let Some(name) = p.Name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                        cpu.model = Some(name.to_string());
                    }
                }

                // Update vendor if not already set or empty
                if cpu.vendor.as_ref().map_or(true, |v| v.is_empty()) {
                    if let Some(manufacturer) = p.Manufacturer.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                        cpu.vendor = Some(manufacturer.to_string());
                    }
                }

                // Update frequency if not already set
                if cpu.frequency_mhz.is_none() {
                    if let Some(max_speed) = p.MaxClockSpeed {
                        if max_speed > 0 {
                            cpu.frequency_mhz = Some(max_speed as u64);
                        }
                    } else if let Some(current_speed) = p.CurrentClockSpeed {
                        if current_speed > 0 {
                            cpu.frequency_mhz = Some(current_speed as u64);
                        }
                    }
                }
            }
        }
    }

    // If we still don't have model info, try wmic as last resort
    if cpu.model.as_ref().map_or(true, |m| m.is_empty()) {
        if let Ok(model) = get_cpu_name_from_wmic() {
            cpu.model = Some(model);
        }
    }

    // If we still don't have frequency, try to estimate from vendor
    if cpu.frequency_mhz.is_none() && cpu.vendor.is_some() {
        // This is a fallback - modern CPUs typically run at 2-4 GHz
        // We'll leave it as None rather than guessing
    }
}

#[cfg(target_os = "windows")]
fn get_windows_ram_info(sysinfo: &mut SystemInfo) {
    use wmi::WMIConnection;

    let mut modules = Vec::new();
    let mut type_hist: HashMap<String, u32> = HashMap::new();

    let wmi_result = WMIConnection::new();
    
    if let Ok(wmi) = wmi_result {
        // Query for physical memory
        if let Ok(mem_items) = wmi.raw_query::<Win32PhysicalMemory>(
            "SELECT Manufacturer, PartNumber, Capacity, SMBIOSMemoryType, MemoryType FROM Win32_PhysicalMemory"
        ) {
            for m in mem_items {
                let mem_type = m.SMBIOSMemoryType
                    .and_then(map_smbios_memory_type)
                    .or_else(|| m.MemoryType.and_then(map_smbios_memory_type))
                    .map(|s| s.to_string());

                if let Some(t) = &mem_type {
                    *type_hist.entry(t.clone()).or_insert(0) += 1;
                }

                let size_mb = m.Capacity.map(|b| b / 1024 / 1024);

                modules.push(MemoryModule {
                    vendor: m.Manufacturer.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string()),
                    part_number: m.PartNumber.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string()),
                    size_mb,
                    memory_type: mem_type,
                });
            }

            // Determine dominant RAM type
            if sysinfo.ram.ram_type.is_none() {
                if let Some((t, _)) = type_hist.into_iter().max_by_key(|(_, c)| *c) {
                    sysinfo.ram.ram_type = Some(t);
                }
            }
        }
    }

    sysinfo.ram.modules = modules;
}

#[cfg(target_os = "windows")]
fn get_windows_disk_info(disks: &mut Vec<DiskInfo>) {
    use wmi::WMIConnection;

    // For simplicity, apply info to all disks based on WMI data
    // This is a simplified approach that should work for most systems
    let wmi_result = WMIConnection::new();
    
    if let Ok(wmi) = wmi_result {
        // Get disk drive information
        if let Ok(drives) = wmi.raw_query::<Win32DiskDrive>(
            "SELECT DeviceID, Model, Manufacturer, InterfaceType, MediaType FROM Win32_DiskDrive"
        ) {
            for (i, drive) in drives.into_iter().enumerate() {
                if i >= disks.len() {
                    break;
                }

                let disk = &mut disks[i];

                let model = drive.Model.as_deref().map(str::trim).filter(|s| !s.is_empty());
                let vendor = drive.Manufacturer.as_deref().map(str::trim).filter(|s| !s.is_empty());
                let interface = drive.InterfaceType.as_deref().map(str::trim);
                let media = drive.MediaType.as_deref().map(str::trim);
                
                // Determine disk type
                let disk_type = determine_disk_type(interface, model, media);

                if disk.vendor.is_none() && vendor.is_some() {
                    disk.vendor = vendor.map(|s| s.to_string());
                }
                if disk.model.is_none() && model.is_some() {
                    disk.model = model.map(|s| s.to_string());
                }
                if disk.disk_type.is_none() && disk_type.is_some() {
                    disk.disk_type = disk_type;
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn determine_disk_type(
    interface: Option<&str>,
    model: Option<&str>,
    media: Option<&str>,
) -> Option<String> {
    let iface = interface.unwrap_or("").to_lowercase();
    let m = model.unwrap_or("").to_lowercase();
    let media = media.unwrap_or("").to_lowercase();

    if iface.contains("nvme") || m.contains("nvme") {
        return Some("NVMe".to_string());
    }
    if media.contains("ssd") || m.contains("ssd") {
        return Some("SSD".to_string());
    }
    if media.contains("hdd") || m.contains("hdd") {
        return Some("HDD".to_string());
    }
    if media.contains("fixed") {
        // "Fixed hard disk media" => HDD by default
        return Some("HDD".to_string());
    }
    None
}

#[cfg(target_os = "windows")]
fn map_smbios_memory_type(code: u16) -> Option<&'static str> {
    match code {
        20 => Some("DDR"),
        21 => Some("DDR2"),
        24 => Some("DDR3"),
        26 => Some("DDR4"),
        34 => Some("DDR5"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn get_cpu_name_from_wmic() -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("wmic")
        .args(&["cpu", "get", "name", "/value"])
        .output()
        .ok()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.trim().starts_with("Name=") {
                let cpu_name = line.trim()[5..].trim();
                if !cpu_name.is_empty() {
                    return Some(cpu_name.to_string());
                }
            }
        }
    }
    
    None
}

// WMI Structs for Windows
#[cfg(target_os = "windows")]
use serde::Deserialize;

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
struct Win32Processor {
    Name: Option<String>,
    Manufacturer: Option<String>,
    MaxClockSpeed: Option<u32>,
    CurrentClockSpeed: Option<u32>,
}

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
struct Win32PhysicalMemory {
    Manufacturer: Option<String>,
    PartNumber: Option<String>,
    Capacity: Option<u64>,
    SMBIOSMemoryType: Option<u16>,
    MemoryType: Option<u16>,
}

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
struct Win32DiskDrive {
    DeviceID: Option<String>,
    Model: Option<String>,
    Manufacturer: Option<String>,
    InterfaceType: Option<String>,
    MediaType: Option<String>,
}

// ============================================================================
// LINUX-SPECIFIC IMPLEMENTATIONS
// ============================================================================

#[cfg(target_os = "linux")]
fn get_linux_cpu_info(cpu: &mut CpuInfo) {
    use std::fs;

    // Try to read from /proc/cpuinfo as fallback
    if cpu.model.is_none() || cpu.vendor.is_none() || cpu.frequency_mhz.is_none() {
        if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
            for line in contents.lines() {
                let line = line.trim();

                if cpu.vendor.is_none() && line.starts_with("vendor_id") {
                    if let Some(v) = line.split(':').nth(1) {
                        let v = v.trim();
                        if !v.is_empty() {
                            cpu.vendor = Some(v.to_string());
                        }
                    }
                } else if cpu.model.is_none() && line.starts_with("model name") {
                    if let Some(v) = line.split(':').nth(1) {
                        let v = v.trim();
                        if !v.is_empty() {
                            cpu.model = Some(v.to_string());
                        }
                    }
                } else if cpu.frequency_mhz.is_none() && line.starts_with("cpu MHz") {
                    if let Some(v) = line.split(':').nth(1) {
                        if let Ok(f) = v.trim().parse::<f64>() {
                            if f > 0.0 {
                                cpu.frequency_mhz = Some(f as u64);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn get_linux_ram_info(sysinfo: &mut SystemInfo) {
    use std::process::Command;
    
    let mut modules = Vec::new();
    let mut type_hist: HashMap<String, u32> = HashMap::new();

    // Try dmidecode for RAM module info
    if let Ok(out) = Command::new("dmidecode").arg("-t").arg("17").output() {
        if out.status.success() {
            if let Ok(txt) = String::from_utf8(out.stdout) {
                let mut current = MemoryModule::default();

                for l in txt.lines() {
                    let line = l.trim();

                    if line.starts_with("Memory Device") {
                        if current.memory_type.is_some() {
                            if let Some(t) = &current.memory_type {
                                *type_hist.entry(t.clone()).or_insert(0) += 1;
                            }
                            modules.push(current);
                        }
                        current = MemoryModule::default();
                    }

                    if let Some(v) = line.strip_prefix("Manufacturer:") {
                        current.vendor = Some(v.trim().to_string());
                    }
                    if let Some(v) = line.strip_prefix("Part Number:") {
                        current.part_number = Some(v.trim().to_string());
                    }
                    if let Some(v) = line.strip_prefix("Type:") {
                        let t = v.trim();
                        if t.starts_with("DDR") {
                            current.memory_type = Some(t.into());
                        }
                    }
                    if let Some(v) = line.strip_prefix("Size:") {
                        let s = v.trim();
                        if s.ends_with("GB") {
                            if let Ok(n) = s[..s.len() - 2].trim().parse::<u64>() {
                                current.size_mb = Some(n * 1024);
                            }
                        }
                        if s.ends_with("MB") {
                            if let Ok(n) = s[..s.len() - 2].trim().parse::<u64>() {
                                current.size_mb = Some(n);
                            }
                        }
                    }
                }

                // Push the last module
                if current.memory_type.is_some() {
                    if let Some(t) = &current.memory_type {
                        *type_hist.entry(t.clone()).or_insert(0) += 1;
                    }
                    modules.push(current);
                }

                // Set RAM type
                if sysinfo.ram.ram_type.is_none() {
                    if let Some((t, _)) = type_hist.into_iter().max_by_key(|(_, c)| *c) {
                        sysinfo.ram.ram_type = Some(t);
                    }
                }
            }
        }
    }

    sysinfo.ram.modules = modules;
}

#[cfg(target_os = "linux")]
fn get_linux_disk_info(disks: &mut Vec<DiskInfo>) {
    use std::fs;

    for disk in disks.iter_mut() {
        // Extract device name from path (e.g., "/dev/sda" -> "sda")
        let raw_name = disk.name.clone();
        let dev_name = raw_name
            .split(|c| c == '/' || c == '\\')
            .filter(|s| !s.is_empty())
            .last()
            .unwrap_or(&raw_name)
            .to_string();

        // For NVMe: "nvme0n1p1" -> "nvme0n1"
        let block_name = if dev_name.starts_with("nvme") {
            if let Some(p_pos) = dev_name.rfind('p') {
                dev_name[..p_pos].to_string()
            } else {
                dev_name
            }
        } else {
            dev_name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
        };

        // Read from /sys/block
        let base_path = format!("/sys/block/{}", block_name);

        // Get vendor and model
        if disk.vendor.is_none() {
            if let Ok(vendor) = fs::read_to_string(format!("{}/device/vendor", base_path)) {
                disk.vendor = Some(vendor.trim().to_string());
            }
        }

        if disk.model.is_none() {
            if let Ok(model) = fs::read_to_string(format!("{}/device/model", base_path)) {
                disk.model = Some(model.trim().to_string());
            }
        }

        // Get disk type from rotational
        if disk.disk_type.is_none() {
            if let Ok(rotational) = fs::read_to_string(format!("{}/queue/rotational", base_path)) {
                let rotational = rotational.trim();
                let base_type = if rotational == "0" {
                    "SSD"
                } else if rotational == "1" {
                    "HDD"
                } else {
                    "Unknown"
                };

                // Set disk type with bus info if available
                if let Ok(protocol) = fs::read_to_string(format!("{}/device/protocol", base_path))
                    .or_else(|_| fs::read_to_string(format!("{}/device/transport", base_path)))
                {
                    let protocol = protocol.trim();
                    if !protocol.is_empty() {
                        if protocol.to_lowercase().contains("nvme") {
                            disk.disk_type = Some("NVMe".to_string());
                        } else if base_type != "Unknown" {
                            disk.disk_type = Some(format!("{} ({})", base_type, protocol));
                        } else {
                            disk.disk_type = Some(protocol.to_string());
                        }
                    }
                } else if base_type != "Unknown" {
                    disk.disk_type = Some(base_type.to_string());
                }
            }
        }
    }
}

// ============================================================================
// MACOS-SPECIFIC IMPLEMENTATIONS
// ============================================================================

#[cfg(target_os = "macos")]
fn get_macos_disk_info(disks: &mut Vec<DiskInfo>) {
    use std::process::Command;

    if disks.is_empty() {
        // Try to get disk info from diskutil
        if let Ok(out) = Command::new("diskutil").arg("info").arg("-all").output() {
            if out.status.success() {
                if let Ok(txt) = String::from_utf8(out.stdout) {
                    let mut model = None;
                    let mut vendor = None;

                    for line in txt.lines() {
                        let l = line.trim();
                        if l.starts_with("Device Model:") {
                            model = Some(l[13..].trim().to_string());
                        }
                        if l.starts_with("Device Manufacturer:") {
                            vendor = Some(l[20..].trim().to_string());
                        }
                    }

                    // Apply to first disk if we have info
                    if !disks.is_empty() {
                        let first_disk = &mut disks[0];
                        if model.is_some() {
                            first_disk.model = model;
                        }
                        if vendor.is_some() {
                            first_disk.vendor = vendor;
                        }

                        // Determine disk type from model
                        if let Some(m) = &first_disk.model {
                            let s = m.to_lowercase();
                            if s.contains("nvme") {
                                first_disk.disk_type = Some("NVMe".into());
                            } else if s.contains("ssd") {
                                first_disk.disk_type = Some("SSD".into());
                            }
                        }
                    }
                }
            }
        }
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
    fn get_system_info_does_not_panic() {
        let sys = get_system_info();
        assert!(sys.cpus().len() > 0);
    }

    #[test]
    fn get_detailed_system_info_does_not_panic() {
        let sys_info = get_detailed_system_info();
        assert!(sys_info.cpu.cores_logical > 0);
        assert!(sys_info.ram.total_mb > 0);
    }

    #[test]
    fn get_detailed_system_info_has_expected_structure() {
        let sys_info = get_detailed_system_info();

        // Check CPU info
        assert!(sys_info.cpu.cores_logical >= 1);
        if let Some(physical) = sys_info.cpu.cores_physical {
            assert!(physical <= sys_info.cpu.cores_logical);
        }

        // Check RAM info
        assert!(sys_info.ram.total_mb > 0);
        let _ = sys_info.ram.modules.len();

        // Check disks
        let _ = sys_info.disks.len();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_map_smbios_memory_type() {
        use super::map_smbios_memory_type;
        assert_eq!(map_smbios_memory_type(20), Some("DDR"));
        assert_eq!(map_smbios_memory_type(21), Some("DDR2"));
        assert_eq!(map_smbios_memory_type(24), Some("DDR3"));
        assert_eq!(map_smbios_memory_type(26), Some("DDR4"));
        assert_eq!(map_smbios_memory_type(34), Some("DDR5"));
        assert_eq!(map_smbios_memory_type(999), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_determine_disk_type() {
        use super::determine_disk_type;
        
        assert_eq!(determine_disk_type(Some("NVMe"), None, None), Some("NVMe".to_string()));
        assert_eq!(determine_disk_type(None, Some("Samsung NVMe"), None), Some("NVMe".to_string()));
        assert_eq!(determine_disk_type(None, None, Some("SSD")), Some("SSD".to_string()));
        assert_eq!(determine_disk_type(None, Some("Crucial SSD"), None), Some("SSD".to_string()));
        assert_eq!(determine_disk_type(None, None, Some("HDD")), Some("HDD".to_string()));
        assert_eq!(determine_disk_type(None, None, Some("Fixed hard disk media")), Some("HDD".to_string()));
        assert_eq!(determine_disk_type(Some("SATA"), None, None), None);
    }
}
