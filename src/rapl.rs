use anyhow::Result;
use std::fs;
use std::path::Path;

pub const DOMAINS: &[&str] = &["pkg", "pp0", "dram", "psys"];

// Intel RAPL sysfs paths (powercap)
pub const POWERCAP_SYSFS: &[(&str, &str)] = &[
    (
        "pkg",
        "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj",
    ),
    (
        "pp0",
        "/sys/class/powercap/intel-rapl/intel-rapl:0:0/energy_uj",
    ),
    (
        "dram",
        "/sys/class/powercap/intel-rapl/intel-rapl:0:2/energy_uj",
    ),
    (
        "psys",
        "/sys/class/powercap/intel-rapl/intel-rapl:1/energy_uj",
    ),
];

// ── Accumulator ───────────────────────────────────────────────

/// Monotonic accumulator for 32-bit RAPL energy counters.
/// RAPL registers overflow every ~52 min at 84W.
pub struct RaplAccumulator {
    last_raw: u64,
    pub cumulative_uj: u128,
}

impl RaplAccumulator {
    pub fn new(initial_raw: u64) -> Self {
        Self {
            last_raw: initial_raw,
            cumulative_uj: 0,
        }
    }

    pub fn update(&mut self, new_raw: u64) {
        if new_raw < self.last_raw {
            self.cumulative_uj += (u32::MAX as u128 - self.last_raw as u128) + new_raw as u128 + 1;
        } else {
            self.cumulative_uj += (new_raw - self.last_raw) as u128;
        }
        self.last_raw = new_raw;
    }

    pub fn total_joules(&self) -> f64 {
        self.cumulative_uj as f64 / 1_000_000.0
    }
}

// ── Reader ────────────────────────────────────────────────────

/// Per-domain RAPL reader. Reads from sysfs energy_uj files.
pub struct RaplReader {
    path: String,
    accumulator: RaplAccumulator,
}

impl RaplReader {
    pub fn new(domain: &str) -> Result<Self> {
        let path = find_energy_path(domain)
            .ok_or_else(|| anyhow::anyhow!("No RAPL source for domain '{}'", domain))?;
        let initial = Self::read_uj(&path)?;
        Ok(Self {
            path,
            accumulator: RaplAccumulator::new(initial),
        })
    }

    #[allow(dead_code)]
    pub fn method(&self) -> &str {
        if self.path.contains("amd_energy") {
            "amd_energy"
        } else {
            "powercap"
        }
    }

    fn read_uj(path: &str) -> Result<u64> {
        Ok(fs::read_to_string(path)?.trim().parse::<u64>()?)
    }

    pub fn update(&mut self) -> Result<()> {
        let raw = Self::read_uj(&self.path)?;
        self.accumulator.update(raw);
        Ok(())
    }

    pub fn total_joules(&self) -> f64 {
        self.accumulator.total_joules()
    }
}

// ── Discovery ─────────────────────────────────────────────────

/// Find the sysfs energy path for a domain, trying powercap then AMD.
fn find_energy_path(domain: &str) -> Option<String> {
    // Intel powercap
    if let Some((_, path)) = POWERCAP_SYSFS.iter().find(|(d, _)| *d == domain) {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // AMD sysfs
    if let Some(path) = crate::amd_rapl::find_path(domain) {
        return Some(path);
    }

    None
}

pub fn available_domains(requested: &[String]) -> Vec<String> {
    requested
        .iter()
        .filter(|d| find_energy_path(d).is_some())
        .cloned()
        .collect()
}

pub fn detect_method() -> &'static str {
    for (_, path) in POWERCAP_SYSFS {
        if Path::new(path).exists() {
            return "powercap";
        }
    }
    if crate::amd_rapl::available() {
        return "amd_energy";
    }
    "estimated"
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overflow_detection() {
        let mut acc = RaplAccumulator::new(u32::MAX as u64 - 100);
        acc.update(50);
        assert_eq!(acc.cumulative_uj, 151);
    }

    #[test]
    fn test_no_overflow() {
        let mut acc = RaplAccumulator::new(1000);
        acc.update(2000);
        assert_eq!(acc.cumulative_uj, 1000);
    }

    #[test]
    fn test_exact_overflow_boundary() {
        let mut acc = RaplAccumulator::new(u32::MAX as u64);
        acc.update(0);
        assert_eq!(acc.cumulative_uj, 1);
    }

    #[test]
    fn test_joules_conversion() {
        let mut acc = RaplAccumulator::new(0);
        acc.update(1_500_000);
        assert!((acc.total_joules() - 1.5).abs() < 1e-9);
    }
}
