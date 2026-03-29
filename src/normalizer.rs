use std::fs;

/// Energy Efficiency Score: dimensionless, cross-machine comparable within CPU vendor.
/// EES = measured_joules / (cpu_tdp_watts * duration_s)
pub fn compute_ees(joules: f64, tdp_watts: f64, duration_s: f64) -> Option<f64> {
    if tdp_watts <= 0.0 || duration_s <= 0.0 || joules < 0.0 {
        return None;
    }
    Some(joules / (tdp_watts * duration_s))
}

/// Detect CPU TDP in watts from sysfs powercap.
pub fn detect_tdp_watts() -> Option<f64> {
    let paths = [
        "/sys/devices/virtual/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw",
        "/sys/class/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw",
    ];

    for path in paths {
        if let Ok(val) = fs::read_to_string(path) {
            if let Ok(uw) = val.trim().parse::<f64>() {
                return Some(uw / 1_000_000.0);
            }
        }
    }
    None
}

/// Compute median of a sorted slice.
pub fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// Compute percentile (0-100) from sorted data.
pub fn percentile(sorted: &[f64], pct: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (sorted.len() * pct / 100).min(sorted.len() - 1);
    sorted[idx]
}

/// Noise percentage: (p95 - p5) / median * 100.
pub fn noise_pct(median: f64, p5: f64, p95: f64) -> f64 {
    if median <= 0.0 {
        return 0.0;
    }
    (p95 - p5) / median * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ees_computation() {
        // 1000J at 150W TDP for 10s -> EES = 1000 / (150 * 10) = 0.667
        let ees = compute_ees(1000.0, 150.0, 10.0).unwrap();
        assert!((ees - 0.6667).abs() < 0.001);
    }

    #[test]
    fn test_ees_invalid_inputs() {
        assert!(compute_ees(100.0, 0.0, 10.0).is_none());
        assert!(compute_ees(100.0, 150.0, 0.0).is_none());
        assert!(compute_ees(-1.0, 150.0, 10.0).is_none());
    }

    #[test]
    fn test_median_odd() {
        let mut v = vec![100.0, 102.0, 98.0, 103.0, 99.0, 101.0, 105.0];
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(median(&v), 101.0);
    }

    #[test]
    fn test_median_even() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(median(&v), 2.5);
    }
}
