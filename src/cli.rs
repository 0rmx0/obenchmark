use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json;
use std::path::PathBuf;
use std::time::Duration;

use crate::{
    benchmarks::suite::{BenchSpec, AVAILABLE_BENCHMARKS},
    engines::runner::build_bench_result,
    model::result::{BenchResult, BenchScore},
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
    #[command(alias = "headless")]
    Cli(CliOptions),
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
}

/// Run the CLI command and return the serialized result.
pub fn run_cli(opts: CliOptions) -> Result<()> {
    if opts.list {
        for spec in AVAILABLE_BENCHMARKS {
            println!("{}", spec.name);
        }
        return Ok(());
    }

    let specs = select_specs(opts.filter.as_deref());
    if specs.is_empty() {
        bail!("no benchmarks match the provided filter");
    }

    let total = specs.len();
    let progress = if total > 0 {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::with_template("{msg} {bar:40.cyan/blue} {pos}/{len}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message("Preparing benchmarks");
        pb.enable_steady_tick(Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let mut scores = Vec::new();
    for spec in specs.iter().copied() {
        if let Some(pb) = &progress {
            pb.set_message(format!("Running {}", spec.name));
        }

        let bench = spec.build();
        let raw_score = bench
            .run()
            .with_context(|| format!("benchmark '{}' failed", spec.name))?;
        let weight = bench.weight();

        scores.push(BenchScore {
            name: spec.name.to_string(),
            raw_score,
            weight,
        });

        if let Some(pb) = &progress {
            pb.inc(1);
        }
    }

    if let Some(pb) = &progress {
        pb.finish_with_message("Benchmarks complete");
    }

    let result = build_bench_result(scores);

    let mut json_dump = None;
    if opts.json || opts.export.is_some() {
        json_dump = Some(serde_json::to_string_pretty(&result)?);
    }

    if opts.json {
        println!("{}", json_dump.as_deref().unwrap());
    } else {
        print_summary(&result);
        if opts.raw {
            for score in &result.scores {
                println!(
                    "{: <25} {:>12} {}",
                    score.name,
                    score.raw_score,
                    unit_for_bench(&score.name)
                );
            }
        }
    }

    if let Some(path) = opts.export {
        let dump = json_dump
            .as_ref()
            .cloned()
            .unwrap_or_else(|| serde_json::to_string_pretty(&result).unwrap());
        std::fs::write(&path, dump)
            .with_context(|| format!("failed writing {}", path.display()))?;
        eprintln!("Wrote JSON output to {}", path.display());
    }

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
                .unwrap_or_else(|| human_bytes(disk.total_bytes.unwrap_or(0)));
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

fn human_bytes(bytes: u64) -> String {
    let mut bytes = bytes as f64;
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut idx = 0;
    while bytes >= 1024.0 && idx < units.len() - 1 {
        bytes /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", bytes as u64, units[idx])
    } else {
        format!("{:.2} {}", bytes, units[idx])
    }
}

fn unit_for_bench(name: &str) -> &'static str {
    match name {
        "CPU Multi-Core" | "CPU Int Math" | "CPU Float Math" | "CPU Prime Calc" | "CPU SSE Ext"
        | "CPU Physics" | "CPU Sorting" | "CPU UCT Single" => "ops/s",

        "CPU Compression" | "CPU Encryption" => "MB/s",

        "Mem DB Ops" => "ops/s",
        "Mem Cached Read" | "Mem Uncached Read" | "Mem Write" | "Mem Threaded" => "MB/s",
        "Mem Available" => "MB",
        "Mem Latency" => "accès/s",

        "Disk Seq Read" | "Disk Seq Write" => "MB/s",
        "Disk IOPS 32K QD20" | "Disk IOPS 4K QD1" => "IOPS",

        _ => "",
    }
}
