use crate::engines::benchmark::Benchmark;
use crate::engines::score::compute_aggregated_scores;
use crate::model::result::{BenchResult, BenchScore, SampleResult};
use crate::util::sysinfo::get_detailed_system_info;
use crossbeam_channel::Sender;

pub enum RunnerEvent {
    BenchStarted(String),
    #[allow(dead_code)]
    BenchFinished(String, u64),
    BenchFinishedWithSamples(String, SampleResult),
    Done(BenchResult),
    Error(String),
}

pub fn run_benchmarks(benches: Vec<Box<dyn Benchmark>>, tx: Sender<RunnerEvent>) {
    std::thread::spawn(move || {
        let mut scores: Vec<BenchScore> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        let mut failed_count = 0;

        for bench in benches {
            let name = bench.name().to_string();
            let weight = bench.weight();

            tx.send(RunnerEvent::BenchStarted(name.clone())).ok();

            match bench.run() {
                Ok(sample_result) => {
                    let raw_score = sample_result.value;
                    
                    // Calculate std_dev_percent for the score
                    let std_dev_percent = if sample_result.value > 0 {
                        Some((sample_result.std_dev / sample_result.value as f64) * 100.0)
                    } else {
                        Some(0.0)
                    };
                    
                    scores.push(BenchScore {
                        name: name.clone(),
                        raw_score,
                        weight,
                        samples: Some(sample_result.clone()),
                        std_dev_percent,
                        min: Some(sample_result.min),
                        max: Some(sample_result.max),
                    });
                    
                    tx.send(RunnerEvent::BenchFinished(name.clone(), raw_score)).ok();
                    tx.send(RunnerEvent::BenchFinishedWithSamples(name, sample_result)).ok();
                }
                Err(e) => {
                    errors.push(format!("Benchmark '{}' failed: {}", name, e));
                    failed_count += 1;
                    tx.send(RunnerEvent::Error(e.to_string())).ok();
                    // Continue with next benchmark instead of returning
                }
            }
        }

        let result = build_bench_result_with_errors(scores, errors);
        tx.send(RunnerEvent::Done(result)).ok();
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

    // Count completed, failed, and skipped benchmarks
    let completed_benchmarks = scores.len();
    let failed_benchmarks = 0; // No failures in this path
    let skipped_benchmarks = 0;

    BenchResult {
        scores,
        final_score: aggregated.global,
        cpu_score: aggregated.cpu,
        mem_score: aggregated.mem,
        disk_score: aggregated.disk,
        gfx_score: aggregated.gfx,
        system_info: Some(system_info),
        errors: vec![],
        validation: None,
        completed_benchmarks,
        failed_benchmarks,
        skipped_benchmarks,
        start_time: None,
        end_time: None,
        duration_seconds: None,
    }
}

pub fn build_bench_result_with_errors(scores: Vec<BenchScore>, errors: Vec<String>) -> BenchResult {
    let aggregated = compute_aggregated_scores(&scores);
    let system_info = get_detailed_system_info();

    // Convert errors to BenchError format
    let bench_errors: Vec<crate::model::result::BenchError> = errors
        .into_iter()
        .map(|error| {
            // Parse benchmark name from error if possible
            if let Some(idx) = error.find("' ") {
                let name = error[1..idx].to_string();
                crate::model::result::BenchError {
                    benchmark_name: name,
                    error_type: "execution_error".to_string(),
                    message: error,
                    timestamp: None,
                }
            } else {
                crate::model::result::BenchError {
                    benchmark_name: "Unknown".to_string(),
                    error_type: "execution_error".to_string(),
                    message: error,
                    timestamp: None,
                }
            }
        })
        .collect();

    // Count completed, failed, and skipped benchmarks
    let completed_benchmarks = scores.len();
    let failed_benchmarks = bench_errors.len();
    let skipped_benchmarks = 0;

    BenchResult {
        scores,
        final_score: aggregated.global,
        cpu_score: aggregated.cpu,
        mem_score: aggregated.mem,
        disk_score: aggregated.disk,
        gfx_score: aggregated.gfx,
        system_info: Some(system_info),
        errors: bench_errors,
        validation: None,
        completed_benchmarks,
        failed_benchmarks,
        skipped_benchmarks,
        start_time: None,
        end_time: None,
        duration_seconds: None,
    }
}
