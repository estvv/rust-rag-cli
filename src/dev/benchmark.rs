// src/dev/benchmark.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkResult {
    pub query_id: usize,
    pub query_text: String,
    pub embedding_time_ms: u64,
    pub retrieval_time_ms: u64,
    pub llm_time_ms: u64,
    pub total_time_ms: u64,
    pub chunks_retrieved: usize,
    pub model_name: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Benchmark {
    pub results: Vec<BenchmarkResult>,
    pub start_time: u64,
    pub end_time: u64,
}

impl Benchmark {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            end_time: 0,
        }
    }

    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    pub fn finalize(&mut self) {
        self.end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    pub fn average_times(&self) -> HashMap<String, f64> {
        if self.results.is_empty() {
            return HashMap::new();
        }

        let len = self.results.len() as f64;
        let sum = self.results.iter().fold(
            (0u64, 0u64, 0u64, 0u64),
            |acc, r| {
                (
                    acc.0 + r.embedding_time_ms,
                    acc.1 + r.retrieval_time_ms,
                    acc.2 + r.llm_time_ms,
                    acc.3 + r.total_time_ms,
                )
            },
        );

        let mut map = HashMap::new();
        map.insert("embedding_time_ms".to_string(), sum.0 as f64 / len);
        map.insert("retrieval_time_ms".to_string(), sum.1 as f64 / len);
        map.insert("llm_time_ms".to_string(), sum.2 as f64 / len);
        map.insert("total_time_ms".to_string(), sum.3 as f64 / len);
        map
    }

    pub fn min_times(&self) -> HashMap<String, u64> {
        if self.results.is_empty() {
            return HashMap::new();
        }

        let mut map = HashMap::new();
        map.insert("embedding_time_ms".to_string(), self.results.iter().map(|r| r.embedding_time_ms).min().unwrap_or(0));
        map.insert("retrieval_time_ms".to_string(), self.results.iter().map(|r| r.retrieval_time_ms).min().unwrap_or(0));
        map.insert("llm_time_ms".to_string(), self.results.iter().map(|r| r.llm_time_ms).min().unwrap_or(0));
        map.insert("total_time_ms".to_string(), self.results.iter().map(|r| r.total_time_ms).min().unwrap_or(0));
        map
    }

    pub fn max_times(&self) -> HashMap<String, u64> {
        if self.results.is_empty() {
            return HashMap::new();
        }

        let mut map = HashMap::new();
        map.insert("embedding_time_ms".to_string(), self.results.iter().map(|r| r.embedding_time_ms).max().unwrap_or(0));
        map.insert("retrieval_time_ms".to_string(), self.results.iter().map(|r| r.retrieval_time_ms).max().unwrap_or(0));
        map.insert("llm_time_ms".to_string(), self.results.iter().map(|r| r.llm_time_ms).max().unwrap_or(0));
        map.insert("total_time_ms".to_string(), self.results.iter().map(|r| r.total_time_ms).max().unwrap_or(0));
        map
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let benchmark: Self = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(benchmark)
    }
}

pub fn collect_benchmark_output(benchmark: &Benchmark) -> String {
    let avg = benchmark.average_times();
    let min = benchmark.min_times();
    let max = benchmark.max_times();

    let total_queries = benchmark.results.len();
    let total_time: u64 = benchmark.results.iter().map(|r| r.total_time_ms).sum();

    let mut output = String::new();
    output.push_str(&format!("=== Benchmark Report ===\n"));
    output.push_str(&format!("Total queries: {}\n", total_queries));
    output.push_str(&format!("Total time: {}ms\n\n", total_time));

    output.push_str(&format!("=== Average Times (ms) ===\n"));
    output.push_str(&format!("Embedding:    {:.2}\n", avg.get("embedding_time_ms").unwrap_or(&0.0)));
    output.push_str(&format!("Retrieval:    {:.2}\n", avg.get("retrieval_time_ms").unwrap_or(&0.0)));
    output.push_str(&format!("LLM:          {:.2}\n", avg.get("llm_time_ms").unwrap_or(&0.0)));
    output.push_str(&format!("Total:        {:.2}\n\n", avg.get("total_time_ms").unwrap_or(&0.0)));

    output.push_str(&format!("=== Min Times (ms) ===\n"));
    output.push_str(&format!("Embedding:    {}\n", min.get("embedding_time_ms").unwrap_or(&0)));
    output.push_str(&format!("Retrieval:    {}\n", min.get("retrieval_time_ms").unwrap_or(&0)));
    output.push_str(&format!("LLM:          {}\n", min.get("llm_time_ms").unwrap_or(&0)));
    output.push_str(&format!("Total:        {}\n\n", max.get("total_time_ms").unwrap_or(&0)));

    output.push_str(&format!("=== Max Times (ms) ===\n"));
    output.push_str(&format!("Embedding:    {}\n", max.get("embedding_time_ms").unwrap_or(&0)));
    output.push_str(&format!("Retrieval:    {}\n", max.get("retrieval_time_ms").unwrap_or(&0)));
    output.push_str(&format!("LLM:          {}\n", max.get("llm_time_ms").unwrap_or(&0)));
    output.push_str(&format!("Total:        {}\n", max.get("total_time_ms").unwrap_or(&0)));

    output
}
