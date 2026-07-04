use crate::engines::benchmark::Benchmark;
use crate::engines::score::compute_aggregated_scores;
use crate::model::result::{BenchResult, BenchScore};
use crate::util::sysinfo::get_detailed_system_info;
use crossbeam_channel::Sender;

pub enum RunnerEvent {
    BenchStarted(String),
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::result::BenchScore;

    #[test]
    fn build_bench_result_empty_scores() {
        let scores = vec![];
        let result = build_bench_result(scores);
        
        assert_eq!(result.scores.len(), 0);
        assert_eq!(result.final_score, 0);
        assert_eq!(result.cpu_score, 0);
        assert_eq!(result.mem_score, 0);
        assert_eq!(result.disk_score, 0);
        assert!(result.system_info.is_some());
    }

    #[test]
    fn build_bench_result_with_scores() {
        let scores = vec![
            BenchScore {
                name: "CPU Multi-Core".to_string(),
                raw_score: 80_000_000,
                weight: 3,
            },
            BenchScore {
                name: "Mem Write".to_string(),
                raw_score: 20_000,
                weight: 2,
            },
            BenchScore {
                name: "Disk Seq Read".to_string(),
                raw_score: 500,
                weight: 2,
            },
        ];
        
        let result = build_bench_result(scores);
        
        assert_eq!(result.scores.len(), 3);
        assert!(result.final_score > 0);
        assert!(result.cpu_score > 0);
        assert!(result.mem_score > 0);
        assert!(result.disk_score > 0);
        assert!(result.system_info.is_some());
    }

    #[test]
    fn build_bench_result_only_cpu() {
        let scores = vec![
            BenchScore {
                name: "CPU Multi-Core".to_string(),
                raw_score: 80_000_000,
                weight: 3,
            },
        ];
        
        let result = build_bench_result(scores);
        
        assert!(result.cpu_score > 0);
        assert_eq!(result.mem_score, 0);
        assert_eq!(result.disk_score, 0);
    }

    #[test]
    fn build_bench_result_only_memory() {
        let scores = vec![
            BenchScore {
                name: "Mem Write".to_string(),
                raw_score: 20_000,
                weight: 2,
            },
        ];
        
        let result = build_bench_result(scores);
        
        assert_eq!(result.cpu_score, 0);
        assert!(result.mem_score > 0);
        assert_eq!(result.disk_score, 0);
    }

    #[test]
    fn build_bench_result_only_disk() {
        let scores = vec![
            BenchScore {
                name: "Disk Seq Read".to_string(),
                raw_score: 500,
                weight: 2,
            },
        ];
        
        let result = build_bench_result(scores);
        
        assert_eq!(result.cpu_score, 0);
        assert_eq!(result.mem_score, 0);
        assert!(result.disk_score > 0);
    }

    #[test]
    fn build_bench_result_preserves_scores() {
        let original_scores = vec![
            BenchScore {
                name: "CPU Multi-Core".to_string(),
                raw_score: 80_000_000,
                weight: 3,
            },
            BenchScore {
                name: "Mem Write".to_string(),
                raw_score: 20_000,
                weight: 2,
            },
        ];
        
        let result = build_bench_result(original_scores.clone());
        
        assert_eq!(result.scores.len(), original_scores.len());
        for (i, score) in result.scores.iter().enumerate() {
            assert_eq!(score.name, original_scores[i].name);
            assert_eq!(score.raw_score, original_scores[i].raw_score);
            assert_eq!(score.weight, original_scores[i].weight);
        }
    }
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
