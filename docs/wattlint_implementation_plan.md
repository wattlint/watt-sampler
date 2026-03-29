# WattLint — Platform Implementation Plan
## The energy linter for software teams

> **Domain:** wattlint.com
> **GitHub org:** github.com/wattlint (already created ✓)
> **Repos:** wattlint/watt-sampler, wattlint/gpu-tracer, wattlint/platform (all created ✓)
> **Tagline:** Catch energy waste before it ships.
> **Elevator pitch:** WattLint is to watts what ESLint is to code style — an automated gate in your CI pipeline that flags energy regressions in pull requests before they reach production.
> **Mission:** Build the precision layer for software energy — from CI gates to GPU kernel observability — across the entire compute stack.

---

## ⚡ START HERE — Which Repo, In What Order, And Why

This is the only section you need to read before opening your editor.

You have three repos. Here is exactly what each one is for and the order you work in them:

```
STEP 1  →  github.com/wattlint/watt-sampler     ← OPEN THIS FIRST (Week 1–2)
STEP 2  →  github.com/wattlint/platform         ← OPEN THIS SECOND (Week 3–12)
STEP 3  →  github.com/wattlint/gpu-tracer       ← DO NOT TOUCH YET (Phase 2, Month 4+)
```

### Why this order is non-negotiable

The entire product is a pipeline:

```
[watt-sampler binary]
        ↓  downloaded and run by
[energy-gate GitHub Action]   ← lives in: platform/actions/energy-gate/
        ↓  sends JSON to
[Go API server]               ← lives in: platform/apps/api/
        ↓  data shown in
[Next.js dashboard]           ← lives in: platform/apps/dashboard/
```

`watt-sampler` is the root of this chain. If it produces wrong measurements, everything downstream is wrong. If it does not exist as a compiled binary on GitHub Releases, the GitHub Action cannot download it and fails immediately. **Nothing else can be built, tested, or demonstrated until `watt-sampler` produces a correct JSON measurement on a real Linux machine.**

This is why you start in `wattlint/watt-sampler`, not in `wattlint/platform`.

---

### STEP 1 — `github.com/wattlint/watt-sampler` (Week 1–2)

**What this repo is:** A standalone Rust CLI tool. Public, open-source (MIT licence). This is the only thing you build in these two weeks.

**What it does:** Measures CPU and GPU energy consumption while running a benchmark command. Outputs a precise JSON report. Supports RAPL on Intel and AMD, NVML for NVIDIA GPUs.

**When STEP 1 is done:** Running this command on a Linux machine with RAPL access produces valid, noise-stable JSON:
```bash
./watt-sampler run --iterations 7 --warmup 2 -- ./my-benchmark
```
And the output looks like this (correct JSON, `valid: true`, `noise_pct < 5%`):
```json
{
  "schema_version": "1",
  "host": { "cpu_vendor": "GenuineIntel", "cpu_tdp_watts": 150, ... },
  "summary": { "median_total_joules": 2281.4, "ees": 0.87, "valid": true, "noise_pct": 1.4 }
}
```

**What to build inside this repo:**
```
watt-sampler/
├── Cargo.toml
└── src/
    ├── main.rs         ← CLI entrypoint (clap): `run`, `diff`, `check-hardware` subcommands
    ├── rapl.rs         ← RAPL reading: perf_event_open → powercap → AMD → MSR (in that order)
    ├── amd_rapl.rs     ← AMD-specific: /sys/devices/platform/amd_energy/
    ├── gpu.rs          ← NVML via nvml-wrapper crate (NEVER nvidia-smi subprocess)
    ├── normalizer.rs   ← EES = joules / (tdp_watts × duration_s)
    ├── runner.rs       ← spawn subprocess, poll RAPL at 100ms, collect 7 iterations
    ├── report.rs       ← JSON serialiser (stdout only; all warnings/errors to stderr)
    └── diff.rs         ← compare two JSON files, compute delta %, apply threshold
```

**GitHub Release required at end of Step 1.** The Action in STEP 2 downloads the binary from a GitHub Release. Before moving to STEP 2 you must:
1. Tag a release `v0.1.0` on `wattlint/watt-sampler`
2. Attach compiled binaries for `linux-amd64` and `linux-arm64`
3. Record the SHA256 checksums of both binaries (you will hardcode these in the Action)

---

### STEP 2 — `github.com/wattlint/platform` (Week 3–12)

**What this repo is:** The private monorepo containing everything else — the GitHub Action, the Go API, the Next.js dashboard, the docs site, and all infrastructure config.

**You work inside this repo in four sub-steps, in this order:**

#### STEP 2A — `actions/energy-gate/` (Week 3–4)
The GitHub Action that teams add to their CI. It downloads the `watt-sampler` binary released in STEP 1, runs it on the PR branch, compares energy to the baseline, and posts a PR comment.

**Depends on:** `watt-sampler` binary existing on GitHub Releases (done in STEP 1).

**When 2A is done:** Adding this to any repo's workflow catches energy regressions and posts a comment:
```yaml
- uses: wattlint/energy-gate@v1
  with:
    benchmark-command: cargo bench
    threshold-pct: 10
```

#### STEP 2B — `apps/api/` (Week 5–8)
The Go 1.23 API server. Receives measurement JSON from the Action, stores it in Neon, handles GitHub webhooks, updates PR comments, enforces billing plans.

**Depends on:** `actions/energy-gate/` working end-to-end so you can test the upload path (2A's `uploader.ts` calls this API).

**When 2B is done:** The full data pipeline works: Action runs → uploads JSON → API stores in Neon → GitHub PR comment shows the result → dashboard can query data.

#### STEP 2C — `apps/dashboard/` (Week 9–12)
The Next.js 15 dashboard at `app.wattlint.com`. Shows energy trend charts, per-PR breakdowns, and settings.

**Depends on:** API having real data in Neon to display (done in 2B).

**When 2C is done:** A user can sign in with GitHub, see their repo's 30-day energy trend, drill into any PR, and configure their threshold.

#### STEP 2D — `apps/docs/` (Week 12, ongoing)
The Docusaurus documentation site at `wattlint.com/docs`. Getting-started guide, self-hosted runner setup, noise reduction guide.

**Depends on:** Everything else working first — docs describe the real product.

---

### STEP 3 — `github.com/gpu-tracer` (Month 4+, Phase 2)

**Do not open this repo yet.** It is Phase 2. The GPU observability product lives here. You will begin work on it after WattLint CI is shipping revenue and you have paying Pro users. The architecture for this product is fully designed in Part 10 of this plan — it will be a separate Rust CLI that uses CUPTI + eBPF to trace GPU kernels, with results shown as a new tab in the existing `platform/apps/dashboard/`. The `platform/apps/api/` already has a placeholder migration (`002_gpu_tables.up.sql`) ready for when you start Phase 2.

---

### Daily Working Rule

At any point during development, you should always know which of these four files you are working on:

| Current task | File you are in |
|---|---|
| Building energy measurement | `wattlint/watt-sampler/src/` |
| Building the CI gate | `wattlint/platform/actions/energy-gate/src/` |
| Building the backend | `wattlint/platform/apps/api/internal/` |
| Building the frontend | `wattlint/platform/apps/dashboard/app/` |
| Deploying the backend | `wattlint/platform/infra/fly/` |
| Deploying the frontend | `wattlint/platform/infra/vercel/` |
| GPU observability (Phase 2) | `wattlint/gpu-tracer/src/` |

If you are ever unsure which file to open, return to this table.

---

### Relationship Between the Three Repos

```
wattlint/watt-sampler  (public OSS)
│
│   releases compiled binaries to GitHub Releases
│   e.g. watt-sampler-linux-amd64 v0.1.0
│
└──► wattlint/platform  (private)
         │
         ├── actions/energy-gate/
         │       downloads watt-sampler binary from watt-sampler releases
         │       runs it, uploads results to API
         │
         ├── apps/api/
         │       receives JSON from Action
         │       stores in Neon, posts PR comments
         │
         └── apps/dashboard/
                 reads from API
                 shows charts and PR history


wattlint/gpu-tracer  (public OSS, Phase 2 only)
         │
         releases compiled binaries to GitHub Releases
         │
└──► wattlint/platform  (same platform, new tab)
         ├── actions/gpu-profile/   (Phase 2)
         ├── apps/api/              (extends existing API with gpu_traces table)
         └── apps/dashboard/        (adds GPU tab to existing dashboard)
```

The `platform` repo never contains Rust source code. It only contains the consumer of `watt-sampler` (the Action), the backend that stores its output, and the frontend that displays it. Rust source lives in the dedicated OSS repos. This separation means:
- Anyone can use `watt-sampler` independently without WattLint's SaaS
- The OSS Rust tools drive GitHub stars and community trust
- The `platform` repo stays focused on product and infrastructure

---

## Part 0 — Brand Registration Checklist

Complete all of these before writing code. The GitHub org and repos are already done.

- [x] github.com/wattlint — GitHub organisation ✓
- [x] github.com/wattlint/platform — private repo ✓
- [x] github.com/wattlint/watt-sampler — public repo ✓
- [x] github.com/wattlint/gpu-tracer — public repo ✓
- [ ] wattlint.com — Cloudflare Registrar ($10/year, at-cost pricing, free DNSSEC)
- [ ] npmjs.com/org/wattlint — npm org `@wattlint` (for the Action package)
- [ ] crates.io — reserve `watt-sampler` and `gpu-tracer` crate names
- [ ] x.com/wattlint — Twitter/X handle
- [ ] pypi.org/user/wattlint — reserve for future Python SDK

---

## Part 1 — What WattLint Is: The Platform Vision

WattLint is a **platform**, not a single tool. Two products share one backend, one dashboard, one billing system, one GitHub App.

```
┌─────────────────────────────────────────────────────────────┐
│                    wattlint.com platform                     │
├──────────────────────────┬──────────────────────────────────┤
│  WattLint CI             │  WattLint GPU                    │
│  (ships Month 1–3)       │  (ships Month 4–9)               │
│                          │                                  │
│  watt-sampler CLI (Rust) │  gpu-tracer CLI (Rust)           │
│  → GitHub Action         │  → GitHub Action                 │
│  → Energy regression PR  │  → Kernel trace PR annotation    │
│    comment               │  → Watt-per-token dashboard      │
│  → Energy trend chart    │                                  │
├──────────────────────────┴──────────────────────────────────┤
│              Shared platform backend                         │
│  Go API (Fly.io) → Neon PostgreSQL + Upstash Redis          │
│  Next.js dashboard (Vercel) → GitHub App webhooks           │
│  Stripe billing → shared across both products               │
└─────────────────────────────────────────────────────────────┘
```

The expansion motion: teams install WattLint CI first because it has zero friction (one GitHub Action line). When GPU observability ships, the same teams get a new tab in the dashboard they already use — no new sign-up, no new billing relationship. This is how Datadog grew: land with APM, expand to Logs, then Infrastructure.

---

## Part 2 — Technology Stack

Every choice below is final and justified. Do not substitute without a written reason.

| Layer | Choice | Reason |
|---|---|---|
| Core energy sampler | **Rust (stable 1.78+)** | `perf_event_open` via `perf-event` crate is the fastest, lowest-variance RAPL mechanism (arxiv:2401.15985 confirms this). No GC pauses that contaminate measurements. Single static binary — users install by downloading one file. Memory safety eliminates the silent overflow bugs that plague every existing RAPL tool. |
| GitHub Action | **TypeScript (Node 20, strict mode)** | GitHub Actions is TypeScript-native. `@actions/core`, `@actions/cache`, `@actions/tool-cache` are first-class. Compiles to a single `dist/index.js` — zero runtime install required for end users. |
| SaaS API backend | **Go 1.23** | Best fit for a persistent long-running HTTP server with a connection pool to Neon. Critical: Vercel (serverless) cannot maintain a pgx connection pool — each cold start creates a new connection, rapidly exhausting Neon's connection limit. Fly.io runs an actual VM. Go binary starts once, pgx pool stays warm. |
| Database | **Neon PostgreSQL 16** (Pro plan, already owned) | Standard PostgreSQL with zero incremental cost. The TimescaleDB hypertable partitioning is replaced by a composite index + a Go cron aggregator. Continuous aggregate views are replaced by an hourly Go job that materialises `measurements_daily`. Migrate to TimescaleDB only when `measurements` table exceeds ~50M rows — not before. |
| Cache / rate-limit | **Upstash Redis** (already owned) | `redis/go-redis/v9` API is identical to self-hosted Redis. Zero code change. PR comment deduplication and rate limiting are low-frequency, short-lived key operations — exactly what Upstash serverless is optimised for. Free tier covers early growth easily. |
| Dashboard frontend | **Next.js 15 + TypeScript** | App Router + server components for data-heavy dashboard pages. Recharts for energy trend charts. Deployed to Vercel free tier — zero-config deploys, ISR for dashboard caching, edge network. |
| API deployment | **Fly.io** | Persistent VM. pgx connection pool survives between requests. ~$1.94/month for 256MB shared-CPU VM. Fly free tier (3 VMs) covers dev + staging + prod during MVP phase. |
| Dashboard deployment | **Vercel free tier** | Natural home for Next.js 15 App Router. Zero cost during MVP. |
| Monorepo tooling | **Turborepo + pnpm workspaces** | Parallel builds across apps. Only rebuilds what changed. `turbo run build --filter=apps/api` builds just the Go API. |

**Cost at launch: ~$0–5/month total.** Neon Pro (owned), Upstash (owned), Fly.io free tier (3 VMs), Vercel free tier.

---

## Part 3 — Monorepo Structure

One **private** monorepo: `github.com/wattlint/platform`
Two **public** OSS mirrors synced via `git subtree`:
- `github.com/wattlint/watt-sampler` — the Rust CLI (MIT licence)
- `github.com/wattlint/gpu-tracer` — the GPU tracer CLI (MIT licence, Phase 2)

The OSS tools drive GitHub stars and developer trust. The platform drives revenue. This is the open-core model used by PlanetScale, Turso, and Neon themselves.

```
wattlint/                                  ← private (github.com/wattlint/platform)
│
├── apps/
│   ├── api/                               ← Go 1.23 — single API for ALL WattLint products
│   │   ├── Dockerfile
│   │   ├── go.mod
│   │   ├── cmd/
│   │   │   └── api/
│   │   │       └── main.go                ← server entrypoint
│   │   └── internal/
│   │       ├── api/
│   │       │   ├── measurements.go        ← POST /api/v1/measurements (CI + GPU)
│   │       │   ├── repos.go               ← repo registration, baselines
│   │       │   ├── billing.go             ← Stripe webhook + plan enforcement
│   │       │   └── middleware.go          ← auth, rate-limit, request-id
│   │       ├── db/
│   │       │   ├── queries.go             ← pgx/v5 parameterised queries
│   │       │   ├── daily_agg.go           ← hourly cron: materialize measurements_daily
│   │       │   └── migrations/
│   │       │       ├── 001_initial.up.sql
│   │       │       ├── 001_initial.down.sql
│   │       │       ├── 002_gpu_tables.up.sql     ← added when GPU ships
│   │       │       └── 002_gpu_tables.down.sql
│   │       ├── github/
│   │       │   ├── app.go                 ← GitHub App JWT + installation tokens
│   │       │   └── comments.go            ← PR comment search-and-update (never duplicate)
│   │       ├── alerts/
│   │       │   ├── slack.go
│   │       │   └── email.go
│   │       └── cron/
│   │           └── aggregator.go          ← replaces TimescaleDB continuous aggregates
│   │
│   ├── dashboard/                         ← Next.js 15 — single dashboard for ALL products
│   │   ├── app/
│   │   │   ├── layout.tsx
│   │   │   ├── (auth)/
│   │   │   │   ├── login/page.tsx         ← GitHub OAuth
│   │   │   │   └── callback/page.tsx
│   │   │   ├── dashboard/
│   │   │   │   └── [owner]/[repo]/
│   │   │   │       ├── page.tsx           ← repo overview: energy tab + GPU tab (Phase 2)
│   │   │   │       ├── pr/[number]/page.tsx
│   │   │   │       └── settings/page.tsx
│   │   │   └── api/                       ← thin Next.js proxy routes to Go API
│   │   └── components/
│   │       ├── EnergyTrendChart.tsx       ← Recharts ComposedChart with P5-P95 band
│   │       ├── DomainBreakdownChart.tsx   ← PKG vs DRAM vs GPU bar chart
│   │       ├── PRComparisonTable.tsx
│   │       ├── NoiseScatterPlot.tsx       ← all 7 iteration dots for trust-building
│   │       └── GPUKernelTrace.tsx         ← Phase 2, stubbed now
│   │
│   └── docs/                              ← Docusaurus — wattlint.com/docs
│       ├── docs/
│       │   ├── getting-started.md
│       │   ├── ci/
│       │   │   ├── quickstart.md
│       │   │   ├── self-hosted-runners.md
│       │   │   ├── github-hosted-runners.md
│       │   │   └── noise-reduction.md
│       │   └── gpu/                       ← Phase 2, placeholder
│       └── docusaurus.config.ts
│
├── packages/
│   ├── watt-sampler/                      ← PUBLIC OSS: github.com/wattlint/watt-sampler
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                    ← CLI entrypoint (clap)
│   │       ├── rapl.rs                    ← perf_event_open → powercap → MSR fallback chain
│   │       ├── amd_rapl.rs                ← /sys/devices/platform/amd_energy/
│   │       ├── gpu.rs                     ← NVML + AMD hwmon (no nvidia-smi subprocess)
│   │       ├── normalizer.rs              ← EES: joules / (tdp_watts × duration_s)
│   │       ├── runner.rs                  ← subprocess exec + CPU affinity + overflow guard
│   │       ├── report.rs                  ← JSON serialiser (stdout only, errors to stderr)
│   │       └── diff.rs                    ← baseline comparison + threshold check
│   │
│   └── gpu-tracer/                        ← PUBLIC OSS: github.com/wattlint/gpu-tracer
│       ├── Cargo.toml                     ← stubbed now, built in Phase 2
│       └── src/
│           ├── main.rs
│           ├── cupti.rs                   ← CUPTI + eBPF (Polar Signals parcagpu approach)
│           ├── otel.rs                    ← OpenTelemetry span export
│           └── report.rs
│
├── actions/
│   ├── energy-gate/                       ← GitHub Action: wattlint/energy-gate@v1
│   │   ├── action.yml
│   │   ├── package.json
│   │   └── src/
│   │       ├── index.ts
│   │       ├── installer.ts               ← download + SHA256 verify watt-sampler binary
│   │       ├── baseline.ts                ← @actions/cache: get or create main-branch baseline
│   │       ├── runner.ts                  ← execute watt-sampler, parse JSON
│   │       ├── reporter.ts                ← build PR comment markdown
│   │       └── uploader.ts                ← POST to SaaS API (optional, needs token)
│   │
│   └── gpu-profile/                       ← Phase 2: wattlint/gpu-profile@v1
│       ├── action.yml
│       └── src/
│
├── infra/
│   ├── fly/
│   │   └── fly.toml                       ← Fly.io: wattlint-api app, region iad
│   └── vercel/
│       └── vercel.json                    ← root dir: apps/dashboard
│
├── .github/
│   └── workflows/
│       ├── release-watt-sampler.yml       ← cross-compile linux-amd64 + linux-arm64, publish release
│       ├── release-gpu-tracer.yml         ← Phase 2
│       ├── deploy-api.yml                 ← flyctl deploy on push to main
│       └── deploy-dashboard.yml           ← vercel --prod on push to main
│
├── turbo.json                             ← Turborepo pipeline config
└── package.json                           ← pnpm workspace root
```

---

## Part 4 — Database: Neon PostgreSQL

### 4.1 Why Neon, Not TimescaleDB

TimescaleDB hypertables partition by time for fast range queries. A regular PostgreSQL table with the right composite index achieves the same query plan at our scale. The breakeven is ~50M rows — we're not there yet. When we get there, the migration is Postgres→Postgres with a schema tweak, not a platform change.

TimescaleDB continuous aggregates are replaced by `apps/api/internal/cron/aggregator.go` — an hourly goroutine that computes daily rollups and upserts into `measurements_daily`. This is ~60 lines of Go.

### 4.2 Schema

```sql
-- Registered repositories
CREATE TABLE repos (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id       BIGINT UNIQUE NOT NULL,
    full_name       TEXT NOT NULL,           -- "owner/repo"
    api_token_hash  TEXT NOT NULL,           -- bcrypt(token, cost=12), never plaintext
    plan            TEXT DEFAULT 'free',     -- free | pro | enterprise
    stripe_customer TEXT,                    -- Stripe customer ID
    default_branch  TEXT DEFAULT 'main',
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Raw CI energy measurements (watt-sampler JSON output)
CREATE TABLE measurements (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    time                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    repo_id             UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    commit_sha          CHAR(40) NOT NULL,
    branch              TEXT NOT NULL,
    pr_number           INT,                 -- NULL for main-branch baselines
    benchmark_cmd_hash  TEXT NOT NULL,       -- sha256(benchmark_command) for grouping
    benchmark_cmd       TEXT NOT NULL,
    is_baseline         BOOLEAN DEFAULT FALSE,
    runner_type         TEXT NOT NULL,       -- 'rapl-perf-event' | 'rapl-powercap' | 'estimated'

    -- CPU domain measurements (all nullable — not every domain available everywhere)
    pkg_joules_median   DOUBLE PRECISION,
    pkg_joules_p5       DOUBLE PRECISION,
    pkg_joules_p95      DOUBLE PRECISION,
    dram_joules_median  DOUBLE PRECISION,
    pp0_joules_median   DOUBLE PRECISION,
    cpu_avg_watts       DOUBLE PRECISION,
    cpu_vendor          TEXT NOT NULL,       -- 'GenuineIntel' | 'AuthenticAMD'
    cpu_model           TEXT NOT NULL,
    cpu_tdp_watts       INT,

    -- GPU measurements (nullable if no GPU present)
    gpu_joules_median   DOUBLE PRECISION,
    gpu_avg_watts       DOUBLE PRECISION,
    gpu_name            TEXT,

    -- Summary
    total_joules_median DOUBLE PRECISION NOT NULL,
    total_joules_p95    DOUBLE PRECISION,
    ees                 DOUBLE PRECISION,    -- Energy Efficiency Score (dimensionless)
    noise_pct           DOUBLE PRECISION,
    duration_s          DOUBLE PRECISION,
    iterations          INT DEFAULT 7,
    valid               BOOLEAN DEFAULT TRUE,
    warnings            TEXT[],
    raw_json            JSONB               -- full watt-sampler JSON for debugging
);

-- Critical indexes (replace TimescaleDB hypertable partitioning)
-- Baseline lookups: latest valid baseline for a given repo + benchmark + CPU vendor
CREATE INDEX idx_measurements_baseline
    ON measurements (repo_id, benchmark_cmd_hash, cpu_vendor, time DESC)
    WHERE is_baseline = TRUE AND valid = TRUE;

-- Time-range scans per repo (dashboard trend queries)
CREATE INDEX idx_measurements_repo_time
    ON measurements (repo_id, time DESC);

-- PR detail lookups
CREATE INDEX idx_measurements_pr
    ON measurements (repo_id, pr_number, time DESC)
    WHERE pr_number IS NOT NULL;

-- Pre-computed daily rollups (populated by Go cron, not TimescaleDB)
-- The cron job runs every hour and upserts only new data
CREATE TABLE measurements_daily (
    day                 DATE NOT NULL,
    repo_id             UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    benchmark_cmd_hash  TEXT NOT NULL,
    branch              TEXT NOT NULL,
    avg_total_joules    DOUBLE PRECISION,
    min_total_joules    DOUBLE PRECISION,
    max_total_joules    DOUBLE PRECISION,
    avg_ees             DOUBLE PRECISION,
    measurement_count   INT,
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (day, repo_id, benchmark_cmd_hash, branch)
);

-- PR regression/improvement alerts
CREATE TABLE alerts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id       UUID NOT NULL REFERENCES repos(id),
    pr_number     INT NOT NULL,
    commit_sha    CHAR(40),
    delta_pct     DOUBLE PRECISION NOT NULL,
    threshold_pct DOUBLE PRECISION NOT NULL,
    alert_type    TEXT NOT NULL,   -- 'regression' | 'improvement'
    channels      TEXT[],          -- ['github_pr', 'slack', 'email']
    created_at    TIMESTAMPTZ DEFAULT NOW()
);

-- Phase 2: GPU kernel traces (added via migration 002_gpu_tables.up.sql)
CREATE TABLE gpu_traces (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    time                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    repo_id               UUID NOT NULL REFERENCES repos(id),
    commit_sha            CHAR(40) NOT NULL,
    pr_number             INT,
    inference_request_id  TEXT,              -- correlates to LLM request if applicable
    kernel_name           TEXT NOT NULL,
    sm_efficiency_pct     DOUBLE PRECISION,
    memory_bandwidth_gbps DOUBLE PRECISION,
    duration_us           DOUBLE PRECISION,
    energy_uj             DOUBLE PRECISION,
    stall_reason_pct      JSONB,             -- {"memory_dependency": 34.2, "sync": 12.1}
    raw_json              JSONB
);

CREATE INDEX idx_gpu_traces_repo_time
    ON gpu_traces (repo_id, time DESC);
```

### 4.3 Daily Aggregator (Go cron — replaces TimescaleDB continuous aggregates)

```go
// apps/api/internal/cron/aggregator.go
//
// Runs every 1 hour via time.Ticker in a background goroutine started at server boot.
// Tracks last_processed_time in a simple key-value table (cron_state).
// Queries: SELECT date_trunc('day', time), repo_id, benchmark_cmd_hash, branch,
//          AVG(total_joules_median), MIN(...), MAX(...), AVG(ees), COUNT(*)
//          FROM measurements
//          WHERE time > last_processed AND valid = TRUE
//          GROUP BY 1, 2, 3, 4
// Upserts into measurements_daily using ON CONFLICT (day, repo_id, ...) DO UPDATE
// Runs in ~100ms on Neon for typical data volumes.
// Zero TimescaleDB dependency. Same query runs on any PostgreSQL instance.
```

---

## Part 5 — Infrastructure

### 5.1 Upstash Redis — two usage patterns

```go
// Rate limiting (sliding window log pattern)
// Key: "ratelimit:{sha256(api_token)[:16]}"
// ZADD currentTimestamp → ZREMRANGEBYSCORE old entries → ZCARD → reject at 100/min
// TTL on key: 2 minutes (auto-cleanup)

// PR comment deduplication
// Key: "comment:{repo_id}:{pr_number}"
// Value: GitHub comment ID (integer as string)
// TTL: 90 days (matches free plan history retention)
// On new measurement:
//   GET key → if exists: PATCH existing comment (no new comment)
//   if not exists: POST new comment → SET key with TTL
```

Both patterns are well within Upstash free tier (10,000 commands/day). A repo with 200 active PRs receiving one energy measurement per day = 400 Redis ops/day.

### 5.2 Fly.io — Go API

```toml
# infra/fly/fly.toml
app = "wattlint-api"
primary_region = "iad"  # US East — closest to GitHub webhook origin servers

[build]
  dockerfile = "apps/api/Dockerfile"

[env]
  PORT = "8080"
  # Secrets (set via: fly secrets set KEY=VALUE):
  # NEON_DATABASE_URL, UPSTASH_REDIS_URL, GITHUB_APP_ID,
  # GITHUB_APP_PRIVATE_KEY, GITHUB_WEBHOOK_SECRET, STRIPE_SECRET_KEY

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

**Go multi-stage Dockerfile — ~11MB final image:**
```dockerfile
FROM golang:1.23-alpine AS builder
WORKDIR /app
COPY apps/api/go.mod apps/api/go.sum ./
RUN go mod download
COPY apps/api/ .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /wattlint-api ./cmd/api

FROM scratch
COPY --from=builder /wattlint-api /wattlint-api
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
EXPOSE 8080
ENTRYPOINT ["/wattlint-api"]
```

Cost breakdown: Fly.io free tier = 3 shared VMs free forever. This app uses 1. Staging uses 1. Leaves 1 spare. **$0/month until you need a dedicated CPU.**

### 5.3 Vercel — Next.js Dashboard

Connect `wattlint/platform` monorepo to Vercel. Root directory: `apps/dashboard`. Framework preset: Next.js. Environment variable: `NEXT_PUBLIC_API_URL=https://api.wattlint.com`. Vercel handles the rest — edge network, ISR, preview deployments on every PR. **$0/month on free tier for MVP.**

---

## Part 6 — Phase 0: Core Sampler (Week 1–2)

### 6.1 RAPL Access Chain

Never go straight to MSR (requires root). Always try in this order:

```
1. perf_event_open (preferred — no root if paranoid ≤ 2)
   Check: /sys/bus/event_source/devices/power/events/energy-pkg exists
   Use: perf-event Rust crate
   Domains: energy-pkg, energy-cores, energy-ram, energy-psys (all optional except pkg)

2. sysfs powercap (fallback — Linux 3.13+)
   Read: /sys/class/powercap/intel-rapl/intel-rapl:*/energy_uj (microjoules)
   Note: Linux 5.10+ restricts to CAP_SYS_ADMIN — detect and warn if permission denied

3. AMD RAPL (parallel path — detect by AuthenticAMD in /proc/cpuinfo)
   Read: /sys/devices/platform/amd_energy/energy*_input (microjoules)
   Available: Zen+ and newer only

4. MSR (last resort — requires root + modprobe msr)
   Offsets: PKG=0x611, PP0=0x639, DRAM=0x619, PSYS=0x64D
   Unit: decode from MSR_RAPL_POWER_UNIT (0x606) bits [12:8]

5. None available: set runner_type="estimated", estimate via CPU time × TDP model
   Clearly mark all output as ESTIMATED — never present estimates as real measurements
```

### 6.2 Overflow Handling (mandatory)

```rust
// RAPL counters are 32-bit. At 84W draw they overflow every ~52 minutes.
// Sample at 100ms intervals to catch every overflow.
// Monotonic accumulator using u128 to never overflow the accumulator itself:

struct RaplAccumulator {
    last_raw: u64,
    cumulative_uj: u128,
}

impl RaplAccumulator {
    fn update(&mut self, new_raw: u64) {
        if new_raw < self.last_raw {
            // Counter wrapped: add distance from last to MAX, then from 0 to new
            self.cumulative_uj += (u32::MAX as u128 - self.last_raw as u128)
                                  + new_raw as u128 + 1;
        } else {
            self.cumulative_uj += new_raw - self.last_raw;
        }
        self.last_raw = new_raw;
    }
}
```

### 6.3 GPU Reader

```rust
// NVIDIA: nvml::Nvml::init() once at startup
// device.power_usage()? → milliwatts (NOT watts)
// Sample at 100ms, average 10 readings per iteration
// Multi-GPU: report per-device AND sum total
// NEVER call nvidia-smi as subprocess — 150ms+ startup per call kills accuracy

// AMD GPU:
// Enumerate /sys/class/drm/card*/device/hwmon/hwmon*/power1_average
// Read microwatts, convert to milliwatts for consistent units

// Neither available: gpu_joules_median = null
// This is a warning (log to stderr), not an error (do not exit non-zero)
```

### 6.4 Normalisation

```
Energy Efficiency Score (EES) — dimensionless, cross-machine comparable within CPU vendor

EES = measured_joules / (cpu_tdp_watts × duration_s)

EES < 1.0 → more efficient than TDP-normalised expectation
EES > 1.0 → less efficient

TDP source (in order of preference):
  1. /sys/devices/virtual/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw
     (actual configured TDP in microwatts — not marketing TDP)
  2. /sys/class/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw
  3. Parse from /proc/cpuinfo model name against known TDP database (fallback)

Store EES, not raw joules, for cross-PR comparison.
PR comments show: "This PR uses 23% more energy per operation (EES: 1.23 vs 1.00)"
```

### 6.5 Noise Reduction

```
Before each benchmark iteration:
  1. Warn if CPU governor != "performance"
     Check: /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
     Do NOT force-change — warn only. Developers may not have permissions.

  2. CPU affinity: pin child process to cores 2..N-1
     Avoids core 0 (OS interrupt handler, timer interrupts)
     Uses nix::sched::sched_setaffinity on the child PID post-spawn

  3. Background load check: read /proc/stat before each iteration
     If other-process CPU utilization > 5% → add warning to output

  4. Thermal throttling detection:
     If measured_avg_watts > cpu_tdp_watts × 0.95 → warn "thermal throttling detected"
     Results may be inflated due to power capping

Iteration strategy:
  - Run warmup_iters (default: 2) — discarded, not measured
  - Run measurement_iters (default: 7) — recorded
  - Report: median (not mean — robust to OS jitter outliers)
  - Report: p5, p95 for uncertainty visualisation
  - noise_pct = (p95 - p5) / median × 100
  - If noise_pct > 15% → set valid=false, add to warnings[]
  - If noise_pct > 20% AND --force not set → exit(1)

Cross-measurement validity rules (enforced in watt-sampler diff):
  REFUSE comparison if:
    - cpu_vendor differs (Intel vs AMD baseline comparison is invalid)
    - either measurement has valid=false
    - runner_type differs (real RAPL vs estimated — incompatible)
  WARN if:
    - cpu_model differs (same vendor, different model — EES comparison is approximate)
    - measurement age > 7 days (baseline may be stale)
    - GPU present in one but not the other
```

### 6.6 CLI Output Contract

```bash
# All commands follow this contract:
# stdout: JSON only (or human-readable if --output human)
# stderr: all warnings, errors, progress messages
# exit 0: success, measurement valid
# exit 1: measurement invalid OR regression detected (when using diff --fail)
# exit 2: hardware error (no RAPL available, required binary missing)

watt-sampler run \
  --iterations 7 \
  --warmup 2 \
  --domains pkg,dram,gpu \
  --output json \
  -- ./target/release/benchmark --time 5s

watt-sampler diff \
  --baseline ./baseline.json \
  --current ./current.json \
  --threshold 10 \
  --output markdown
  # exits 1 if regression > threshold

watt-sampler check-hardware
  # Reports: RAPL method available, CPU model, TDP, GPU detected
  # Exits 0 if real measurement possible, 2 if estimation only
```

---

## Part 7 — Phase 1: GitHub Action — energy-gate (Week 3–4)

### 7.1 action.yml

```yaml
name: 'WattLint Energy Gate'
description: 'Block PRs that increase energy consumption beyond your threshold'
branding:
  icon: 'zap'
  color: 'yellow'
inputs:
  benchmark-command:
    description: 'Command to run your benchmark. Must be deterministic.'
    required: true
  threshold-pct:
    description: 'Fail PR if energy increases by more than this % vs baseline'
    default: '10'
  iterations:
    description: 'Measurement iterations (minimum 5 for statistical validity)'
    default: '7'
  domains:
    description: 'RAPL domains: pkg,dram,gpu (comma-separated, default: pkg,gpu)'
    default: 'pkg,gpu'
  api-token:
    description: 'WattLint API token for SaaS dashboard (optional for free use)'
    required: false
  fail-on-regression:
    description: 'Block PR merge on energy regression (default: true)'
    default: 'true'
outputs:
  energy-delta-pct:
    description: 'Signed percentage change vs baseline (negative = improvement)'
  regression-detected:
    description: '"true" if energy regressed beyond threshold'
  baseline-joules:
    description: 'Baseline median total joules'
  current-joules:
    description: 'Current median total joules'
runs:
  using: 'node20'
  main: 'dist/index.js'
```

### 7.2 Action Execution Flow

```
Step 1 — installer.ts
  Detect runner arch: process.arch → linux-amd64 | linux-arm64
  Download watt-sampler from:
    https://github.com/wattlint/watt-sampler/releases/download/v{VERSION}/watt-sampler-{ARCH}
  MANDATORY: verify SHA256 before chmod +x. Reject binary if checksum mismatch.
  Cache with @actions/tool-cache to avoid re-download.
  Run `watt-sampler check-hardware` → detect RAPL availability.
  If RAPL unavailable on GitHub-hosted runner: log warning, continue with estimation.

Step 2 — baseline.ts
  Cache key: sha256(repo_full_name + ":" + default_branch + ":" + benchmark_command)
  Check @actions/cache for existing baseline JSON.
  If cache miss OR age > 7 days:
    Checkout main branch to temp directory.
    Install dependencies (same commands as PR branch — detected from package.json/Cargo.toml).
    Build project.
    Run watt-sampler on main branch code.
    Store in @actions/cache.
  Return baseline JSON.

Step 3 — runner.ts
  Run watt-sampler on current PR code (already built by CI at this point).
  Parse and validate JSON from stdout.
  Validate: schema_version == "1", valid == true, all required fields present.

Step 4 — reporter.ts
  Compute delta = (current_median - baseline_median) / baseline_median × 100
  Determine domain-level deltas (PKG, DRAM, GPU separately)
  Build markdown comment (see §7.3 format)
  Search PR comments for HTML marker: <!-- wattlint-report -->
  PATCH existing comment if found. POST new comment if not.
  NEVER post duplicate comments — this will cause developers to disable the Action.

Step 5 — uploader.ts (if api-token provided)
  POST /api/v1/measurements with full JSON + PR metadata
  Failure here is a warning, NOT a fatal error — measurement already happened.

Step 6 — gate
  Set all output variables via core.setOutput
  If delta > threshold AND fail-on-regression == 'true':
    core.setFailed(`⚡ WattLint: Energy regression +${delta.toFixed(1)}% vs baseline (threshold: ${threshold}%)`)
```

### 7.3 PR Comment Format

```markdown
<!-- wattlint-report -->
## ⚡ WattLint Energy Report

| Domain | Baseline (main) | This PR | Δ |
|--------|----------------|---------|---|
| CPU Package (median) | 412.3 J | 507.1 J | 🔴 +23.0% |
| DRAM (median) | 28.1 J | 27.9 J | ✅ −0.7% |
| GPU (median) | 1,840.2 J | 1,839.8 J | ✅ −0.02% |
| **Total (median)** | **2,280.6 J** | **2,374.8 J** | 🔴 **+4.1%** |
| Avg CPU Power | 85.3 W | 104.9 W | 🔴 +23.0% |
| Benchmark duration | 4.83 s | 4.84 s | ✅ +0.2% |

**Threshold:** 10% · **Status:** 🔴 REGRESSION DETECTED (CPU Package +23.0%)

> 💡 CPU energy rose 23% with only 0.2% longer runtime. This pattern typically indicates
> increased cache misses, branch mispredictions, or vectorisation regression.
> Runtime held steady but the CPU is working harder per operation.

<details>
<summary>All 7 iterations</summary>

| Iter | Baseline (J) | Current (J) |
|------|-------------|-------------|
| 1    | 410.1       | 505.3       |
| 2    | 412.3       | 508.1       |
...

</details>

<details>
<summary>Environment</summary>
CPU: Intel Xeon Gold 6226R · TDP: 150 W · GPU: NVIDIA A100 · Measurement: RAPL perf_event · Iterations: 7 (warmup: 2) · Noise: 1.4%
</details>

[View full trend → wattlint.com/dashboard/owner/repo](https://wattlint.com/dashboard/owner/repo)
```

---

## Part 8 — Phase 2: SaaS Backend (Week 5–8)

### 8.1 API Routes

```
POST   /api/v1/measurements          Receive watt-sampler JSON, store, trigger PR comment
GET    /api/v1/repos/:id/baseline    Latest valid baseline for a benchmark+cpu_vendor
GET    /api/v1/repos/:id/trend       Daily rollup data for dashboard Recharts chart
GET    /api/v1/repos/:id/prs/:pr     Full PR measurement + all iterations
POST   /api/v1/repos                 Register new repo, return API token
POST   /webhooks/github              GitHub App webhooks (HMAC-SHA256 verified)
GET    /health                       Fly.io health check
```

### 8.2 Go Dependencies

```go
require (
    github.com/go-chi/chi/v5        v5.0.12
    github.com/jackc/pgx/v5         v5.5.5
    github.com/redis/go-redis/v9    v9.5.1
    github.com/google/go-github/v60 v60.0.0
    golang.org/x/crypto             v0.21.0   // bcrypt for token hashing
    github.com/stripe/stripe-go/v76 v76.0.0   // billing
)
```

### 8.3 Critical: Baseline Selection

```go
// Never compare measurements across CPU vendors.
// Never serve a stale baseline without a warning.
// Prefer the most recent valid main-branch measurement that matches:
//   1. Same repo_id
//   2. Same benchmark_cmd_hash
//   3. Same cpu_vendor
//   4. Same runner_type (rapl vs estimated — do not mix)
// If no matching baseline: return 404, action will build one from main branch.

func GetBaseline(ctx context.Context, repoID, cmdHash, cpuVendor, runnerType string) (*Measurement, error)
```

---

## Part 9 — Phase 3: Dashboard (Week 9–12)

### 9.1 Page Structure

**`/dashboard/[owner]/[repo]` — Repo Overview**
- Tab 1: Energy (WattLint CI)
  - Line chart: total_joules_median over time, per branch
  - Shaded band: P5–P95 uncertainty envelope
  - Scatter annotations: red dot = regression PR, green = improvement
  - Summary cards: regressions this month, energy trend (+/- % vs 30-day avg)
- Tab 2: GPU (WattLint GPU — Phase 2, shown greyed out with "coming soon")

**`/dashboard/[owner]/[repo]/pr/[number]` — PR Detail**
- Bar chart: PKG vs DRAM vs GPU — baseline vs PR (grouped bars, clear delta labels)
- Scatter plot: all 7 iterations for both baseline and PR (shows noise visually)
- Energy advisor text box (rule-based analysis — see §9.2)
- Commit history: energy readings across all commits in this PR

**`/dashboard/[owner]/[repo]/settings` — Configuration**
- Threshold % (per benchmark command, default 10%)
- Alert channels: Slack webhook URL, email address
- Excluded branches
- Plan management (upgrade to Pro/Enterprise)

### 9.2 Energy Advisor Rules

```
Rule 1: CPU energy up >15%, GPU energy stable, duration stable
  → "CPU is working harder per operation without doing more work.
     Likely cause: cache misses, branch mispredictions, or vectorisation regression.
     Try: perf stat -e cache-misses,branch-misses ./benchmark"

Rule 2: DRAM energy up >15%, CPU energy stable
  → "Memory bandwidth increased significantly.
     Likely cause: more allocations, worse cache locality, or data layout change.
     Try: valgrind --tool=massif or heaptrack to profile allocations."

Rule 3: GPU energy up >10%, CPU energy stable
  → "GPU workload increased — more compute or more memory transfers.
     Check for: changed batch sizes, reduced kernel fusion, or extra data copies."

Rule 4: All domains up proportionally, duration up proportionally
  → "Energy increase is proportional to runtime increase.
     This is expected for a slower algorithm — not a power efficiency regression."

Rule 5: All domains up, duration unchanged
  → "Same runtime but more watts — power density increased.
     The code is running hotter. This is the most impactful type of regression."
```

---

## Part 10 — Phase 4: WattLint GPU (Month 4–9)

This product lives in the **same platform**, same dashboard, same Go API. It adds:
- `packages/gpu-tracer/` Rust CLI (public OSS)
- `actions/gpu-profile/` GitHub Action
- `002_gpu_tables.up.sql` migration
- New tab in existing dashboard

### 10.1 What It Solves

The gap identified in the research: there is no tool that connects the LLM API layer (tokens, cost per request) to the GPU hardware layer (which kernels fired, at what efficiency, consuming how many watts). Langfuse knows about tokens. Nsight knows about kernels. Nobody bridges them.

WattLint GPU bridges them. For each inference request (tagged with a correlation ID), it shows:
- Which CUDA kernels executed
- SM efficiency per kernel
- Memory bandwidth utilisation
- Energy in microjoules
- **Watt-per-token** as a first-class metric

### 10.2 Technical Approach

Build on Polar Signals's `parcagpu` approach (published October 2025):
- CUPTI profiling API for kernel timing
- USDT probes as the CPU-to-GPU bridge
- eBPF for data collection at <4% overhead
- Correlation IDs match API requests to GPU traces

Integrate with vLLM and TGI via their OpenTelemetry spans — bolt-on, no server modification required.

### 10.3 Target Customer

Any team running self-hosted inference (vLLM, TGI, TensorRT-LLM) who wants to know:
1. Which model checkpoint is most energy-efficient for their workload
2. Which features of their product are driving GPU spend
3. Whether a prompt engineering change saved or cost GPU energy

---

## Part 11 — Pricing

```
FREE (forever):
  - Unlimited measurements
  - Self-hosted runners only (real RAPL access)
  - 7-day history
  - PR comments (energy regression gate)
  - 1 private repo, unlimited public repos
  - watt-sampler CLI always free and open-source

PRO ($29/month per organisation):
  - Unlimited repos
  - 90-day history + trend charts on dashboard
  - GitHub-hosted runner support (cloud estimation model)
  - Slack + email alerts
  - Custom threshold per benchmark command
  - WattLint GPU when it ships (included in Pro)

ENTERPRISE ($199/month):
  - 2-year history
  - SSO (SAML/OIDC)
  - SLA (99.9% uptime)
  - On-premise deployment (self-host the Go API + dashboard)
  - Dedicated Slack support channel
  - Custom integrations (Jira, PagerDuty)
```

---

## Part 12 — Go-to-Market & Marketing

### 12.1 Pre-Launch (2 weeks before shipping)

**Content:**
Write these 3 blog posts and schedule them before launch. They do double duty as SEO content and launch ammunition:

1. `"Why Jensen Huang's 'performance per watt' quote should be in your CI pipeline"` — references actual NVIDIA earnings call transcript, Herb Sutter's C++ article, the power constraint research. This is the top-of-funnel piece. Target keywords: "CI energy profiling", "energy regression testing", "watt per token".

2. `"We found code that uses 39× more energy than it should — and CI would never catch it"` — references the arxiv paper on LLM-generated code energy variance. Specific, alarming, data-backed. Target developer pain directly.

3. `"The RAPL guide no one wrote: how to accurately measure CPU energy in production"` — deep technical post on the perf_event_open path, overflow handling, AMD vs Intel differences. This builds credibility with the Rust/C++ community who will be the early adopters.

**GitHub:**
- Create `github.com/wattlint/watt-sampler` as a public repo
- Write an excellent README with a "30-second demo" GIF (record terminal: run watt-sampler, show JSON output)
- Add to GitHub Marketplace as a GitHub Action
- Post to awesome-rust and awesome-ci lists

### 12.2 Launch Week

**Hacker News:** "Show HN: WattLint — energy regression testing for CI, like ESLint but for watts"
- Post Tuesday 9am ET (peak engagement)
- First comment (post it yourself): share the blog post about 39× energy variance in LLM-generated code
- Second comment: link to a live demo on a public repo showing a real energy regression caught

**Reddit:**
- r/rust: "I built an energy profiler in Rust that uses perf_event_open — here's how RAPL actually works" (the technical post)
- r/programming: the 39× variance post
- r/devops: "Catch energy regressions in PRs before they ship — like a linter but for watts"

**X/Twitter:**
- Tag @herbsutter on the post referencing his article
- Thread: 5 tweets showing the problem (AI code energy variance) → the solution (watt-sampler run) → the PR comment output → the dashboard chart
- No product screenshots in the first tweet — lead with the data, reveal the product

**Dev.to + Hashnode:**
Publish the 3 blog posts cross-posted. Tag: `#rust`, `#devops`, `#sustainability`, `#cicd`, `#github-actions`

### 12.3 Growth Flywheel

```
Phase 1 (Month 1–2): Community seeding
  Target: Rust open-source maintainers (ripgrep, tokio, axum, etc.)
  Why: They care most about performance per watt. They have big communities.
  Action: File issues/PRs to their repos showing energy regressions found with WattLint.
  Goal: 1 major Rust project adopts WattLint → 500+ stars happen overnight.

Phase 2 (Month 2–3): Integration push
  Add first-class support for criterion.rs benchmarks (the standard Rust benchmarking tool)
  Add first-class support for cargo-bench
  Add Python: pytest-benchmark integration
  Add Node.js: benchmark.js integration
  Each integration = blog post = SEO content = community post.

Phase 3 (Month 3–6): Enterprise motion
  Target: companies spending >$50k/month on cloud GPU
  Message: "WattLint GPU shows you exactly which features and users are driving your GPU bill"
  Channel: LinkedIn posts, direct outreach to AI infra engineers
  Trial: 30-day free Pro trial, no credit card

Phase 4 (Month 6+): Content SEO
  Target: "CI energy monitoring", "energy regression testing", "watt per token LLM"
  These are low-competition keywords today — nobody owns them yet
  Publish: monthly "State of Software Energy" report using anonymised aggregated data
  (% of PRs causing regressions, avg energy delta by language, etc.)
  This report becomes a citation magnet and link-building asset.
```

### 12.4 Distribution Channels Ranked by Priority

| Channel | Why | Action |
|---|---|---|
| GitHub Marketplace | 90M+ developers, where the product lives | Submit Action immediately on launch |
| Hacker News Show HN | Where Rust/C++/systems devs hang out | One shot — make it count with the right framing |
| r/rust | Highest-quality early adopters for this product | Technical post, not product post |
| OSS maintainer outreach | A single adoption by ripgrep is worth 1000 cold signups | Manual, personal emails to 10 maintainers |
| Dev.to / Hashnode | SEO amplification, long-tail traffic | Cross-post all blog content |
| Twitter/X | Real-time developer audience, meme potential | The "39× energy waste" stat is tweet-worthy |

---

## Part 13 — Implementation Pitfalls (Do Not Make These Mistakes)

| Mistake | Correct Approach |
|---|---|
| Using `nvidia-smi` subprocess for GPU | Use NVML directly via `nvml-wrapper` crate — 150ms+ subprocess startup ruins accuracy |
| Going straight to MSR for RAPL | Use `perf_event_open` first (no root required at paranoid ≤ 2) |
| Ignoring 32-bit RAPL counter overflow | Implement monotonic accumulator; sample at 100ms; this is the #1 silent bug in RAPL tools |
| Comparing raw joules across machines | Use EES (normalised by TDP × duration); cross-machine joule comparison is meaningless |
| Single-run measurements | 7 iterations minimum; report median; validate noise < 15% |
| RAPL is per-socket, not per-core | Cannot attribute to individual cores — measure per-socket total |
| AMD RAPL uses different sysfs path | Detect `AuthenticAMD` in cpuinfo, read `/sys/devices/platform/amd_energy/` |
| GitHub-hosted runners block RAPL | Detect and fall back to estimation model; label ALL estimated results clearly |
| Comparing Intel baseline to AMD current | Store cpu_vendor; refuse cross-vendor comparison with clear error message |
| Posting duplicate PR comments | Search for `<!-- wattlint-report -->` marker; always PATCH, only POST once |
| Using Vercel for the Go API | Serverless functions cannot maintain pgx connection pool — use Fly.io |
| Using TimescaleDB now | Neon PostgreSQL + composite index + Go cron aggregator is sufficient until 50M rows |

---

## Part 14 — 90-Day Success Metrics

```
Technical (non-negotiable):
  - Measurement noise coefficient < 2% on isolated self-hosted runner
  - False positive regression rate < 1%
  - RAPL read latency < 500µs per sample (perf_event_open path)
  - Action install to first PR comment: < 60 seconds

Business (aggressive but achievable):
  - 500 GitHub Action installs in first 30 days (HN + r/rust launch)
  - 50 paying Pro teams at $29/month by Day 90 ($1,450 MRR)
  - Featured in GitHub Marketplace "Recently Added" section
  - 1 major open-source Rust project publicly adopts WattLint
  - 1 blog post from an external developer about energy they saved

Platform (seeds GPU product):
  - WattLint GPU waitlist: 100 sign-ups by Day 90
  - At least 1 AI company pilot of GPU observability by Month 4
```

---

## Part 15 — Key References

- Intel RAPL spec: Intel Software Developer Manual Vol. 3, §14.9
- AMD RAPL: Zen+ via `/sys/devices/platform/amd_energy/`
- Fastest RAPL mechanism: arxiv:2401.15985 (Rust implementation, perf-events wins)
- RAPL accuracy validation: "RAPL in Action" — ACM TOMPECS 2018 (±3% vs wall power)
- LLM code energy variance: arxiv:2505.20324 (up to 39× variance across models)
- GPU production profiling: Polar Signals parcagpu (CUPTI + eBPF, <4% overhead)
- Eco-CI (category primer, not a competitor): github.com/green-coding-solutions/eco-ci-energy-estimation
- Neon PostgreSQL docs: neon.tech/docs
- Upstash Redis: upstash.com/docs/redis
- Fly.io Go deployment: fly.io/docs/languages-and-frameworks/golang
