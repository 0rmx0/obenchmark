use crate::runner::BenchResult;

pub fn print_report(result: &BenchResult) {
    println!("\n===== OBenchmark Professional Report =====");
    for (name, score) in &result.scores {
        println!("{:<20} {}", name, score);
    }
    println!("-------------------------------------------");
    println!("Global Score: {}", result.global_score);
}