use std::fs;
use std::path::Path;

const AMD_ENERGY_DIR: &str = "/sys/devices/platform/amd_energy";

/// Check if AMD RAPL is available on this system.
pub fn available() -> bool {
    is_amd_cpu() && Path::new(AMD_ENERGY_DIR).exists()
}

/// Find sysfs path for a RAPL domain on AMD.
pub fn find_path(domain: &str) -> Option<String> {
    if !is_amd_cpu() {
        return None;
    }
    let entries = read_sorted_energy_files()?;
    match domain {
        "pkg" => entries.first().cloned(),
        "dram" => entries.get(1).cloned(),
        _ => None,
    }
}

fn is_amd_cpu() -> bool {
    fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.contains("AuthenticAMD"))
        .unwrap_or(false)
}

fn read_sorted_energy_files() -> Option<Vec<String>> {
    let dir = Path::new(AMD_ENERGY_DIR);
    let mut files: Vec<String> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("energy") && name.ends_with("_input") {
                Some(e.path().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    files.sort();
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}
