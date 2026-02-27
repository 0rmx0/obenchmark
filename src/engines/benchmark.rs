use anyhow::Result;

pub trait Benchmark: Send + Sync {
    fn name(&self) -> &str;
    fn weight(&self) -> u64;
    fn run(&self) -> Result<u64>;
}