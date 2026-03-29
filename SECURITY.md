# Security

## Reporting vulnerabilities

Email security@wattlint.com. Do not open a public issue.

## Supply chain

- All dependencies are pinned to exact versions in `Cargo.toml`
- The GitHub Action that uses this binary verifies SHA256 checksums before execution
- No network calls are made by the sampler itself — it only reads local sysfs and NVML

## Permissions

`watt-sampler` requires read access to:
- `/sys/class/powercap/intel-rapl/` (RAPL energy counters)
- `/sys/devices/platform/amd_energy/` (AMD RAPL)
- `/dev/nvidia*` (NVML, only if GPU measurement is requested)

It does not write to any system files or require root by default (with `perf_event_paranoid ≤ 2`).
