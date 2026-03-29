# Setup

## Build from source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/wattlint/watt-sampler.git
cd watt-sampler
cargo build --release

# Verify
./target/release/watt-sampler check-hardware
```

## Docker (for macOS development)

```bash
# Build in Linux container
docker run --rm -v $(pwd):/app -w /app rust:latest cargo build --release

# Run tests
docker run --rm -v $(pwd):/app -w /app rust:latest cargo test

# Test CLI (no RAPL in Docker — energy will be null)
docker run --rm -v $(pwd):/app -w /app rust:latest \
  ./target/release/watt-sampler run --domains pkg -- echo hello
```

## RAPL permissions

Check your current setting:

```bash
cat /proc/sys/kernel/perf_event_paranoid
```

| Value | Can use RAPL? |
|-------|--------------|
| ≤ 2   | Yes, as current user |
| 3     | Root required |

To lower temporarily: `sudo sh -c 'echo 2 > /proc/sys/kernel/perf_event_paranoid'`

To lower permanently, add to `/etc/sysctl.conf`:
```
kernel.perf_event_paranoid = 2
```

## GPU support

NVIDIA GPUs require the NVML library (`libnvidia-ml.so`) which is installed with the NVIDIA driver. No additional install needed.

AMD GPU power is read from `/sys/class/drm/card*/device/hwmon/hwmon*/power1_average`.
