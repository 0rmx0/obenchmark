# OBenchmark

Open-source cross-platform benchmark tool written in Rust.

## Features
- CPU multi-core real benchmark (several subtests described below)
- Detailed memory tests including cached/uncached read, write, latency, availability
- Disk I/O tests covering sequential read/write and random read/write IOPS at different depths
- Normalized score system with per-benchmark clamping and 5‑digit final score
- Beautiful Slint GUI dashboard with benchmark selection, live progress and result export (JSON + HTML)
- French/English UI and dark/light theme
- Local run history with comparison between runs
- Export results as JSON or a self-contained HTML report for later analysis

## Benchmark Tests
Each run executes a suite of individual benchmarks; results are normalized and weighted to produce a single final score (max 99999).

### CPU tests
- **Multi-Core**: parallel integer loop to exercise all cores.
- **Int Math**: saturated integer arithmetic.
- **Float Math**: floating point operations with trig functions.
- **Prime Calculation**: primes per second in a time window.
- **SSE Ext**: simulated SIMD via vector additions.
- **Compression**: one-shot zlib compression of pseudo-random data.
- **Encryption**: repeated SHA‑256 hashing.
- **Physics**: simple position update loop.
- **Sorting**: random 64‑bit integer sort.
- **UCT Single**: dummy tree search loop (UCT-style).

### Memory tests
- **DB Ops**: push and random access pattern.
- **Cached Read**: repetitive small-buffer reads (fits in cache).
- **Uncached Read**: large-buffer sequential scan, repeated passes.
- **Write**: memory write bandwidth, repeated passes.
- **Available**: system-reported free RAM (MB).
- **Latency**: pointer-chasing traversal to measure access latency.
- **Threaded**: concurrent writes in multiple threads.

All timed tests loop for a configurable duration (default 5 s per benchmark) so results are stable and comparable.

### Disk tests
- **Sequential Read/Write**: 512 MB contiguous ops (64 MB in quick mode).
- **IOPS 32K QD20 (read/write)**: random 32 KB ops with queue depth 20.
- **IOPS 4K QD1 (read/write)**: random 4 KB ops, single queued.

Test files are created in a configurable directory (default: the system temp dir), removed automatically when the run ends (even on Ctrl‑C), and the Linux page cache is dropped (`posix_fadvise(DONTNEED)`) before read measurements so they hit the device rather than RAM.

## Scoring
Raw results are normalized against predefined baselines per category. Each normalized value is capped to 10 000 to avoid outliers skewing the average. Scores are weighted, averaged, and finally clamped to at most five digits (≤ 99999) to produce the final score shown in the GUI. This ensures reproducible, comparable numbers across different hardware.

## Usage
Build and run with `cargo run` or use the provided binary. In the GUI you can pick which benchmarks to run, then click **Start Benchmark**; the dashboard shows live per-benchmark progress and final results. Export buttons write a timestamped JSON file and an HTML report to the user data directory (`~/.local/share/obenchmark/results/` on Linux) and display the path. **New Analysis** restarts the tests. Every finished run is also appended to the local history.

## CLI Usage
You can run the same benchmark suite without the GUI with `obenchmark cli` (or `cargo run -- cli`).
- `--list` prints the available benchmarks.
- `--filter <substring>` runs only benchmarks whose name contains the provided text (case-insens).
- `--raw` appends the raw score and unit per benchmark to the summary.
- `--json` prints the full result as JSON to stdout; this also feeds `--export`.
- `--export <file>` saves the serialized result to `file`.
- `--html <file>` also writes a self-contained HTML report to `file`.
- `--quick` runs a fast pass (1 s per benchmark, smaller disk files) — ideal for CI and smoke tests.
- `--duration <secs>` overrides the per-benchmark duration.
- `--disk-dir <dir>` chooses where disk test files are created (default: temp dir; use your system disk mount to avoid measuring a tmpfs).
- `--verbose` prints scoring diagnostics on stderr.
- The CLI shows a progress bar that reports the current benchmark and the number completed, so you can watch execution even outside the GUI.

Exit codes: `0` success, `1` fatal error, `2` the run completed but some benchmarks failed (listed in the summary and in the JSON `errors` field; the run continues past individual failures).

The CLI still reports final, CPU, RAM and disk scores plus system information (including CPU temperature), so you can script repeated runs or integrate the tool into CI.

### History and comparison
- Every CLI/GUI run is appended to `~/.local/share/obenchmark/history.jsonl` (JSON Lines).
- `obenchmark history` lists recorded runs (timestamp, global/CPU/RAM/disk scores).
- `obenchmark compare <fileA.json> <fileB.json>` prints per-benchmark and per-category deltas between two exported results.

## Development
See [CONTRIBUTING.md](CONTRIBUTING.md). In short: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, then `cargo run -- cli --quick` for a fast smoke run. `Cargo.lock` is committed so CI and local builds are reproducible.
