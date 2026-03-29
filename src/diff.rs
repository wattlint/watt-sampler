use crate::runner::{Measurement, RunResult};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct DiffResult {
    pub baseline_joules: f64,
    pub current_joules: f64,
    pub delta_joules: f64,
    pub delta_pct: f64,
    pub threshold_pct: f64,
    pub regression: bool,
    pub domains: HashMap<String, DomainDelta>,
}

#[derive(Debug, Serialize)]
pub struct DomainDelta {
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub delta_pct: f64,
}

/// Compare baseline and current measurements.
pub fn diff(baseline: &RunResult, current: &RunResult, threshold_pct: f64) -> DiffResult {
    let baseline_j = baseline.summary.median_total_joules;
    let current_j = current.summary.median_total_joules;
    let delta_j = current_j - baseline_j;
    let delta_pct = safe_pct(delta_j, baseline_j);

    let mut domains = HashMap::new();

    type Extractor = fn(&Measurement) -> Option<f64>;
    let domain_extractors: &[(&str, Extractor)] = &[
        ("pkg", |m| m.energy.pkg_joules),
        ("pp0", |m| m.energy.pp0_joules),
        ("dram", |m| m.energy.dram_joules),
        ("psys", |m| m.energy.psys_joules),
        ("gpu", |m| m.energy.gpu_joules),
    ];

    for &(name, extractor) in domain_extractors {
        let b_val = median_of(&baseline.measurements, extractor);
        let c_val = median_of(&current.measurements, extractor);
        if let (Some(bv), Some(cv)) = (b_val, c_val) {
            domains.insert(
                name.to_string(),
                DomainDelta {
                    baseline: bv,
                    current: cv,
                    delta: cv - bv,
                    delta_pct: safe_pct(cv - bv, bv),
                },
            );
        }
    }

    DiffResult {
        baseline_joules: baseline_j,
        current_joules: current_j,
        delta_joules: delta_j,
        delta_pct,
        threshold_pct,
        regression: delta_pct > threshold_pct,
        domains,
    }
}

fn median_of(measurements: &[Measurement], f: fn(&Measurement) -> Option<f64>) -> Option<f64> {
    let mut vals: Vec<f64> = measurements.iter().filter_map(f).collect();
    if vals.is_empty() {
        return None;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(crate::normalizer::median(&vals))
}

fn safe_pct(delta: f64, base: f64) -> f64 {
    if base == 0.0 {
        0.0
    } else {
        delta / base * 100.0
    }
}
