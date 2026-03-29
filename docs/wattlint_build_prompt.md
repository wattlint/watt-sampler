# WattLint — Engineering Build Prompt
## Feed this directly to Claude Code or any AI coding agent

---

## BEFORE YOU WRITE A SINGLE LINE OF CODE — READ THIS

You have three repos already created at github.com/wattlint:

| Repo | Visibility | Start when |
|---|---|---|
| `wattlint/watt-sampler` | Public | **Day 1 — start here** |
| `wattlint/platform` | Private | After watt-sampler binary is released |
| `wattlint/gpu-tracer` | Public | Phase 2 only — do not touch yet |

**You always open `wattlint/watt-sampler` first.** The GitHub Action inside `platform` downloads the `watt-sampler` binary from its GitHub Releases. The API stores its JSON output. The dashboard displays that data. If `watt-sampler` does not exist as a working compiled binary, nothing else can be built, run, or tested. Do not open `wattlint/platform` until `watt-sampler` can produce a valid JSON measurement on a real Linux machine.

**The build order is:**

```
1. wattlint/watt-sampler          Build Rust CLI. Release binary. Test on real Linux.
        ↓  (only after binary is released to GitHub Releases)
2. wattlint/platform              In this order inside this repo:
   2a. actions/energy-gate/         TypeScript GitHub Action
   2b. apps/api/                    Go API server
   2c. apps/dashboard/              Next.js dashboard
   2d. apps/docs/                   Docusaurus docs (last)
        ↓  (Phase 2, months later)
3. wattlint/gpu-tracer            Rust GPU tracer — do not start yet
```

Do not proceed to the next step until the current step is fully working and tested. The dependency chain is strict.

---



---

## THE PRODUCT IN ONE SENTENCE

A GitHub Action runs your benchmark suite on every PR, measures CPU and GPU energy using hardware counters, compares it to the baseline from `main`, and posts a detailed comment blocking the merge if energy increased beyond a configurable threshold.

---

## STRICT TECHNOLOGY STACK — NO DEVIATIONS WITHOUT A WRITTEN REASON

| Component | Technology | Non-negotiable reason |
|---|---|---|
| Energy sampler CLI | **Rust stable 1.78+** | `perf_event_open` via `perf-event` crate is the fastest, lowest-variance RAPL mechanism (confirmed by arxiv:2401.15985). No GC pauses contaminating measurements. Single static binary. Memory safety eliminates RAPL overflow bugs. |
| GitHub Action | **TypeScript, Node 20, strict mode** | GitHub Actions is TypeScript-native. `@actions/core`, `@actions/cache`, `@actions/tool-cache` are first-class. Zero runtime install for users. |
| SaaS API | **Go 1.23** | Needs persistent process for pgx connection pooling to Neon. Vercel serverless would exhaust Neon connections. Fly.io runs an actual VM — the pgx pool stays warm. |
| Database | **Neon PostgreSQL 16** | Standard PostgreSQL with composite indexes. Replace TimescaleDB hypertables with a regular table + time index. Replace continuous aggregates with a Go cron aggregator. Migrate to TimescaleDB only if rows exceed 50M — not now. |
| Cache | **Upstash Redis** | `redis/go-redis/v9` has identical API to self-hosted Redis. Serverless, pay-per-request. Free tier covers all MVP usage. |
| Dashboard | **Next.js 15 + TypeScript** | App Router + server components. Recharts for charts. |
| API hosting | **Fly.io** | Persistent VM. pgx connection pool. ~$1.94/month. |
| Dashboard hosting | **Vercel free tier** | Natural home for Next.js 15. Zero cost. |
| Monorepo | **Turborepo + pnpm workspaces** | Parallel builds. Only rebuilds what changed. |

**You must never:**
- Call `nvidia-smi` as a subprocess. Use NVML directly via `nvml-wrapper` crate.
- Read RAPL via MSR as the primary method. Use `perf_event_open` first.
- Skip RAPL overflow handling. It is mandatory. The counter is 32-bit.
- Compare raw joules across machines. Use EES (normalised by TDP × duration).
- Deploy the Go API to Vercel. It must go to Fly.io.
- Use Python anywhere. No scripts, no tooling, nothing.
- Run benchmarks without warmup iterations.
- Report a single-run measurement as a result.
- Post duplicate PR comments. Always search-and-update.
- Skip the SHA256 checksum on binary downloads.

---

## REPOSITORY LAYOUT

Three repos. Each directory below is labelled with which GitHub repo it lives in.

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
REPO 1: github.com/wattlint/watt-sampler  (public — start here)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Cargo.toml
src/
├── main.rs         CLI entrypoint (clap): run / diff / check-hardware
├── rapl.rs         RAPL reading via perf_event_open → powercap → AMD → MSR
├── amd_rapl.rs     AMD /sys/devices/platform/amd_energy/ path
├── gpu.rs          NVML via nvml-wrapper (NEVER nvidia-smi subprocess)
├── normalizer.rs   EES = joules / (tdp_watts × duration_s)
├── runner.rs       spawn subprocess, poll RAPL 100ms, 7 iterations
├── report.rs       JSON to stdout; all warnings/errors to stderr
└── diff.rs         compare baseline vs current, threshold check

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
REPO 2: github.com/wattlint/platform  (private — start after REPO 1 is released)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
actions/
└── energy-gate/                 ← BUILD FIRST inside this repo (Step 2A)
    ├── action.yml
    ├── package.json
    └── src/
        ├── index.ts             Action entrypoint
        ├── installer.ts         Download + SHA256 verify watt-sampler binary
        ├── baseline.ts          @actions/cache: get or build main-branch baseline
        ├── runner.ts            Execute watt-sampler, parse JSON output
        ├── reporter.ts          Build PR comment markdown
        └── uploader.ts          POST measurement JSON to apps/api (optional SaaS)

apps/
├── api/                         ← BUILD SECOND inside this repo (Step 2B)
│   ├── Dockerfile
│   ├── go.mod
│   ├── cmd/api/main.go
│   └── internal/
│       ├── api/
│       │   ├── measurements.go  POST /api/v1/measurements
│       │   ├── repos.go         repo registration + baseline queries
│       │   ├── billing.go       Stripe webhook + plan enforcement
│       │   └── middleware.go    auth, rate-limit (Upstash), request-id
│       ├── db/
│       │   ├── queries.go       pgx/v5 parameterised queries (Neon)
│       │   ├── daily_agg.go     hourly cron: materialise measurements_daily
│       │   └── migrations/
│       │       ├── 001_initial.up.sql
│       │       ├── 001_initial.down.sql
│       │       ├── 002_gpu_tables.up.sql    (Phase 2 placeholder, do not apply yet)
│       │       └── 002_gpu_tables.down.sql
│       ├── github/
│       │   ├── app.go           GitHub App JWT + installation tokens
│       │   └── comments.go      PR comment search-and-update
│       ├── alerts/
│       │   ├── slack.go
│       │   └── email.go
│       └── cron/
│           └── aggregator.go    hourly goroutine, replaces TimescaleDB aggregates
│
├── dashboard/                   ← BUILD THIRD inside this repo (Step 2C)
│   ├── app/
│   │   ├── layout.tsx
│   │   ├── (auth)/login/page.tsx
│   │   ├── (auth)/callback/page.tsx
│   │   └── dashboard/[owner]/[repo]/
│   │       ├── page.tsx         repo overview: energy trend chart
│   │       ├── pr/[number]/page.tsx
│   │       └── settings/page.tsx
│   └── components/
│       ├── EnergyTrendChart.tsx
│       ├── DomainBreakdownChart.tsx
│       ├── PRComparisonTable.tsx
│       └── NoiseScatterPlot.tsx
│
└── docs/                        ← BUILD LAST (Step 2D, ongoing)
    └── docs/
        ├── getting-started.md
        └── ci/

infra/
├── fly/fly.toml                 Fly.io config for apps/api
└── vercel/vercel.json           Vercel config for apps/dashboard

.github/workflows/
├── release-watt-sampler.yml     NOTE: this workflow is in REPO 1, not here
├── deploy-api.yml               fly deploy on push to main
└── deploy-dashboard.yml         vercel --prod on push to main

turbo.json
package.json                     pnpm workspace root

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
REPO 3: github.com/wattlint/gpu-tracer  (public — DO NOT BUILD YET)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Cargo.toml                       Only create a stub README for now
src/main.rs                      fn main() { println!("coming soon"); }
```

---

## PHASE 0: RUST ENERGY SAMPLER — `github.com/wattlint/watt-sampler`

**This entire phase happens inside `wattlint/watt-sampler`. Do not open `wattlint/platform` yet.**

### Release Workflow (`.github/workflows/release.yml` inside watt-sampler repo)

This is the first file to create. It compiles cross-platform binaries and attaches them to a GitHub Release. The Action in `platform` downloads binaries from this release. Without this, Step 2A cannot be tested.

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            artifact: watt-sampler-linux-amd64
            os: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            artifact: watt-sampler-linux-arm64
            os: ubuntu-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Install cross-compilation tools
        run: |
          sudo apt-get install -y musl-tools
          cargo install cross --git https://github.com/cross-rs/cross
      - name: Build
        run: cross build --release --target ${{ matrix.target }}
      - name: Rename binary
        run: mv target/${{ matrix.target }}/release/watt-sampler ${{ matrix.artifact }}
      - name: Compute SHA256
        run: sha256sum ${{ matrix.artifact }} > ${{ matrix.artifact }}.sha256
      - name: Upload to release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            ${{ matrix.artifact }}
            ${{ matrix.artifact }}.sha256
```

**After the first release tag, copy the SHA256 values from the release assets into `platform/actions/energy-gate/src/installer.ts`. This is the supply-chain security check — the Action refuses to execute any binary whose checksum does not match.**

### STEP 1 IS COMPLETE WHEN:

All five of these are true — do not move to Step 2 until all five pass:

- [ ] `cargo test` passes with zero failures
- [ ] `cargo clippy` has zero warnings
- [ ] `watt-sampler run -- sleep 1` produces valid JSON on a Linux machine with RAPL
- [ ] `watt-sampler diff --baseline a.json --current b.json` computes correct delta
- [ ] `watt-sampler check-hardware` correctly reports RAPL method and GPU presence
- [ ] Tag `v0.1.0` pushed, GitHub Release created, binaries attached, SHA256 files attached



### Cargo.toml Dependencies (exact versions)

```toml
[dependencies]
perf-event = "0.4"
nvml-wrapper = "0.10"
sysinfo = "0.30"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
tokio = { version = "1", features = ["full"] }
nix = { version = "0.29", features = ["process", "sched"] }
```

### src/rapl.rs — RAPL Access Chain

Implement these methods in priority order. Stop at the first one that succeeds.

**Method 1: perf_event_open (preferred)**
```rust
// Check: /sys/bus/event_source/devices/power/events/energy-pkg exists
// If yes: use perf-event crate to open counting events
// Requires perf_event_paranoid <= 2 (no root needed)
// Open separately for each available domain:
//   energy-pkg   → package total (always try this first)
//   energy-cores → PP0 CPU cores (optional)
//   energy-ram   → DRAM (optional)
//   energy-psys  → platform total (Skylake+, optional)
```

**Method 2: sysfs powercap**
```rust
// Read /sys/class/powercap/intel-rapl/intel-rapl:*/energy_uj
// Parse subdirectory hierarchy:
//   intel-rapl:0       → package 0
//   intel-rapl:0:0     → PP0 (cores) of package 0
//   intel-rapl:0:1     → PP1 (uncore) if present
//   intel-rapl:1       → package 1 (multi-socket)
// Values in microjoules (u64 string in file)
// Note: Linux 5.10+ needs CAP_SYS_ADMIN — catch PermissionDenied, fall to method 3
```

**Method 3: AMD RAPL (parallel, detect by cpu_vendor)**
```rust
// Check /proc/cpuinfo for "vendor_id\t: AuthenticAMD"
// Read /sys/devices/platform/amd_energy/energy*_input (microjoules)
// Enumerate glob pattern, sort by filename for consistent ordering
// Available on Zen+ (Ryzen 2000+, EPYC 2nd gen+) only
```

**Method 4: MSR (last resort — log warning that root is required)**
```rust
// Only reach here if methods 1, 2, 3 all failed
// Open /dev/cpu/0/msr (requires root OR modprobe msr + permissions)
// Read energy unit from MSR_RAPL_POWER_UNIT = 0x606
//   Energy unit = 2^(-(bits 12:8 of result)) joules per LSB
// PKG energy: MSR_PKG_ENERGY_STATUS = 0x611
// DRAM energy: MSR_DRAM_ENERGY_STATUS = 0x619
// PP0 energy: MSR_PP0_ENERGY_STATUS = 0x639
// PSYS energy: MSR_PLATFORM_ENERGY_STATUS = 0x64D
```

**OVERFLOW HANDLING — implement this exactly or measurements will be silently wrong:**
```rust
// RAPL energy registers are 32-bit counters
// At 84W TDP they overflow approximately every 52 minutes
// Sampling at 100ms intervals guarantees we catch every overflow

pub struct RaplAccumulator {
    last_raw: u64,
    cumulative_uj: u128,  // u128: cannot overflow for any realistic workload
}

impl RaplAccumulator {
    pub fn new(initial_raw: u64) -> Self {
        Self { last_raw: initial_raw, cumulative_uj: 0 }
    }

    pub fn update(&mut self, new_raw: u64) {
        if new_raw < self.last_raw {
            // Overflow occurred: counter wrapped from near-MAX back to near-0
            // Distance = (MAX - last) + (new - 0) + 1
            self.cumulative_uj += (u32::MAX as u128 - self.last_raw as u128)
                                  + new_raw as u128 + 1;
        } else {
            self.cumulative_uj += new_raw - self.last_raw;
        }
        self.last_raw = new_raw;
    }

    pub fn total_joules(&self) -> f64 {
        self.cumulative_uj as f64 / 1_000_000.0  // microjoules → joules
    }
}

// Unit test — must pass:
#[test]
fn test_overflow_detection() {
    let mut acc = RaplAccumulator::new(u32::MAX as u64 - 100);
    acc.update(50);
    // Expected: (100 units from last to MAX) + (50 units from 0 to new) + 1
    assert_eq!(acc.cumulative_uj, 151);
}
```

### src/gpu.rs — GPU Power Reading

```rust
// NVIDIA path (try first):
// 1. nvml::Nvml::init()  — returns error if no NVIDIA GPU present (handle gracefully)
// 2. For each device index 0..device_count:
//    device.power_usage()? returns MILLIWATTS (not watts — common mistake)
// 3. Sample every 100ms, collect 10 readings per iteration, return average
// 4. For multi-GPU: Vec<GpuMeasurement> with per-device AND aggregated total

// AMD GPU path (try if NVML init fails):
// Enumerate /sys/class/drm/card*/device/hwmon/hwmon*/power1_average
// Values in MICROWATTS
// Read, convert: microwatts / 1000 = milliwatts

// Neither available:
// Return Ok(None)  — NOT an error
// Log info message to stderr: "No GPU detected, gpu_joules will be null"
// Do NOT exit non-zero
```

### src/normalizer.rs — Energy Efficiency Score

```rust
// EES = measured_joules / (cpu_tdp_watts × duration_s)
// EES < 1.0 → more efficient than TDP-ceiling expectation
// EES > 1.0 → less efficient
//
// Read actual configured TDP (not marketing TDP):
// Path 1: /sys/devices/virtual/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw
// Path 2: /sys/class/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw
// Value is in MICROWATTS — divide by 1_000_000 to get watts
// If neither path exists: use sysinfo crate TDP estimate

pub fn compute_ees(joules: f64, tdp_watts: f64, duration_s: f64) -> Option<f64> {
    if tdp_watts <= 0.0 || duration_s <= 0.0 || joules < 0.0 {
        return None;
    }
    Some(joules / (tdp_watts * duration_s))
}
```

### src/runner.rs — Subprocess + Measurement Loop

```rust
// Before spawning benchmark:
// 1. Record start energy snapshot from all available RAPL domains
// 2. Spawn subprocess with std::process::Command
// 3. Set CPU affinity to cores 2..N-1 using nix::sched::sched_setaffinity
//    (core 0 has OS interrupt handler — avoid it)
// 4. Start background thread polling RAPL at 100ms intervals
//    (needed to detect overflow, also gives real-time power curve)
// 5. Wait for subprocess to complete
// 6. Record end energy snapshot
// 7. Compute total = end_snapshot - start_snapshot (using accumulators)
//
// Noise warnings (do not fail — only warn):
//   - Check /proc/stat before each iteration for other-process CPU > 5%
//   - Check scaling_governor: warn if not "performance"
//   - Check if avg_watts > tdp_watts × 0.95 (thermal throttling)
//
// Iteration loop:
//   for i in 0..(warmup_iters + measurement_iters):
//       result = run_once()
//       if i >= warmup_iters: measurements.push(result)
//
// Statistical summary:
//   let sorted: Vec<f64> = measurements.iter().map(|m| m.total_joules).collect::<Vec<_>>()
//   sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
//   let median = sorted[sorted.len() / 2];
//   let p5 = sorted[sorted.len() / 20];
//   let p95 = sorted[sorted.len() * 19 / 20];
//   let noise_pct = (p95 - p5) / median * 100.0;
//
//   if noise_pct > 15.0 { warnings.push("High measurement noise..."); valid = false; }
```

### src/main.rs — CLI Commands

Implement exactly these three subcommands:

```
watt-sampler run [OPTIONS] -- <COMMAND> [ARGS...]

Options:
  --iterations <N>       Measurement iterations [default: 7]
  --warmup <N>           Warmup iterations to discard [default: 2]
  --domains <DOMAINS>    RAPL domains: pkg,pp0,dram,psys,gpu [default: pkg,gpu]
  --output <FORMAT>      json | human [default: json]
  --no-affinity          Skip CPU affinity pinning (use in VMs that don't support it)
  --force                Continue even if noise > 20%

watt-sampler diff --baseline <FILE> --current <FILE> [OPTIONS]

Options:
  --threshold <PCT>      Regression threshold percent [default: 10]
  --output <FORMAT>      json | human | markdown [default: human]
  --fail                 Exit 1 if regression detected (use in CI)

watt-sampler check-hardware
  (no options — detects and reports hardware capabilities)
```

### JSON Output Schema (stdout ONLY — errors/warnings to stderr)

```json
{
  "schema_version": "1",
  "timestamp_utc": "2025-12-30T10:00:00Z",
  "host": {
    "cpu_vendor": "GenuineIntel",
    "cpu_model": "Intel Xeon Gold 6226R",
    "cpu_tdp_watts": 150,
    "cpu_sockets": 2,
    "rapl_method": "perf_event | powercap | msr | estimated",
    "gpu": [
      {"index": 0, "name": "NVIDIA A100 80GB", "tdp_watts": 400}
    ]
  },
  "benchmark": {
    "command": "./target/release/benchmark --time 5s",
    "iterations_requested": 7,
    "warmup_iters": 2,
    "domains_measured": ["pkg", "dram", "gpu"]
  },
  "measurements": [
    {
      "iteration": 1,
      "duration_s": 4.832,
      "energy": {
        "pkg_joules": 412.3,
        "pp0_joules": null,
        "dram_joules": 28.1,
        "psys_joules": null,
        "gpu_joules": 1840.2,
        "total_joules": 2280.6
      },
      "power": {
        "pkg_avg_watts": 85.3,
        "gpu_avg_watts": 380.7
      }
    }
  ],
  "summary": {
    "median_total_joules": 2281.4,
    "p5_total_joules": 2270.1,
    "p95_total_joules": 2290.8,
    "median_pkg_watts": 85.5,
    "ees": 0.87,
    "noise_pct": 1.4,
    "valid": true,
    "warnings": []
  }
}
```

**Contract:**
- stdout: JSON only. No print statements. No progress bars. No banners.
- stderr: warnings, progress, errors. Everything that is not JSON.
- exit 0: success, valid measurement
- exit 1: invalid measurement (noise > 20%) OR regression detected (diff --fail)
- exit 2: hardware error (no RAPL, missing NVML, permission denied)

---

## PHASE 1: GITHUB ACTION — `github.com/wattlint/platform/actions/energy-gate/`

**This is the first thing you build inside `wattlint/platform`. Open this repo only after `watt-sampler` v0.1.0 is released with binaries and SHA256 checksums.**

### action.yml

```yaml
name: 'WattLint Energy Gate'
description: 'Catch energy waste before it ships'
branding:
  icon: 'zap'
  color: 'yellow'
inputs:
  benchmark-command:    { required: true }
  threshold-pct:        { default: '10' }
  iterations:           { default: '7' }
  domains:              { default: 'pkg,gpu' }
  api-token:            { required: false }
  fail-on-regression:   { default: 'true' }
outputs:
  energy-delta-pct:     {}
  regression-detected:  {}
  baseline-joules:      {}
  current-joules:       {}
runs:
  using: 'node20'
  main: 'dist/index.js'
```

### installer.ts

```typescript
// Binary versions and SHA256 checksums (update on every release)
const BINARY_VERSION = "0.1.0"
const SHA256: Record<string, string> = {
  "linux-x64":   "REPLACE_WITH_ACTUAL_CHECKSUM",
  "linux-arm64": "REPLACE_WITH_ACTUAL_CHECKSUM",
}

export async function installWattSampler(): Promise<string> {
  const arch = process.arch === 'arm64' ? 'linux-arm64' : 'linux-x64'
  const url = `https://github.com/wattlint/watt-sampler/releases/download/v${BINARY_VERSION}/watt-sampler-${arch}`

  // Check tool cache first
  let binaryPath = tc.find('watt-sampler', BINARY_VERSION, arch)
  if (!binaryPath) {
    const downloadPath = await tc.downloadTool(url)
    // MANDATORY: verify SHA256 before executing
    const actual = await computeSHA256(downloadPath)
    if (actual !== SHA256[arch]) {
      throw new Error(`SHA256 mismatch. Expected ${SHA256[arch]}, got ${actual}. Refusing to execute.`)
    }
    await fs.chmod(downloadPath, 0o755)
    binaryPath = await tc.cacheFile(downloadPath, 'watt-sampler', 'watt-sampler', BINARY_VERSION, arch)
  }

  return path.join(binaryPath, 'watt-sampler')
}

export async function checkRAPLAvailable(): Promise<{ available: boolean; method: string }> {
  // Try perf_event path
  if (fs.existsSync('/sys/bus/event_source/devices/power/events/energy-pkg')) {
    return { available: true, method: 'perf_event' }
  }
  // Try powercap path
  if (fs.existsSync('/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj')) {
    return { available: true, method: 'powercap' }
  }
  // Try AMD path
  if (fs.existsSync('/sys/devices/platform/amd_energy')) {
    return { available: true, method: 'amd_energy' }
  }
  // GitHub-hosted runners will hit here
  core.warning('RAPL not available on this runner. Results will be estimated using cloud energy model.')
  core.warning('For real hardware measurements, use a self-hosted runner.')
  return { available: false, method: 'estimated' }
}
```

### baseline.ts

```typescript
export async function getOrCreateBaseline(config: ActionConfig): Promise<MeasurementResult> {
  const cacheKey = `wattlint-baseline-${computeHash(
    config.repoFullName + config.defaultBranch + config.benchmarkCommand
  )}`

  const cacheHit = await cache.restoreCache([BASELINE_PATH], cacheKey)

  if (cacheHit) {
    const baseline = JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf8'))
    const ageMs = Date.now() - new Date(baseline.timestamp_utc).getTime()
    const ageDays = ageMs / (1000 * 60 * 60 * 24)

    if (ageDays > 7) {
      core.warning(`Baseline is ${ageDays.toFixed(1)} days old. Refreshing.`)
      // Fall through to rebuild
    } else {
      return baseline
    }
  }

  // No cache or stale — build baseline from main branch
  core.info('No baseline found. Checking out main branch to build baseline.')
  await exec.exec('git', ['fetch', 'origin', config.defaultBranch])
  await exec.exec('git', ['checkout', config.defaultBranch])
  // Build the project (detect build system)
  await buildProject(config)
  const baseline = await runWattSampler(config)
  fs.writeFileSync(BASELINE_PATH, JSON.stringify(baseline))
  await cache.saveCache([BASELINE_PATH], cacheKey)
  return baseline
}
```

### reporter.ts

```typescript
export function buildPRComment(
  baseline: MeasurementResult,
  current: MeasurementResult,
  config: ActionConfig
): string {
  const deltaPkg = computeDelta(baseline.summary.median_pkg_watts, current.summary.median_pkg_watts)
  const deltaTotal = computeDelta(baseline.summary.median_total_joules, current.summary.median_total_joules)
  const deltaDram = baseline.summary.median_dram_joules
    ? computeDelta(baseline.summary.median_dram_joules, current.summary.median_dram_joules)
    : null

  const overThreshold = deltaTotal > config.thresholdPct
  const statusEmoji = overThreshold ? '🔴' : (deltaTotal < -2 ? '🟢' : '✅')
  const statusText = overThreshold ? 'REGRESSION DETECTED' : (deltaTotal < -2 ? 'IMPROVEMENT' : 'WITHIN THRESHOLD')

  // Advisory analysis (rule-based)
  const advisory = generateAdvisory(deltaPkg, deltaDram, deltaTotal, current)

  return `<!-- wattlint-report -->
## ⚡ WattLint Energy Report

| Domain | Baseline (${config.defaultBranch}) | This PR | Δ |
|--------|------|---------|---|
| CPU Package (median) | ${baseline.summary.median_total_joules.toFixed(1)} J | ${current.summary.median_total_joules.toFixed(1)} J | ${formatDelta(deltaPkg)} |
${deltaDram !== null ? `| DRAM (median) | ${baseline.summary.median_dram_joules!.toFixed(1)} J | ${current.summary.median_dram_joules!.toFixed(1)} J | ${formatDelta(deltaDram)} |\n` : ''}| **Total (median)** | **${baseline.summary.median_total_joules.toFixed(1)} J** | **${current.summary.median_total_joules.toFixed(1)} J** | **${formatDelta(deltaTotal)}** |
| Avg CPU Power | ${baseline.summary.median_pkg_watts.toFixed(1)} W | ${current.summary.median_pkg_watts.toFixed(1)} W | ${formatDelta(deltaPkg)} |
| Benchmark duration | ${baseline.measurements[0].duration_s.toFixed(2)} s | ${current.measurements[0].duration_s.toFixed(2)} s | ${formatDelta(computeDurationDelta(baseline, current))} |

**Threshold:** ${config.thresholdPct}% · **Status:** ${statusEmoji} ${statusText}

${advisory ? `> 💡 ${advisory}\n` : ''}
<details>
<summary>All ${current.benchmark.iterations_requested} iterations</summary>
...
</details>

<details>
<summary>Environment</summary>
CPU: ${current.host.cpu_model} · TDP: ${current.host.cpu_tdp_watts} W · Measurement: RAPL ${current.host.rapl_method} · Iterations: ${current.benchmark.iterations_requested} (warmup: ${current.benchmark.warmup_iters}) · Noise: ${current.summary.noise_pct.toFixed(1)}%${current.host.gpu.length > 0 ? ` · GPU: ${current.host.gpu[0].name}` : ''}
</details>`
}

// CRITICAL: search for existing comment before posting
export async function postOrUpdatePRComment(
  octokit: Octokit,
  owner: string,
  repo: string,
  prNumber: number,
  body: string
): Promise<void> {
  const MARKER = '<!-- wattlint-report -->'

  // Search existing comments
  const { data: comments } = await octokit.issues.listComments({
    owner, repo, issue_number: prNumber, per_page: 100
  })

  const existing = comments.find(c => c.body?.includes(MARKER))

  if (existing) {
    await octokit.issues.updateComment({ owner, repo, comment_id: existing.id, body })
    core.info(`Updated existing WattLint comment (ID: ${existing.id})`)
  } else {
    await octokit.issues.createComment({ owner, repo, issue_number: prNumber, body })
    core.info('Created new WattLint comment')
  }
}
```

---

### STEP 2A IS COMPLETE WHEN:

All of these are true — do not build the API until all pass:

- [ ] `npm test` passes with zero failures
- [ ] `npm run build` compiles to `dist/index.js` without TypeScript errors
- [ ] Adding the Action to a test repo's workflow and opening a PR results in a PR comment containing `<!-- wattlint-report -->`
- [ ] Updating the PR does not create a second comment — it updates the existing one
- [ ] Threshold breach causes the Action to exit with a failure status that blocks PR merge
- [ ] RAPL unavailable case (GitHub-hosted runner) shows "ESTIMATED" label in comment

---

## PHASE 2: GO API SERVER — `github.com/wattlint/platform/apps/api/`

### go.mod Dependencies

```go
require (
    github.com/go-chi/chi/v5        v5.0.12
    github.com/jackc/pgx/v5         v5.5.5
    github.com/redis/go-redis/v9    v9.5.1
    github.com/google/go-github/v60 v60.0.0
    golang.org/x/crypto             v0.21.0
    github.com/stripe/stripe-go/v76 v76.0.0
)
```

### Database Schema (Migration 001)

Create this exactly. Use `golang-migrate/migrate` — never raw CREATE TABLE in application code.

```sql
-- apps/api/internal/db/migrations/001_initial.up.sql

CREATE TABLE repos (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id       BIGINT UNIQUE NOT NULL,
    full_name       TEXT NOT NULL,
    api_token_hash  TEXT NOT NULL,
    plan            TEXT DEFAULT 'free',
    stripe_customer TEXT,
    default_branch  TEXT DEFAULT 'main',
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE measurements (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    time                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    repo_id             UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    commit_sha          CHAR(40) NOT NULL,
    branch              TEXT NOT NULL,
    pr_number           INT,
    benchmark_cmd_hash  TEXT NOT NULL,
    benchmark_cmd       TEXT NOT NULL,
    is_baseline         BOOLEAN DEFAULT FALSE,
    runner_type         TEXT NOT NULL,
    pkg_joules_median   DOUBLE PRECISION,
    pkg_joules_p5       DOUBLE PRECISION,
    pkg_joules_p95      DOUBLE PRECISION,
    dram_joules_median  DOUBLE PRECISION,
    pp0_joules_median   DOUBLE PRECISION,
    cpu_avg_watts       DOUBLE PRECISION,
    cpu_vendor          TEXT NOT NULL,
    cpu_model           TEXT NOT NULL,
    cpu_tdp_watts       INT,
    gpu_joules_median   DOUBLE PRECISION,
    gpu_avg_watts       DOUBLE PRECISION,
    gpu_name            TEXT,
    total_joules_median DOUBLE PRECISION NOT NULL,
    total_joules_p95    DOUBLE PRECISION,
    ees                 DOUBLE PRECISION,
    noise_pct           DOUBLE PRECISION,
    duration_s          DOUBLE PRECISION,
    iterations          INT DEFAULT 7,
    valid               BOOLEAN DEFAULT TRUE,
    warnings            TEXT[],
    raw_json            JSONB
);

CREATE INDEX idx_measurements_baseline
    ON measurements (repo_id, benchmark_cmd_hash, cpu_vendor, time DESC)
    WHERE is_baseline = TRUE AND valid = TRUE;

CREATE INDEX idx_measurements_repo_time
    ON measurements (repo_id, time DESC);

CREATE INDEX idx_measurements_pr
    ON measurements (repo_id, pr_number, time DESC)
    WHERE pr_number IS NOT NULL;

CREATE TABLE measurements_daily (
    day                  DATE NOT NULL,
    repo_id              UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    benchmark_cmd_hash   TEXT NOT NULL,
    branch               TEXT NOT NULL,
    avg_total_joules     DOUBLE PRECISION,
    min_total_joules     DOUBLE PRECISION,
    max_total_joules     DOUBLE PRECISION,
    avg_ees              DOUBLE PRECISION,
    measurement_count    INT,
    updated_at           TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (day, repo_id, benchmark_cmd_hash, branch)
);

CREATE TABLE cron_state (
    key       TEXT PRIMARY KEY,
    value     TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE alerts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id       UUID NOT NULL REFERENCES repos(id),
    pr_number     INT NOT NULL,
    commit_sha    CHAR(40),
    delta_pct     DOUBLE PRECISION NOT NULL,
    threshold_pct DOUBLE PRECISION NOT NULL,
    alert_type    TEXT NOT NULL,
    channels      TEXT[],
    created_at    TIMESTAMPTZ DEFAULT NOW()
);
```

### Daily Aggregator Cron

```go
// apps/api/internal/cron/aggregator.go
// Start this as a goroutine from main.go at server startup
// Run every 1 hour using time.NewTicker(time.Hour)

func RunAggregator(ctx context.Context, db *pgxpool.Pool) {
    ticker := time.NewTicker(time.Hour)
    defer ticker.Stop()

    // Run once at startup, then on every tick
    runOnce(ctx, db)
    for {
        select {
        case <-ticker.C:
            runOnce(ctx, db)
        case <-ctx.Done():
            return
        }
    }
}

func runOnce(ctx context.Context, db *pgxpool.Pool) {
    // Get last processed time from cron_state
    // Query: GROUP BY date_trunc('day', time), repo_id, benchmark_cmd_hash, branch
    //        WHERE time > last_processed AND valid = TRUE
    // Upsert into measurements_daily using ON CONFLICT DO UPDATE
    // Update cron_state with current time
    // Log: "Aggregated N new measurements" to stdout
}
```

### API Route Handlers

```go
// POST /api/v1/measurements
// Auth: Bearer token → bcrypt.CompareHashAndPassword against repos.api_token_hash
// Validate: schema_version == "1", valid == true, finite float values
// Insert into measurements
// If pr_number != null: trigger PR comment update via GitHub API
// Return: 201 {"measurement_id": "..."}

// GET /api/v1/repos/:id/baseline?benchmark=<hash>&cpu_vendor=<vendor>&runner_type=<type>
// Auth: Bearer token
// Return: most recent measurement WHERE is_baseline=TRUE AND cpu_vendor=$1 AND runner_type=$2
// If none: 404 (action will build baseline from main branch)

// GET /api/v1/repos/:id/trend?days=30&benchmark=<hash>
// Return: daily aggregates from measurements_daily for Recharts chart
// Format: [{"day": "2025-12-01", "avg_total_joules": 2281.4, "min": 2270.1, "max": 2295.2}]

// POST /webhooks/github
// MANDATORY: verify HMAC-SHA256 signature: X-Hub-Signature-256 header
// Reject any request that fails signature check with 401
// Handle: pull_request.opened, pull_request.synchronize, pull_request.reopened
```

### Security Requirements (all mandatory)

```go
// 1. API tokens: bcrypt.GenerateFromPassword(token, 12) before storing
//    Never store or log plaintext tokens
//    Token shown ONCE to user on creation, never again

// 2. Webhook signature verification:
//    mac := hmac.New(sha256.New, []byte(webhookSecret))
//    mac.Write(body)
//    expected := "sha256=" + hex.EncodeToString(mac.Sum(nil))
//    if !hmac.Equal([]byte(expected), []byte(header)) { return 401 }

// 3. Rate limiting with Upstash Redis (sliding window):
//    Key: "ratelimit:" + sha256(token)[:16]
//    100 requests/minute per token
//    Reject: 429 with Retry-After: 60

// 4. SQL: ONLY parameterised queries via pgx. ZERO string interpolation into SQL.
//    Correct:   db.QueryRow(ctx, "SELECT id FROM repos WHERE id = $1", id)
//    Forbidden: db.QueryRow(ctx, "SELECT id FROM repos WHERE id = '" + id + "'")

// 5. Body size limit: 10MB max (watt-sampler JSON is typically <50KB)
//    Reject with 413 if exceeded

// 6. No NaN or Inf in database: validate all float64 values before insert
//    if math.IsNaN(v) || math.IsInf(v, 0) { return validation error }
```

---

## PHASE 3: DASHBOARD — `github.com/wattlint/platform/apps/dashboard/`

### EnergyTrendChart.tsx

```tsx
// Recharts ComposedChart
// X-axis: date (formatted as "Dec 30")
// Y-axis: joules
// Line: avg_total_joules (solid, primary color)
// Area: P5 to P95 band (shaded, low opacity — shows measurement uncertainty)
// Scatter: individual PR data points
//   Red dot: regression (delta > threshold)
//   Green dot: improvement (delta < -2%)
//   Grey dot: within threshold
// Tooltip: show commit SHA, PR number, delta vs previous
// Responsive: use ResponsiveContainer width="100%"
```

### PRComparisonTable.tsx

```tsx
// Bar chart: grouped bars for baseline vs current, per energy domain
// Domains: CPU Package, DRAM (if available), GPU (if available), Total
// Color: baseline = grey, current = red if regression / green if improvement
// Delta labels above each bar pair: "+23.0%" in red, "-0.7%" in green
```

### NoiseScatterPlot.tsx

```tsx
// Scatter plot showing all iteration measurements
// X-axis: iteration number (1-7)
// Y-axis: total joules for that iteration
// Two series: baseline (grey dots) and current PR (colored dots)
// Horizontal line: median value for each series
// Purpose: shows developers the noise level and makes trust visible
// If noise_pct > 10%: add annotation "High variance — consider using a self-hosted runner"
```

---

## TESTING REQUIREMENTS

### Rust (packages/watt-sampler/)

```rust
// tests/unit.rs — these must all pass:

#[test]
fn test_rapl_overflow_detection() {
    let mut acc = RaplAccumulator::new(u32::MAX as u64 - 100);
    acc.update(50);
    assert_eq!(acc.cumulative_uj, 151);
}

#[test]
fn test_ees_computation() {
    // 1000J at 150W TDP for 10s → EES = 1000 / (150 × 10) = 0.667
    let ees = compute_ees(1000.0, 150.0, 10.0).unwrap();
    assert!((ees - 0.667).abs() < 0.001);
}

#[test]
fn test_median_of_seven() {
    let values = vec![100.0, 102.0, 98.0, 103.0, 99.0, 101.0, 105.0];
    // Sorted: [98, 99, 100, 101, 102, 103, 105] → median = 101
    assert_eq!(compute_median(&values), 101.0);
}

// tests/integration.rs — requires Linux with RAPL access:
#[test]
#[cfg(target_os = "linux")]
fn test_real_measurement() {
    // Run watt-sampler on a simple CPU-bound program (sleep 1)
    // Verify output is valid JSON with schema_version = "1"
    // Verify all required fields are present
    // Verify noise_pct is finite and reasonable (0..100)
    // Verify total_joules > 0
}
```

### TypeScript (actions/energy-gate/)

```typescript
// __tests__/installer.test.ts
// Mock fs, tc (tool-cache), and network
// Test: SHA256 mismatch causes throw (supply chain security check)
// Test: correct arch detection for linux-x64 vs linux-arm64
// Test: cache hit skips download

// __tests__/reporter.test.ts
// Snapshot test: given fixed baseline and current, output matches expected markdown
// Test: MARKER comment is always present in output
// Test: regression correctly identified at various delta values
// Test: within-threshold correctly identified

// __tests__/baseline.test.ts
// Mock @actions/cache
// Test: stale baseline (>7 days) triggers rebuild
// Test: fresh cache hit returns without rebuild
```

### Go (apps/api/)

```go
// internal/api/measurements_test.go
// Use testcontainers-go to spin up real Neon-compatible PostgreSQL
// Test: valid measurement accepted and stored
// Test: NaN/Inf in float fields rejected with 422
// Test: missing required fields rejected with 422
// Test: invalid API token rejected with 401
// Test: webhook signature mismatch rejected with 401
// Test: rate limit enforced after 100 requests
// All SQL queries must use parameterised form — test with SQL injection attempt
```

---

## FLY.IO DEPLOYMENT

```toml
# infra/fly/fly.toml
app = "wattlint-api"
primary_region = "iad"

[build]
  dockerfile = "apps/api/Dockerfile"

[[services]]
  internal_port = 8080
  protocol = "tcp"
  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]
  [services.concurrency]
    type = "connections"
    hard_limit = 25
    soft_limit = 20

[[vm]]
  memory = "256mb"
  cpu_kind = "shared"
  cpus = 1
```

```dockerfile
# apps/api/Dockerfile
FROM golang:1.23-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /wattlint-api ./cmd/api

FROM scratch
COPY --from=builder /wattlint-api /wattlint-api
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
EXPOSE 8080
ENTRYPOINT ["/wattlint-api"]
```

**Fly.io secrets to set (fly secrets set KEY=value):**
```
NEON_DATABASE_URL          postgres://...@ep-xxx.us-east-2.aws.neon.tech/neondb?sslmode=require
UPSTASH_REDIS_URL          rediss://default:...@...upstash.io:6380
GITHUB_APP_ID              12345
GITHUB_APP_PRIVATE_KEY     -----BEGIN RSA PRIVATE KEY-----...
GITHUB_WEBHOOK_SECRET      random-32-char-secret
STRIPE_SECRET_KEY          sk_live_...
```

---

## VERCEL DEPLOYMENT

```json
// infra/vercel/vercel.json
{
  "framework": "nextjs",
  "buildCommand": "cd apps/dashboard && pnpm build",
  "outputDirectory": "apps/dashboard/.next",
  "installCommand": "pnpm install",
  "env": {
    "NEXT_PUBLIC_API_URL": "https://api.wattlint.com"
  }
}
```

---

## DELIVERABLE CHECKLIST — ALL MUST PASS

### Rust sampler
- [ ] `watt-sampler run -- ./benchmark` outputs valid JSON to stdout, nothing else
- [ ] `watt-sampler diff --baseline a.json --current b.json` reports correct delta
- [ ] `watt-sampler check-hardware` correctly detects RAPL method and GPU presence
- [ ] RAPL overflow unit test passes
- [ ] EES computation unit test passes  
- [ ] Median calculation unit test passes
- [ ] `cargo test` all tests pass
- [ ] `cargo clippy` zero warnings
- [ ] Binary size < 10MB (verify with `ls -lh target/release/watt-sampler`)
- [ ] Binary is statically linked (verify with `ldd target/release/watt-sampler`)

### GitHub Action
- [ ] `action.yml` is syntactically valid (`yamllint action.yml` passes)
- [ ] Action posts PR comment with `<!-- wattlint-report -->` marker
- [ ] Action updates existing comment, never creates duplicate
- [ ] SHA256 mismatch causes action to fail with clear error message
- [ ] `npm test` all tests pass
- [ ] TypeScript strict mode: zero errors

### Go API
- [ ] Server starts and passes health check: `GET /health` → `{"status": "ok"}`
- [ ] Valid measurement accepted: `POST /api/v1/measurements` → 201
- [ ] Invalid token rejected: 401
- [ ] NaN float values rejected: 422
- [ ] Webhook signature mismatch: 401
- [ ] `go test ./...` all tests pass
- [ ] `go vet ./...` zero warnings
- [ ] All SQL is parameterised: grep the codebase for string concatenation in query strings

### Dashboard
- [ ] `pnpm build` succeeds without TypeScript errors
- [ ] Energy trend chart renders with real data from API
- [ ] PR comparison table shows correct delta values
- [ ] Settings page saves threshold and returns 200

### Infrastructure
- [ ] `docker-compose up` starts PostgreSQL + Redis locally
- [ ] Database migrations run cleanly: `migrate up` and `migrate down` both work
- [ ] Fly.io deploy succeeds: `flyctl deploy`
- [ ] Vercel deploy succeeds: `vercel --prod`

---

## START ORDER

1. Build `packages/watt-sampler/` first. Validate it produces correct JSON on a real Linux machine.
2. Build `actions/energy-gate/` and test against a real GitHub repo.
3. Build `apps/api/` with all routes and database migrations.
4. Build `apps/dashboard/` connecting to the real API.
5. Deploy API to Fly.io, dashboard to Vercel.
6. End-to-end test: open a PR on a test repo, verify the full flow: Action runs → measurement stored → PR comment posted → dashboard shows data.

Do not start step N+1 until step N is fully working and tested.
