use std::fs;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use miette::Result;

use bench::{BenchRunner, ExecutionMode, PipelineStats};

/// Execution mode for benchmarks.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum BenchMode {
    /// Run only VM.
    Vm,
    /// Run only JIT.
    Jit,
    /// Run both VM and JIT (default).
    #[default]
    Both,
}

impl From<BenchMode> for ExecutionMode {
    fn from(mode: BenchMode) -> Self {
        match mode {
            BenchMode::Vm => ExecutionMode::Vm,
            BenchMode::Jit => ExecutionMode::Jit,
            BenchMode::Both => ExecutionMode::Both,
        }
    }
}

#[derive(Args)]
pub struct BenchCmd {
    /// Run only a specific script (otherwise runs all in benchmarks/scripts/).
    #[arg(short, long)]
    script: Option<PathBuf>,

    /// Execution mode: vm, jit, or both.
    #[arg(short, long, value_enum, default_value = "both")]
    mode: BenchMode,

    /// Number of iterations.
    #[arg(short = 'n', long, default_value = "10")]
    iterations: usize,

    /// Output format: text, json, csv.
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Compare with baseline file.
    #[arg(long)]
    baseline: Option<PathBuf>,

    /// Save results as new baseline.
    #[arg(long)]
    save_baseline: Option<PathBuf>,
}

impl BenchCmd {
    pub fn run(self) -> Result<()> {
        let scripts = self.collect_scripts()?;
        let mode: ExecutionMode = self.mode.into();

        println!(
            "Running {} iterations for {} scripts...\n",
            self.iterations,
            scripts.len()
        );

        let mut results = Vec::new();

        for (name, source) in &scripts {
            println!("═══ {} ═══", name);

            let mut samples = Vec::with_capacity(self.iterations);
            let mut first_result = None;
            let mut code_size = None;

            for i in 0..self.iterations {
                let bench_result = BenchRunner::run_timed(source, mode);

                if i == 0 {
                    first_result = Some(bench_result.value);
                    code_size = Some(bench_result.code_size);
                }

                samples.push(bench_result.timings);
            }

            let stats = PipelineStats::from_samples(&samples);

            // Print result
            if let Some(result) = &first_result {
                match result {
                    Ok((v, heap)) => println!("  result: {}", v.display(heap)),
                    Err(e) => println!("  error: {}", e),
                }
            }

            // Print timings (avg)
            println!("{}", stats.avg.display());

            // Print code size
            if let Some(cs) = &code_size {
                println!("{}", cs.display());
            }

            // Print speedup comparison
            if let (Some(vm), Some(jit)) = (stats.avg.vm_exec, stats.avg.jit_exec) {
                let speedup = vm.as_nanos() as f64 / jit.as_nanos() as f64;
                if speedup > 1.0 {
                    println!("  JIT is {:.1}x faster than VM\n", speedup);
                } else {
                    println!("  VM is {:.1}x faster than JIT\n", 1.0 / speedup);
                }
            } else {
                println!();
            }

            results.push((name.clone(), stats));
        }

        // Compare with baseline if provided
        if let Some(baseline_path) = &self.baseline {
            self.compare_baseline(baseline_path, &results)?;
        }

        // Save baseline if requested
        if let Some(save_path) = &self.save_baseline {
            self.save_baseline_file(save_path, &results)?;
        }

        // Output in requested format
        match self.format.as_str() {
            "json" => self.output_json(&results)?,
            "csv" => self.output_csv(&results)?,
            _ => {} // text already printed
        }

        Ok(())
    }

    fn collect_scripts(&self) -> Result<Vec<(String, String)>> {
        if let Some(script) = &self.script {
            let name = script.file_stem().unwrap().to_string_lossy().to_string();
            let source = fs::read_to_string(script).map_err(|e| miette::miette!("{e}"))?;
            return Ok(vec![(name, source)]);
        }

        // Default: read from benchmarks/scripts/
        let scripts_dir = PathBuf::from("benchmarks/scripts");
        if !scripts_dir.exists() {
            return Err(miette::miette!(
                "benchmarks/scripts/ directory not found. Create it or use --script."
            ));
        }

        let mut scripts = Vec::new();

        for entry in fs::read_dir(&scripts_dir).map_err(|e| miette::miette!("{e}"))? {
            let entry = entry.map_err(|e| miette::miette!("{e}"))?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toy") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let source = fs::read_to_string(&path).map_err(|e| miette::miette!("{e}"))?;
                scripts.push((name, source));
            }
        }

        scripts.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(scripts)
    }

    fn compare_baseline(&self, path: &PathBuf, results: &[(String, PipelineStats)]) -> Result<()> {
        let content = fs::read_to_string(path).map_err(|e| miette::miette!("{e}"))?;
        let baseline: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| miette::miette!("{e}"))?;

        println!("\n═══ Comparison with baseline ═══\n");

        // Support new format with "results" wrapper
        let baseline_results = baseline.get("results").unwrap_or(&baseline);

        for (name, stats) in results {
            if let Some(base) = baseline_results.get(name) {
                let base_vm = base["vm_total_avg_us"].as_f64().unwrap_or(0.0);
                let curr_vm = stats.avg.total_vm().as_micros() as f64;

                let change = ((curr_vm - base_vm) / base_vm) * 100.0;
                let arrow = if change < -5.0 {
                    "↓"
                } else if change > 5.0 {
                    "↑"
                } else {
                    "≈"
                };

                println!(
                    "  {}: {:.1}% {} ({:.0}µs → {:.0}µs)",
                    name,
                    change.abs(),
                    arrow,
                    base_vm,
                    curr_vm
                );
            }
        }

        Ok(())
    }

    fn save_baseline_file(
        &self,
        path: &PathBuf,
        results: &[(String, PipelineStats)],
    ) -> Result<()> {
        let mut results_map = serde_json::Map::new();
        for (name, stats) in results {
            results_map.insert(name.clone(), stats.to_json_map().into());
        }

        let mut root = serde_json::Map::new();
        root.insert("iterations".into(), self.iterations.into());
        root.insert("results".into(), results_map.into());

        let content = serde_json::to_string_pretty(&root).map_err(|e| miette::miette!("{e}"))?;
        fs::write(path, content).map_err(|e| miette::miette!("{e}"))?;
        println!("\nBaseline saved to {:?}", path);

        Ok(())
    }

    fn output_json(&self, results: &[(String, PipelineStats)]) -> Result<()> {
        let mut results_map = serde_json::Map::new();
        for (name, stats) in results {
            results_map.insert(name.clone(), stats.to_json_map().into());
        }

        let mut root = serde_json::Map::new();
        root.insert("iterations".into(), self.iterations.into());
        root.insert("results".into(), results_map.into());

        let json = serde_json::to_string_pretty(&root).map_err(|e| miette::miette!("{e}"))?;
        println!("\n{}", json);
        Ok(())
    }

    fn output_csv(&self, results: &[(String, PipelineStats)]) -> Result<()> {
        println!(
            "\nname,lex_avg_us,lex_stddev_us,parse_avg_us,parse_stddev_us,hir_avg_us,hir_stddev_us,infer_avg_us,infer_stddev_us,mir_avg_us,mir_stddev_us,compile_avg_us,compile_stddev_us,vm_exec_avg_us,vm_exec_stddev_us,vm_total_avg_us,jit_compile_avg_us,jit_compile_stddev_us,jit_exec_avg_us,jit_exec_stddev_us,jit_total_avg_us"
        );
        for (name, s) in results {
            let avg = &s.avg;
            let sd = &s.stddev;
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                name,
                avg.lex.as_micros(),
                sd.lex.as_micros(),
                avg.parse.as_micros(),
                sd.parse.as_micros(),
                avg.hir.as_micros(),
                sd.hir.as_micros(),
                avg.infer.as_micros(),
                sd.infer.as_micros(),
                avg.mir.as_micros(),
                sd.mir.as_micros(),
                avg.compile.as_micros(),
                sd.compile.as_micros(),
                avg.vm_exec.map(|d| d.as_micros()).unwrap_or(0),
                sd.vm_exec.map(|d| d.as_micros()).unwrap_or(0),
                avg.total_vm().as_micros(),
                avg.jit_compile.map(|d| d.as_micros()).unwrap_or(0),
                sd.jit_compile.map(|d| d.as_micros()).unwrap_or(0),
                avg.jit_exec.map(|d| d.as_micros()).unwrap_or(0),
                sd.jit_exec.map(|d| d.as_micros()).unwrap_or(0),
                avg.total_jit().as_micros(),
            );
        }
        Ok(())
    }
}
