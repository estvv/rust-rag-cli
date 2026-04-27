// src/dev/mod.rs

mod debug;
mod benchmark;

pub use debug::init_debug;
pub use debug::get_logs;
pub use benchmark::{Benchmark, BenchmarkResult, collect_benchmark_output};
