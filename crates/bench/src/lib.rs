//! Benchmark utilities for the toy language pipeline.

mod runner;
mod timings;

use std::fs;
use std::path::Path;

pub use runner::{BenchRunner, ExecutionMode};
pub use timings::{CodeSizeMetrics, PipelineStats, PipelineTimings};

/// Load all benchmark scripts from `benchmarks/scripts/`.
/// Returns Vec of (name, source) sorted by name.
pub fn load_scripts() -> Vec<(String, String)> {
    load_scripts_from(Path::new("benchmarks/scripts"))
}

/// Load benchmark scripts from a specific directory.
pub fn load_scripts_from(dir: &Path) -> Vec<(String, String)> {
    let mut scripts = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toy") {
                if let Some(name) = path.file_stem() {
                    if let Ok(source) = fs::read_to_string(&path) {
                        scripts.push((name.to_string_lossy().to_string(), source));
                    }
                }
            }
        }
    }

    scripts.sort_by(|a, b| a.0.cmp(&b.0));
    scripts
}
