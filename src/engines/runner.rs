use crate::engines::benchmark::Benchmark;
use crate::engines::score::compute_aggregated_scores;
use crate::model::result::{BenchResult, BenchScore};
use crate::util::sysinfo::get_detailed_system_info;
use anyhow::{Context, Result};
use crossbeam_channel::Sender;

pub enum RunnerEvent {
    BenchStarted(String),
    BenchFinished(String, u64),
    Done(BenchResult),
    Error(String),
}

pub fn run_benchmarks(benches: Vec<Box<dyn Benchmark>>, tx: Sender<RunnerEvent>) {
    std::thread::spawn(move || {
        let mut scores: Vec<BenchScore> = Vec::new();

        for bench in benches {
            let name = bench.name().to_string();
            let weight = bench.weight();

            tx.send(RunnerEvent::BenchStarted(name.clone())).ok();

            match bench.run() {
                Ok(score) => {
                    scores.push(BenchScore {
                        name: name.clone(),
                        raw_score: score,
                        weight,
                    });
                    tx.send(RunnerEvent::BenchFinished(name.clone(), score))
                        .ok();
                }
                Err(e) => {
                    tx.send(RunnerEvent::Error(e.to_string())).ok();
                    return;
                }
            }
        }

        tx.send(RunnerEvent::Done(build_bench_result(scores))).ok();
    });
}

pub fn run_benchmarks_blocking(benches: Vec<Box<dyn Benchmark>>) -> Result<BenchResult> {
    let mut scores = Vec::new();

    for bench in benches {
        let name = bench.name().to_string();
        let weight = bench.weight();

        let raw_score = bench
            .run()
            .with_context(|| format!("benchmark '{}' failed", name))?;

        scores.push(BenchScore {
            name,
            raw_score,
            weight,
        });
    }

    Ok(build_bench_result(scores))
}

pub fn build_bench_result(scores: Vec<BenchScore>) -> BenchResult {
    let aggregated = compute_aggregated_scores(&scores);
    let system_info = get_detailed_system_info();

    BenchResult {
        scores,
        final_score: aggregated.global,
        cpu_score: aggregated.cpu,
        mem_score: aggregated.mem,
        disk_score: aggregated.disk,
        system_info: Some(system_info),
    }
}
