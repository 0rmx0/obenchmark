use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;

use crate::{
    benchmarks::suite::{BenchSpec, AVAILABLE_BENCHMARKS},
    engines::benchmark::bench_meta,
    engines::runner::build_bench_result,
    model::history,
    model::result::{BenchResult, BenchScore},
    util::config::{
        set_run_config, set_verbose, RunConfig, DEFAULT_BENCH_DURATION, QUICK_BENCH_DURATION,
    },
    util::format::human_bytes,
    util::report::render_html,
};

/// Command-line entry point for OBenchmark.
#[derive(Parser)]
#[command(author, version, about = "OBenchmark CLI", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

/// Supported CLI subcommands.
#[derive(Subcommand)]
pub enum CliCommand {
    /// Run the benchmark suite headless.
    #[command(alias = "headless")]
    Cli(CliOptions),
    /// List recorded runs from the local history.
    History,
    /// Compare two exported JSON result files.
    Compare {
        /// First (reference) JSON result file.
        file_a: PathBuf,
        /// Second JSON result file.
        file_b: PathBuf,
    },
}

/// Options that control the CLI run.
#[derive(Parser, Debug)]
pub struct CliOptions {
    #[arg(
        long,
        short = 'e',
        value_name = "FILE",
        help = "Save the JSON result to FILE"
    )]
    pub export: Option<PathBuf>,
    #[arg(
        long,
        short = 'j',
        help = "Print the result as JSON instead of the human summary"
    )]
    pub json: bool,
    #[arg(long, short = 'r', help = "Append raw benchmark scores to the summary")]
    pub raw: bool,
    #[arg(long, short = 'l', help = "List available benchmarks and exit")]
    pub list: bool,
    #[arg(
        long,
        short = 'f',
        help = "Enable only benchmarks whose name contains this substring (case-insens)"
    )]
    pub filter: Option<String>,
    #[arg(
        long,
        help = "Quick mode: 1s per benchmark and smaller disk test files"
    )]
    pub quick: bool,
    #[arg(
        long,
        value_name = "SECS",
        help = "Duration of each timed benchmark loop in seconds"
    )]
    pub duration: Option<u64>,
    #[arg(
        long,
        value_name = "DIR",
        help = "Directory where disk benchmarks create their test files (default: temp dir)"
    )]
    pub disk_dir: Option<PathBuf>,
    #[arg(long, short = 'v', help = "Verbose scoring diagnostics on stderr")]
    pub verbose: bool,
    #[arg(
        long,
        value_name = "FILE",
        help = "Also write a self-contained HTML report to FILE"
    )]
    pub html: Option<PathBuf>,
}

/// Run the CLI command and return the serialized result.
pub fn run_cli(opts: CliOptions) -> Result<()> {
    set_verbose(opts.verbose);

    if opts.list {
        for spec in AVAILABLE_BENCHMARKS {
            println!("{}", spec.name);
        }
        return Ok(());
    }

    let duration = if opts.quick {
        QUICK_BENCH_DURATION
    } else {
        opts.duration
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_BENCH_DURATION)
    };
    set_run_config(RunConfig {
        bench_duration: duration,
        disk_dir: opts.disk_dir.clone().unwrap_or_else(std::env::temp_dir),
        quick: opts.quick,
        progress_tx: None,
    });

    let specs = select_specs(opts.filter.as_deref());
    if specs.is_empty() {
        bail!("no benchmarks match the provided filter");
    }

    let total = specs.len();
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("{msg} {bar:40.cyan/blue} {pos}/{len}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message("Preparing benchmarks");
    pb.enable_steady_tick(Duration::from_millis(80));

    let mut scores = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for spec in specs.iter().copied() {
        pb.set_message(format!("Running {}", spec.name));

        let bench = spec.build();
        match bench.run() {
            Ok(raw_score) => {
                scores.push(BenchScore {
                    name: spec.name.to_string(),
                    raw_score,
                    weight: spec.weight,
                });
            }
            Err(err) => {
                // Keep going: one failing benchmark must not kill the run.
                let msg = format!("{}: {}", spec.name, err);
                errors.push(msg.clone());
                eprintln!("warning: benchmark failed: {}", msg);
            }
        }

        pb.inc(1);
    }
    pb.finish_with_message("Benchmarks complete");

    let result = build_bench_result(scores, errors);

    // Serialize once, reuse for stdout/file exports.
    let json_dump = if opts.json || opts.export.is_some() {
        Some(serde_json::to_string_pretty(&result)?)
    } else {
        None
    };

    if opts.json {
        println!("{}", json_dump.as_deref().unwrap());
    } else {
        print_summary(&result);
        if opts.raw {
            for score in &result.scores {
                println!(
                    "{: <28} {:>14} {}",
                    score.name,
                    score.raw_score,
                    bench_meta(&score.name).unit
                );
            }
        }
        for err in &result.errors {
            println!("  [failed] {}", err);
        }
    }

    if let Some(path) = &opts.export {
        let dump = json_dump
            .clone()
            .unwrap_or_else(|| serde_json::to_string_pretty(&result).unwrap());
        std::fs::write(path, dump).with_context(|| format!("failed writing {}", path.display()))?;
        eprintln!("Wrote JSON output to {}", path.display());
    }

    if let Some(path) = &opts.html {
        let html = render_html(&result);
        std::fs::write(path, html).with_context(|| format!("failed writing {}", path.display()))?;
        eprintln!("Wrote HTML report to {}", path.display());
    }

    // Record the run in the local history (best effort).
    if let Err(e) = history::append_history(&result) {
        if opts.verbose {
            eprintln!("warning: could not append to history: {}", e);
        }
    }

    // Distinct exit code when some benchmarks failed.
    if !result.errors.is_empty() {
        std::process::exit(2);
    }

    Ok(())
}

/// Print the recorded runs (oldest first).
pub fn run_history() -> Result<()> {
    let runs = history::load_history()?;
    if runs.is_empty() {
        println!(
            "No recorded runs yet. History file: {}",
            history::history_path().display()
        );
        return Ok(());
    }
    println!(
        "{:<26} {:>8} {:>8} {:>8} {:>8}",
        "timestamp", "global", "cpu", "ram", "disk"
    );
    for r in &runs {
        println!(
            "{:<26} {:>8} {:>8} {:>8} {:>8}",
            r.timestamp.as_deref().unwrap_or("-"),
            r.final_score,
            r.cpu_score,
            r.mem_score,
            r.disk_score
        );
    }
    Ok(())
}

/// Compare two JSON result files.
pub fn run_compare(file_a: &std::path::Path, file_b: &std::path::Path) -> Result<()> {
    let a: BenchResult = serde_json::from_str(&std::fs::read_to_string(file_a)?)
        .with_context(|| format!("failed parsing {}", file_a.display()))?;
    let b: BenchResult = serde_json::from_str(&std::fs::read_to_string(file_b)?)
        .with_context(|| format!("failed parsing {}", file_b.display()))?;
    print!(
        "{}",
        history::format_comparison(
            &file_a.display().to_string(),
            &a,
            &file_b.display().to_string(),
            &b
        )
    );
    Ok(())
}

fn select_specs(filter: Option<&str>) -> Vec<BenchSpec> {
    let needle = filter.map(|f| f.to_lowercase());
    AVAILABLE_BENCHMARKS
        .iter()
        .copied()
        .filter(|spec| match &needle {
            Some(n) => spec.name.to_lowercase().contains(n),
            None => true,
        })
        .collect()
}

fn print_summary(result: &BenchResult) {
    println!("OBenchmark result");
    println!("  Global score: {}", result.final_score);
    println!("  CPU score:    {}", result.cpu_score);
    println!("  RAM score:    {}", result.mem_score);
    println!("  Disk score:   {}", result.disk_score);
    if let Some(ts) = &result.timestamp {
        println!("  Timestamp:    {}", ts);
    }

    if let Some(info) = &result.system_info {
        println!("System information:");
        println!(
            "  CPU: {} {}",
            info.cpu.vendor.as_deref().unwrap_or("unknown"),
            info.cpu.model.as_deref().unwrap_or("unknown")
        );
        if let Some(freq) = info.cpu.frequency_mhz {
            println!("  Frequency: {} MHz", freq);
        }
        if let Some(temp) = info.cpu.cpu_temp_c {
            println!("  Temperature: {:.0} °C", temp);
        }
        println!("  Logical cores: {}", info.cpu.cores_logical);
        if let Some(physical) = info.cpu.cores_physical {
            println!("  Physical cores: {}", physical);
        }

        println!(
            "  RAM: {} MB{}",
            info.ram.total_mb,
            info.ram
                .ram_type
                .as_ref()
                .map(|t| format!(" ({})", t))
                .unwrap_or_default()
        );

        for disk in &info.disks {
            let label = disk.vendor.as_deref().unwrap_or("unknown").to_string();
            let model = disk.model.as_deref().unwrap_or("unknown");
            let size = disk
                .size_readable
                .clone()
                .unwrap_or_else(|| human_bytes(disk.total_bytes.unwrap_or(0) as f64));
            println!(
                "  Disk {} {} [{}] - {}",
                label,
                model,
                disk.disk_type.as_deref().unwrap_or("unknown"),
                size
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_specs_none_filter() {
        let specs = select_specs(None);
        assert_eq!(specs.len(), AVAILABLE_BENCHMARKS.len());
    }

    #[test]
    fn select_specs_empty_filter() {
        let specs = select_specs(Some(""));
        assert_eq!(specs.len(), AVAILABLE_BENCHMARKS.len());
    }

    #[test]
    fn select_specs_cpu_filter() {
        let specs = select_specs(Some("CPU"));
        assert!(!specs.is_empty());
        assert!(specs.len() < AVAILABLE_BENCHMARKS.len());
        for spec in specs {
            assert!(spec.name.contains("CPU"));
        }
    }

    #[test]
    fn select_specs_case_insensitive() {
        let specs_cpu = select_specs(Some("cpu"));
        let specs_cpu_upper = select_specs(Some("CPU"));
        assert_eq!(specs_cpu.len(), specs_cpu_upper.len());
    }

    #[test]
    fn select_specs_no_match() {
        let specs = select_specs(Some("NONEXISTENT"));
        assert_eq!(specs.len(), 0);
    }

    #[test]
    fn select_specs_partial_match() {
        let specs = select_specs(Some("Int"));
        assert!(!specs.is_empty());
        let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        assert!(names.contains(&"CPU Int Math"));
    }

    #[test]
    fn select_specs_disk_write() {
        let specs = select_specs(Some("write"));
        let names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Disk Seq Write"));
        assert!(names.contains(&"Disk IOPS 4K QD1 Write"));
        assert!(!names.contains(&"CPU Int Math"));
    }
}
