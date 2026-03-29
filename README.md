# watt-sampler

Measure CPU and GPU energy consumption of any command using hardware performance counters.

## What it does

`watt-sampler` runs your benchmark, reads Intel RAPL and NVIDIA NVML energy counters, and outputs a JSON report with per-domain energy, power stats, and an Energy Efficiency Score.

```bash
watt-sampler run -- ./my-benchmark
```

```json
{
  "schema_version": "1",
  "host": { "cpu_vendor": "GenuineIntel", "rapl_method": "powercap" },
  "summary": {
    "median_total_joules": 2281.4,
    "ees": 0.87,
    "noise_pct": 1.4,
    "valid": true
  }
}
```

## Commands

```
watt-sampler run [OPTIONS] -- <COMMAND> [ARGS...]
  --iterations <N>       Measurement iterations [default: 7]
  --warmup <N>           Warmup iterations to discard [default: 2]
  --domains <DOMAINS>    RAPL domains: pkg,pp0,dram,psys,gpu [default: pkg,gpu]
  --output <FORMAT>      json | human [default: json]
  --no-affinity          Skip CPU affinity pinning
  --force                Continue even if noise > 20%

watt-sampler diff --baseline <FILE> --current <FILE> [OPTIONS]
  --threshold <PCT>      Regression threshold percent [default: 10]
  --output <FORMAT>      human | json | markdown [default: human]
  --fail                 Exit 1 if regression detected

watt-sampler check-hardware
```

## How it works

1. Discovers RAPL energy domains via `/sys/class/powercap/intel-rapl/` (Intel) or `/sys/devices/platform/amd_energy/` (AMD)
2. Spawns your command, pins it to a middle CPU core to reduce OS jitter
3. Polls energy counters every 100ms to detect 32-bit counter overflow
4. Runs 2 warmup + 7 measurement iterations, reports median
5. Computes Energy Efficiency Score: `joules / (TDP_watts × duration_s)`

## Requirements

- Linux (x86_64 or aarch64)
- Intel CPU with RAPL (Sandy Bridge+) or AMD CPU with Zen+ (Ryzen 2000+)
- `perf_event_paranoid ≤ 2` for non-root access, or root for MSR fallback
- Optional: NVIDIA GPU with NVML driver for GPU energy measurement

## Install

Download from [GitHub Releases](https://github.com/wattlint/watt-sampler/releases) or build from source:

```bash
cargo build --release
./target/release/watt-sampler check-hardware
```

## License

MIT
