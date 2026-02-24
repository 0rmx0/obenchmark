mod runner;
mod benchmarks;
mod report;

use benchmarks::{cpu::CpuBenchmark, memory::MemoryBenchmark, disk::DiskBenchmark};
use runner::run_async;
use std::thread;
use std::time::Duration;

fn main() {
    let benches: Vec<Box<dyn benchmarks::Benchmark + Send>> = vec![
        Box::new(CpuBenchmark::new()),
        Box::new(MemoryBenchmark),
        Box::new(DiskBenchmark),
    ];

    let rx = run_async(benches);

    println!("Running benchmarks...");

    loop {
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(res) => {
                    report::print_report(&res);
                }
                Err(e) => {
                    eprintln!("Benchmark failed: {:?}", e);
                }
            }
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}