# OBenchmark

Open-source cross-platform benchmark tool written in Rust.

## Features
- CPU multi-core real benchmark (several subtests described below)
- Detailed memory tests including cached/uncached read, write, latency, availability
- Disk I/O tests covering sequential read/write and random IOPS at different depths
- Normalized score system with per-benchmark clamping and 5‑digit final score
- Beautiful GUI dashboard with progress bars and result export
- Export results as JSON for later analysis

## Benchmark Tests
Each run executes a suite of individual benchmarks; results are normalized and weighted to produce a single final score (max 99999).

### CPU tests
- **Multi-Core**: parallel integer loop to exercise all cores.
- **Int Math**: saturated integer arithmetic.
- **Float Math**: floating point operations with trig functions.
- **Prime Calculation**: count primes in a time window.
- **SSE Ext**: simulated SIMD via vector additions.
- **Compression**: one-shot zlib compression.
- **Encryption**: repeated SHA‑256 hashing.
- **Physics**: simple position update loop.
- **Sorting**: random 64‑bit integer sort.
- **UCT Single**: dummy tree search loop (UCT-style).

### Memory tests
- **DB Ops**: simple push and random access pattern.
- **Cached Read**: repetitive small-buffer reads (fits in cache).
- **Uncached Read**: large-buffer sequential scan.
- **Write**: memory write bandwidth.
- **Available**: system-reported free RAM (MB).
- **Latency**: pointer-chasing traversal to measure access latency.
- **Threaded**: concurrent writes in multiple threads.

### Disk tests
- **Sequential Read/Write**: 512 MB contiguous ops.
- **IOPS 32K QD20**: random 32 KB ops with queue depth 20.
- **IOPS 4K QD1**: random 4 KB ops single queued.

## Scoring
Raw results are normalized against predefined baselines per category. Each normalized value is capped to 10 000 to avoid outliers skewing the average. Scores are weighted, averaged, and finally clamped to at most five decimal digits (≤ 99999) to produce the final score shown in the GUI. This ensures reproducible, comparable numbers across different hardware.

## Usage
Build and run with `cargo run` or use the provided binary. Click **Start Benchmark** to begin; the GUI will display progress and final results. You can export a JSON file of the scores using the corresponding button or restart the tests with the "New Analysis" button.
