# Contributing

## Getting started

1. Clone the repo
2. Build with `cargo build`
3. Run tests with `cargo test`
4. Lint with `cargo clippy`

## Testing on Linux

This tool requires Linux with RAPL support. On macOS, use Docker:

```bash
docker run --rm -v $(pwd):/app -w /app rust:latest cargo build
docker run --rm -v $(pwd):/app -w /app rust:latest cargo test
```

For real energy measurements, test on a Linux machine with Intel/AMD RAPL.

## Code style

- No comments unless explaining non-obvious domain logic
- `cargo clippy` must pass with zero warnings
- All public functions need doc comments
- Unit tests for any new computation logic

## Pull requests

- One logical change per PR
- Include test output showing `cargo test` and `cargo clippy` pass
- If adding a RAPL method or GPU backend, include benchmark output showing it works on real hardware
