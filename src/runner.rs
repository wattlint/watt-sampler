use crate::gpu::{GpuInfo, GpuTracker};
use crate::normalizer;
use crate::rapl::RaplReader;
use nix::sched::{sched_setaffinity, CpuSet};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL_INTERVAL: Duration = Duration::from_millis(100);

// ── Output Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub schema_version: String,
    pub timestamp_utc: String,
    pub host: HostInfo,
    pub benchmark: BenchmarkInfo,
    pub measurements: Vec<Measurement>,
    pub summary: Summary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostInfo {
    pub cpu_vendor: String,
    pub cpu_model: String,
    pub cpu_tdp_watts: Option<f64>,
    pub cpu_sockets: u32,
    pub rapl_method: String,
    pub gpu: Vec<GpuInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkInfo {
    pub command: String,
    pub iterations_requested: u32,
    pub warmup_iters: u32,
    pub domains_measured: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Measurement {
    pub iteration: u32,
    pub duration_s: f64,
    pub energy: Energy,
    pub power: Power,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Energy {
    pub pkg_joules: Option<f64>,
    pub pp0_joules: Option<f64>,
    pub dram_joules: Option<f64>,
    pub psys_joules: Option<f64>,
    pub gpu_joules: Option<f64>,
    pub total_joules: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Power {
    pub pkg_avg_watts: Option<f64>,
    pub gpu_avg_watts: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub median_total_joules: f64,
    pub p5_total_joules: f64,
    pub p95_total_joules: f64,
    pub median_pkg_watts: Option<f64>,
    pub ees: Option<f64>,
    pub noise_pct: f64,
    pub valid: bool,
    pub warnings: Vec<String>,
}

pub struct RunConfig {
    pub iterations: u32,
    pub warmup: u32,
    pub domains: Vec<String>,
    pub no_affinity: bool,
    pub force: bool,
}

// ── Entry Point ───────────────────────────────────────────────

pub fn run(command: &[String], config: &RunConfig) -> anyhow::Result<RunResult> {
    let available = crate::rapl::available_domains(&config.domains);
    let gpu_requested = config.domains.iter().any(|d| d == "gpu");
    let host = build_host_info(&available, gpu_requested);

    let total_iters = config.warmup + config.iterations;
    let mut measurements = Vec::with_capacity(config.iterations as usize);
    let mut warnings = Vec::new();

    if let Ok(gov) =
        std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    {
        if gov.trim() != "performance" {
            warnings.push(format!(
                "CPU governor is '{}', expected 'performance'",
                gov.trim()
            ));
        }
    }

    for i in 0..total_iters {
        let mut m = run_single(command, &available, gpu_requested, config)?;
        m.iteration = i.saturating_sub(config.warmup) + 1;

        if i >= config.warmup {
            if let (Some(w), Some(tdp)) = (m.power.pkg_avg_watts, host.cpu_tdp_watts) {
                if w > tdp * 0.95 {
                    warnings.push("Thermal throttling likely (avg power > 95% TDP)".into());
                }
            }
            measurements.push(m);
        }
    }

    let summary = build_summary(&measurements, &host, &warnings, config.force);

    Ok(RunResult {
        schema_version: "1".into(),
        timestamp_utc: iso8601_now(),
        host,
        benchmark: BenchmarkInfo {
            command: command.join(" "),
            iterations_requested: config.iterations,
            warmup_iters: config.warmup,
            domains_measured: config.domains.clone(),
        },
        measurements,
        summary,
    })
}

// ── Single Iteration ──────────────────────────────────────────

fn run_single(
    command: &[String],
    available: &[String],
    gpu_requested: bool,
    config: &RunConfig,
) -> anyhow::Result<Measurement> {
    let mut rapl_readers: Vec<(String, RaplReader)> = available
        .iter()
        .filter_map(|d| RaplReader::new(d).ok().map(|r| (d.clone(), r)))
        .collect();

    let mut gpu_tracker = if gpu_requested {
        GpuTracker::new()
    } else {
        None
    };

    for (_, reader) in &mut rapl_readers {
        let _ = reader.update();
    }

    let start = Instant::now();
    let child = Command::new(&command[0])
        .args(&command[1..])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let child_pid = child.id();
    if !config.no_affinity {
        pin_cpu(child_pid);
    }

    let child_done = Arc::new(AtomicBool::new(false));
    let rapl_done = child_done.clone();
    let gpu_done = child_done.clone();

    let rapl_handle = thread::spawn(move || {
        while !rapl_done.load(Ordering::Relaxed) {
            for (_, reader) in &mut rapl_readers {
                let _ = reader.update();
            }
            thread::sleep(POLL_INTERVAL);
        }
        rapl_readers
    });

    let gpu_handle = thread::spawn(move || {
        while !gpu_done.load(Ordering::Relaxed) {
            if let Some(ref mut t) = gpu_tracker {
                let _ = t.sample();
            }
            thread::sleep(POLL_INTERVAL);
        }
        gpu_tracker
    });

    let mut child = child;
    child.wait()?;
    child_done.store(true, Ordering::Relaxed);

    let duration = start.elapsed().as_secs_f64();
    let rapl_readers = rapl_handle.join().unwrap_or_default();
    let gpu_tracker = gpu_handle.join().unwrap_or(None);

    let mut pkg_j = None;
    let mut pp0_j = None;
    let mut dram_j = None;
    let mut psys_j = None;

    for (name, reader) in &rapl_readers {
        let j = reader.total_joules();
        match name.as_str() {
            "pkg" => pkg_j = Some(j),
            "pp0" => pp0_j = Some(j),
            "dram" => dram_j = Some(j),
            "psys" => psys_j = Some(j),
            _ => {}
        }
    }

    let poll_interval_ms = POLL_INTERVAL.as_secs_f64() * 1000.0;
    let gpu_j = gpu_tracker
        .as_ref()
        .and_then(|t| t.total_joules(poll_interval_ms));
    let total_j = pkg_j.unwrap_or(0.0) + gpu_j.unwrap_or(0.0);
    let pkg_avg = if duration > 0.0 {
        pkg_j.map(|j| j / duration)
    } else {
        None
    };
    let gpu_avg = gpu_tracker.as_ref().and_then(|t| t.avg_watts());

    Ok(Measurement {
        iteration: 0,
        duration_s: duration,
        energy: Energy {
            pkg_joules: pkg_j,
            pp0_joules: pp0_j,
            dram_joules: dram_j,
            psys_joules: psys_j,
            gpu_joules: gpu_j,
            total_joules: total_j,
        },
        power: Power {
            pkg_avg_watts: pkg_avg,
            gpu_avg_watts: gpu_avg,
        },
    })
}

// ── CPU Affinity ──────────────────────────────────────────────

fn pin_cpu(pid: u32) {
    let num_cpus = num_cpus();
    let target = if num_cpus > 2 { num_cpus / 2 } else { 0 };

    let mut set = CpuSet::new();
    if set.set(target).is_ok() {
        let _ = sched_setaffinity(Pid::from_raw(pid as i32), &set);
    }
}

fn num_cpus() -> usize {
    let sys = sysinfo::System::new_all();
    sys.cpus().len()
}

// ── Summary ───────────────────────────────────────────────────

fn build_summary(
    measurements: &[Measurement],
    host: &HostInfo,
    warnings: &[String],
    force: bool,
) -> Summary {
    let mut totals: Vec<f64> = measurements.iter().map(|m| m.energy.total_joules).collect();
    totals.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut pkg_watts: Vec<f64> = measurements
        .iter()
        .filter_map(|m| m.power.pkg_avg_watts)
        .collect();
    pkg_watts.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median_total = normalizer::median(&totals);
    let p5 = normalizer::percentile(&totals, 5);
    let p95 = normalizer::percentile(&totals, 95);
    let noise = normalizer::noise_pct(median_total, p5, p95);

    let median_pkg = if pkg_watts.is_empty() {
        None
    } else {
        Some(normalizer::median(&pkg_watts))
    };

    let mut durations: Vec<f64> = measurements.iter().map(|m| m.duration_s).collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_duration = normalizer::median(&durations);

    let ees = host
        .cpu_tdp_watts
        .and_then(|tdp| normalizer::compute_ees(median_total, tdp, median_duration));

    let mut out_warnings = warnings.to_vec();
    if noise > 15.0 {
        out_warnings.push(format!(
            "High measurement noise: {:.1}% (threshold: 15%)",
            noise
        ));
    }

    let valid = noise <= 15.0;
    if noise > 20.0 && !force {
        eprintln!(
            "Warning: noise {:.1}% exceeds 20%. Use --force to accept.",
            noise
        );
    }

    Summary {
        median_total_joules: median_total,
        p5_total_joules: p5,
        p95_total_joules: p95,
        median_pkg_watts: median_pkg,
        ees,
        noise_pct: noise,
        valid,
        warnings: out_warnings,
    }
}

// ── Host Info ─────────────────────────────────────────────────

fn build_host_info(available: &[String], gpu_requested: bool) -> HostInfo {
    let sys = sysinfo::System::new_all();

    let cpu_vendor = sys
        .cpus()
        .first()
        .map(|c| {
            let brand = c.brand().to_lowercase();
            if brand.contains("intel") {
                "GenuineIntel".into()
            } else if brand.contains("amd") || brand.contains("ryzen") {
                "AuthenticAMD".into()
            } else {
                "Unknown".into()
            }
        })
        .unwrap_or_else(|| "Unknown".into());

    let cpu_model = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".into());

    HostInfo {
        cpu_vendor,
        cpu_model,
        cpu_tdp_watts: normalizer::detect_tdp_watts(),
        cpu_sockets: sys.physical_core_count().unwrap_or(1) as u32,
        rapl_method: if available.is_empty() {
            "estimated".into()
        } else {
            crate::rapl::detect_method().into()
        },
        gpu: if gpu_requested {
            crate::gpu::detect_gpus()
        } else {
            Vec::new()
        },
    }
}

// ── Timestamp ─────────────────────────────────────────────────

fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let total_days = (secs / 86400) as i64;
    let seconds_in_day = secs % 86400;
    let hour = seconds_in_day / 3600;
    let minute = (seconds_in_day % 3600) / 60;
    let second = seconds_in_day % 60;

    let mut year = 1970i64;
    let mut remaining = total_days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let leap = is_leap(year);
    let month_days = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0usize;
    let mut day_of_year = remaining;
    for &d in &month_days {
        if day_of_year < d as i64 {
            break;
        }
        day_of_year -= d as i64;
        month += 1;
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month + 1,
        day_of_year + 1,
        hour,
        minute,
        second,
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
