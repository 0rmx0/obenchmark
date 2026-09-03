use anyhow::Result;
use crate::model::result::SampleResult;

pub trait Benchmark: Send + Sync {
    fn name(&self) -> &str;
    fn weight(&self) -> u64;
    fn run(&self) -> Result<SampleResult>;
}
