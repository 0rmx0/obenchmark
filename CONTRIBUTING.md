# Contributing to OBenchmark

Thanks for your interest in improving OBenchmark!

## Building

Rust stable (2021 edition) is required.

Linux build dependencies (Slint/winit/fontconfig):

```bash
sudo apt-get install -y pkg-config \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev \
  libwayland-dev libxkbcommon-dev libudev-dev libvulkan-dev libfontconfig1-dev
```

On Windows and macOS a plain `cargo build` works.

```bash
cargo build          # debug build (GUI)
cargo run -- cli --quick   # headless fast smoke run
```

## Checks (run before opening a PR)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run -- cli --quick --export /tmp/smoke.json   # end-to-end smoke run
```

`Cargo.lock` is committed on purpose (this is an executable): do not gitignore it, and commit its updates.

## Project layout

- `src/benchmarks/` — the individual benchmarks (cpu / memory / disk) and `suite.rs`, the registry of all benchmarks (`BenchSpec`: name, weight, builder).
- `src/engines/` — the `Benchmark` trait, `bench_meta()` (unit/category/baseline per benchmark — the single source of truth), the scoring (`score.rs`) and the threaded runner (`runner.rs`).
- `src/model/` — serializable result types (`result.rs`) and the local history (`history.rs`).
- `src/app/` + `ui/app.slint` — the Slint GUI.
- `src/cli.rs` — the headless CLI (`obenchmark cli`, `history`, `compare`).
- `src/util/` — shared helpers: `format` (human-readable sizes), `config` (per-run configuration), `cleanup` (temp file guards + signal handlers), `fadvise` (page-cache dropping on Linux), `report` (HTML export), `sysinfo` (system information).

## Adding a benchmark

1. Implement the `Benchmark` trait in `src/benchmarks/` (`run()` returns a raw `u64` score).
2. Register it in `AVAILABLE_BENCHMARKS` in `src/benchmarks/suite.rs` (name, weight, builder).
3. Add its unit, category and baseline in `bench_meta()` in `src/engines/benchmark.rs`.

That's it: scoring, CLI units, GUI display, JSON export and HTML report pick it up automatically.

## Pull requests

- Keep changes focused; one PR per topic.
- Update `CHANGELOG.md` (Unreleased section) and the README when behavior changes.
- CI runs fmt, clippy (`-D warnings`), tests and a `--quick` smoke run on every PR, then builds release artifacts for Linux/Windows/macOS (x86‑64 + ARM64).
