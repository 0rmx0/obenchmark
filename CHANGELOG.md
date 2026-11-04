# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed
- Final score cap is now 99 999 (5 digits) and per-benchmark cap 10 000, matching the documented scoring.
- `CPU Prime Calc` and `CPU UCT Single` now report operations per second instead of a raw 5 s count.
- Memory benchmarks (`DB Ops`, `Cached Read`, `Uncached Read`, `Write`, `Latency`, `Threaded`) now loop for the configured duration instead of a single short pass, making measurements stable.
- Memory buffer sizing can no longer exceed half of the physical RAM on small machines (OOM risk removed).
- Disk random IOPS are computed over the measured elapsed time (not the nominal duration) and worker thread panics are propagated instead of being silently ignored.
- Disk read benchmarks drop the Linux page cache (`posix_fadvise(DONTNEED)`) before measuring so they hit the device, not RAM.
- Disk test files get unique per-process names and are always removed via a RAII guard (even on error/panic/Ctrl‑C); stale files from dead processes are cleaned at startup.
- A failing benchmark no longer aborts the whole run: errors are collected per benchmark (GUI panel, CLI summary, JSON `errors` field) and the CLI exits with code 2.
- Removed the unnecessary `unsafe` block in the memory latency benchmark.
- `CpuSSE`, `CpuFloatMath` and `CpuPhysics` accumulators are bounded (no more inf/NaN drift); compression now uses reproducible pseudo-random data instead of zeros.
- French UI strings corrected; `Cargo.lock` is now committed for reproducible builds.

### Changed
- Benchmark metadata (unit, category, baseline, weight) is centralized: `BenchSpec` carries the weight and `engines::benchmark::bench_meta()` is the single source of truth for units/categories/baselines — no more duplicated `match` tables in CLI/GUI/scoring.
- `human_bytes` is implemented once in `util::format` and shared by CLI, GUI and sysinfo.
- Benchmark duration, disk test directory and quick mode are configurable at runtime (`RunConfig`), enabling `--quick` / `--duration` / `--disk-dir`.
- Score debug output moved behind `--verbose`.

### Added
- New disk benchmarks: random write IOPS 32K QD20 and 4K QD1.
- CLI: `--quick`, `--duration <secs>`, `--disk-dir <dir>`, `--verbose`, `--html <file>`, plus `history` and `compare <a> <b>` subcommands; distinct exit codes (0/1/2).
- Local run history (JSONL in the user data dir) with `obenchmark history` and `obenchmark compare`.
- Self-contained HTML report export (CLI `--html` and GUI "Export HTML").
- GUI: benchmark selection checkboxes with All/None, intra-benchmark live progress, last-result readout, error panel, FR/EN toggle, dark/light theme, export path feedback.
- CPU temperature in system information (CLI, GUI, JSON, HTML) to spot thermal throttling.
- `BenchResult` gains `errors` and `timestamp` fields (backward-compatible deserialization of old exports).
- CI: new `Test & Lint` job (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, CLI smoke run with `--quick`).
- `CHANGELOG.md` and `CONTRIBUTING.md`.

### Notes
- GPU benchmarking (wgpu compute shader) and a community leaderboard are on the roadmap but intentionally not part of this change: they need a dedicated backend/design and are not wired to any external service.
