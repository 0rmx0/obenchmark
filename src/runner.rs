use std::thread;
use std::sync::mpsc;
use anyhow::Result;
use crate::benchmarks::Benchmark;

pub struct BenchResult {
    pub scores: Vec<(String, u64)>,
    pub global_score: u64,
}

fn weighted_score(scores: &[(String, u64)]) -> u64 {
    let mut total = 0;
    let mut weight_sum = 0;

    for (name, score) in scores {
        let weight = match name.as_str() {
            "CPU MultiCore" => 4,
            "CPU SingleCore" => 3,
            "Memory" => 2,
            "Disk" => 1,
            _ => 1,
        };
        total += score * weight;
        weight_sum += weight;
    }

    if weight_sum == 0 { 0 } else { total / weight_sum }
}

pub fn run_async(benches: Vec<Box<dyn Benchmark + Send>>) 
    -> mpsc::Receiver<Result<BenchResult>> {

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut scores = Vec::new();

        for bench in benches {
            match bench.run() {
                Ok(score) => scores.push((bench.name().to_string(), score)),
                Err(e) => {
                    tx.send(Err(e)).ok();
                    return;
                }
            }
        }

        let global = weighted_score(&scores);

        tx.send(Ok(BenchResult {
            scores,
            global_score: global,
        })).ok();
    });

    rx
}