pub mod cpu;
pub mod memory;
pub mod disk;

use anyhow::Result;

pub trait Benchmark {
    fn name(&self) -> &'static str;
    fn run(&self) -> Result<u64>;
}