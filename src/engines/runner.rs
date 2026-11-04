use crate::benchmarks::suite::BenchSpec;
use crate::engines::score::compute_aggregated_scores;
use crate::model::result::{BenchResult, BenchScore};
use crate::util::config::{set_run_config, with_run_config};
use crate::util::sysinfo::get_detailed_system_info;
use chrono::Local;
use crossbeam_channel::Sender;

pub enum RunnerEvent {
    BenchStarted(String),
    /// Intra-benchmark progress in 0.0..=1.0.
    Progress(f32),
    BenchFinished(String, u64),
    /// A benchmark failed but the run continues.
    BenchFailed(String, String),
    Done(Box<BenchResult>),
    Error(String),
}

pub fn run_benchmarks(specs: Vec<BenchSpec>, tx: Sender<RunnerEvent>) {
    std::thread::spawn(move || {
        if specs.is_empty() {
            tx.send(RunnerEvent::Error("no benchmarks selected".to_string()))
                .ok();
            return;
        }

        // Route intra-benchmark progress through a dedicated channel.
        let (ptx, prx) = crossbeam_channel::unbounded::<f32>();
        let mut cfg = with_run_config(|c| c.clone());
        cfg.progress_tx = Some(ptx);
        set_run_config(cfg);

        let relay_tx = tx.clone();
        std::thread::spawn(move || {
            while let Ok(frac) = prx.recv() {
                if relay_tx.send(RunnerEvent::Progress(frac)).is_err() {
                    return;
                }
            }
        });

        let mut scores: Vec<BenchScore> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        for spec in &specs {
            tx.send(RunnerEvent::BenchStarted(spec.name.to_string()))
                .ok();

            let bench = spec.build();
            debug_assert_eq!(bench.name(), spec.name);
            match bench.run() {
                Ok(score) => {
                    scores.push(BenchScore {
                        name: spec.name.to_string(),
                        raw_score: score,
                        weight: spec.weight,
                    });
                    tx.send(RunnerEvent::BenchFinished(spec.name.to_string(), score))
                        .ok();
                }
                Err(e) => {
                    let msg = format!("{}: {}", spec.name, e);
                    errors.push(msg.clone());
                    tx.send(RunnerEvent::BenchFailed(spec.name.to_string(), msg))
                        .ok();
                }
            }
        }

        // Stop relaying progress.
        with_run_config(|c| {
            let mut cfg = c.clone();
            cfg.progress_tx = None;
            set_run_config(cfg);
        });

        tx.send(RunnerEvent::Done(Box::new(build_bench_result(
            scores, errors,
        ))))
        .ok();
    });
}

pub fn build_bench_result(scores: Vec<BenchScore>, errors: Vec<String>) -> BenchResult {
    let aggregated = compute_aggregated_scores(&scores);
    let system_info = get_detailed_system_info();

    BenchResult {
        scores,
        final_score: aggregated.global,
        cpu_score: aggregated.cpu,
        mem_score: aggregated.mem,
        disk_score: aggregated.disk,
        errors,
        timestamp: Some(Local::now().to_rfc3339()),
        system_info: Some(system_info),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_bench_result_empty_scores() {
        let result = build_bench_result(vec![], vec![]);
        assert_eq!(result.scores.len(), 0);
        assert_eq!(result.final_score, 0);
        assert_eq!(result.cpu_score, 0);
        assert_eq!(result.mem_score, 0);
        assert_eq!(result.disk_score, 0);
        assert!(result.system_info.is_some());
        assert!(result.timestamp.is_some());
        assert!(result.errors.is_empty());
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
        let result = build_bench_result(scores, vec!["Disk Seq Write: boom".to_string()]);
        assert_eq!(result.scores.len(), 3);
        assert!(result.final_score > 0);
        assert!(result.cpu_score > 0);
        assert!(result.mem_score > 0);
        assert!(result.disk_score > 0);
        assert_eq!(result.errors, vec!["Disk Seq Write: boom".to_string()]);
    }

    #[test]
    fn build_bench_result_only_cpu() {
        let scores = vec![BenchScore {
            name: "CPU Multi-Core".to_string(),
            raw_score: 80_000_000,
            weight: 3,
        }];
        let result = build_bench_result(scores, vec![]);
        assert!(result.cpu_score > 0);
        assert_eq!(result.mem_score, 0);
        assert_eq!(result.disk_score, 0);
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
        let result = build_bench_result(original_scores.clone(), vec![]);
        assert_eq!(result.scores.len(), original_scores.len());
        for (i, score) in result.scores.iter().enumerate() {
            assert_eq!(score.name, original_scores[i].name);
            assert_eq!(score.raw_score, original_scores[i].raw_score);
            assert_eq!(score.weight, original_scores[i].weight);
        }
    }
}
