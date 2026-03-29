use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub tdp_watts: Option<u32>,
}

/// GPU energy tracker — samples power and accumulates energy.
pub struct GpuTracker {
    source: GpuSource,
    samples: Vec<f64>, // milliwatts
}

enum GpuSource {
    Nvidia {
        nvml: Box<nvml_wrapper::Nvml>,
        device_count: u32,
    },
    Amd {
        hwmon_paths: Vec<String>,
    },
}

impl GpuTracker {
    pub fn new() -> Option<Self> {
        if let Some(tracker) = Self::try_nvidia() {
            return Some(tracker);
        }
        if let Some(tracker) = Self::try_amd() {
            return Some(tracker);
        }
        None
    }

    fn try_nvidia() -> Option<Self> {
        let nvml = nvml_wrapper::Nvml::init().ok()?;
        let device_count = nvml.device_count().ok()?;
        if device_count == 0 {
            return None;
        }
        Some(Self {
            source: GpuSource::Nvidia {
                nvml: Box::new(nvml),
                device_count,
            },
            samples: Vec::new(),
        })
    }

    fn try_amd() -> Option<Self> {
        let paths = find_amd_hwmon_power_paths();
        if paths.is_empty() {
            return None;
        }
        Some(Self {
            source: GpuSource::Amd { hwmon_paths: paths },
            samples: Vec::new(),
        })
    }

    /// Sample current GPU power draw in milliwatts and store it.
    pub fn sample(&mut self) -> Result<(), String> {
        let mw = match &self.source {
            GpuSource::Nvidia { nvml, device_count } => {
                let mut total = 0u32;
                for i in 0..*device_count {
                    let device = nvml.device_by_index(i).map_err(|e| e.to_string())?;
                    total += device.power_usage().map_err(|e| e.to_string())?;
                }
                total as f64
            }
            GpuSource::Amd { hwmon_paths } => {
                let mut total = 0.0f64;
                for path in hwmon_paths {
                    if let Ok(val) = fs::read_to_string(path) {
                        if let Ok(uw) = val.trim().parse::<f64>() {
                            total += uw / 1000.0; // microwatts -> milliwatts
                        }
                    }
                }
                total
            }
        };
        self.samples.push(mw);
        Ok(())
    }

    /// Compute total energy in joules from accumulated samples.
    /// Assumes ~100ms between samples (matching runner poll interval).
    pub fn total_joules(&self, poll_interval_ms: f64) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        let avg_mw: f64 = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
        let total_ms = self.samples.len() as f64 * poll_interval_ms;
        Some(avg_mw * total_ms / 1_000_000.0)
    }

    /// Average power in watts over all samples.
    pub fn avg_watts(&self) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        Some(self.samples.iter().sum::<f64>() / self.samples.len() as f64 / 1000.0)
    }
}

/// Detect available GPUs and return their info.
pub fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(count) = nvml.device_count() {
            for i in 0..count {
                if let Ok(device) = nvml.device_by_index(i) {
                    let name = device
                        .name()
                        .unwrap_or_else(|_| format!("NVIDIA GPU {}", i));
                    gpus.push(GpuInfo {
                        index: i,
                        name,
                        tdp_watts: None,
                    });
                }
            }
        }
    }

    if gpus.is_empty() {
        let hwmon_paths = find_amd_hwmon_power_paths();
        if !hwmon_paths.is_empty() {
            gpus.push(GpuInfo {
                index: 0,
                name: "AMD GPU".to_string(),
                tdp_watts: None,
            });
        }
    }

    gpus
}

fn find_amd_hwmon_power_paths() -> Vec<String> {
    let drm = Path::new("/sys/class/drm");
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(drm) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("card") || name_str.contains('-') {
                continue;
            }

            let hwmon_dir = entry.path().join("device/hwmon");
            if let Ok(hwmon_entries) = fs::read_dir(&hwmon_dir) {
                for hwmon in hwmon_entries.filter_map(|e| e.ok()) {
                    let power = hwmon.path().join("power1_average");
                    if power.exists() {
                        // Verify it's AMD (vendor check)
                        let vendor_path = entry.path().join("device/vendor");
                        let is_amd = fs::read_to_string(&vendor_path)
                            .map(|v| v.trim() == "0x1002")
                            .unwrap_or(false);
                        if is_amd {
                            paths.push(power.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    paths.sort();
    paths
}
