mod amd_rapl;
mod diff;
mod gpu;
mod normalizer;
mod rapl;
mod report;
mod runner;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(
    name = "watt-sampler",
    version,
    about = "Energy measurement CLI using hardware performance counters"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a benchmark and measure energy consumption
    Run {
        /// Number of measurement iterations
        #[arg(long, default_value_t = 7)]
        iterations: u32,

        /// Warmup iterations to discard
        #[arg(long, default_value_t = 2)]
        warmup: u32,

        /// RAPL domains to measure (comma-separated)
        #[arg(long, default_value = "pkg,gpu")]
        domains: String,

        /// Output format
        #[arg(long, default_value = "json")]
        output: String,

        /// Skip CPU affinity pinning (for VMs)
        #[arg(long)]
        no_affinity: bool,

        /// Continue even if noise > 20%
        #[arg(long)]
        force: bool,

        /// Command to benchmark
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Compare two measurement JSON files
    Diff {
        /// Baseline measurement JSON
        #[arg(long)]
        baseline: String,

        /// Current measurement JSON
        #[arg(long)]
        current: String,

        /// Regression threshold percentage
        #[arg(long, default_value_t = 10.0)]
        threshold: f64,

        /// Output format: human, json, markdown
        #[arg(long, default_value = "human")]
        output: String,

        /// Exit 1 if regression detected
        #[arg(long)]
        fail: bool,
    },

    /// Detect hardware capabilities
    CheckHardware,
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Run {
            iterations,
            warmup,
            domains,
            output,
            no_affinity,
            force,
            command,
        } => cmd_run(
            iterations,
            warmup,
            &domains,
            &output,
            no_affinity,
            force,
            &command,
        ),
        Commands::Diff {
            baseline,
            current,
            threshold,
            output,
            fail,
        } => cmd_diff(&baseline, &current, threshold, &output, fail),
        Commands::CheckHardware => cmd_check_hardware(),
    };

    process::exit(exit_code);
}

fn cmd_run(
    iterations: u32,
    warmup: u32,
    domains_str: &str,
    output: &str,
    no_affinity: bool,
    force: bool,
    command: &[String],
) -> i32 {
    if command.is_empty() {
        eprintln!("Error: no command specified. Usage: watt-sampler run -- <COMMAND> [ARGS...]");
        return 2;
    }

    let domains: Vec<String> = domains_str.split(',').map(String::from).collect();

    let config = runner::RunConfig {
        iterations,
        warmup,
        domains,
        no_affinity,
        force,
    };

    match runner::run(command, &config) {
        Ok(result) => {
            let valid = result.summary.valid;

            match output {
                "human" => print_human(&result),
                _ => {
                    let json = report::to_json(&result);
                    if !json.is_empty() {
                        println!("{}", json);
                    }
                }
            }

            if !valid && !force {
                1
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            2
        }
    }
}

fn cmd_diff(
    baseline_path: &str,
    current_path: &str,
    threshold: f64,
    output: &str,
    fail: bool,
) -> i32 {
    let baseline: runner::RunResult = match read_json(baseline_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading baseline: {}", e);
            return 2;
        }
    };

    let current: runner::RunResult = match read_json(current_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading current: {}", e);
            return 2;
        }
    };

    let result = diff::diff(&baseline, &current, threshold);

    match output {
        "json" => println!("{}", report::diff_to_json(&result)),
        "markdown" => print!("{}", report::diff_to_markdown(&result)),
        _ => print!("{}", report::diff_to_human(&result)),
    }

    if fail && result.regression {
        1
    } else {
        0
    }
}

fn cmd_check_hardware() -> i32 {
    let rapl_method = rapl::detect_method();
    let available = rapl::available_domains(
        &rapl::DOMAINS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );
    let gpus = gpu::detect_gpus();

    let info = serde_json::json!({
        "rapl_method": rapl_method,
        "rapl_domains": available,
        "gpu": gpus,
        "estimated": rapl_method == "estimated" && gpus.is_empty(),
    });

    println!("{}", serde_json::to_string_pretty(&info).unwrap());

    if rapl_method == "estimated" && gpus.is_empty() {
        2
    } else {
        0
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

fn print_human(result: &runner::RunResult) {
    let s = &result.summary;
    println!("\nWattLint Sampler Results");
    println!("───────────────────────");
    println!("Command:   {}", result.benchmark.command);
    println!(
        "Iterations: {} (+ {} warmup)",
        result.benchmark.iterations_requested, result.benchmark.warmup_iters
    );
    println!("RAPL:      {}", result.host.rapl_method);
    println!();
    println!("Median total energy: {:.1} J", s.median_total_joules);
    println!(
        "P5–P95 range:        {:.1}–{:.1} J",
        s.p5_total_joules, s.p95_total_joules
    );
    if let Some(pkg) = s.median_pkg_watts {
        println!("Median CPU power:    {:.1} W", pkg);
    }
    if let Some(ees) = s.ees {
        println!("EES:                 {:.2}", ees);
    }
    println!("Noise:               {:.1}%", s.noise_pct);
    println!("Valid:               {}", s.valid);

    if !s.warnings.is_empty() {
        println!("\nWarnings:");
        for w in &s.warnings {
            println!("  ⚠ {}", w);
        }
    }
}
