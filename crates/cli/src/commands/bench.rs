use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;

use ast::AstNode;
use infer::InferDiagnostic;
use parse::ParseError;
use runtime::{ExecutionMode, Runtime};

#[derive(Diagnostic, Debug, Error)]
#[error("parse errors")]
struct ParseErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<ParseError>,
}

#[derive(Diagnostic, Debug, Error)]
#[error("type errors")]
struct TypeErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<InferDiagnostic>,
}

#[derive(Args)]
pub struct BenchCmd {
    /// Script to benchmark
    pub script: PathBuf,

    /// Number of iterations per mode
    #[arg(short = 'n', long, default_value = "10")]
    pub iterations: u32,

    /// Warmup iterations (excluded from timing)
    #[arg(short, long, default_value = "3")]
    pub warmup: u32,
}

struct BenchResult {
    mode: &'static str,
    min: Duration,
    avg: Duration,
    max: Duration,
    compile_time: Option<Duration>,
}

impl BenchCmd {
    pub fn run(self) -> Result<()> {
        let src = std::fs::read_to_string(&self.script).map_err(|e| miette::miette!("{e}"))?;

        // Parse and compile once to validate and get MIR
        let (tree, errors) = parse::parse(&src);
        if !errors.is_empty() {
            return Err(ParseErrors {
                src: src.clone(),
                errors,
            }
            .into());
        }

        let root = ast::Root::cast(tree).ok_or_else(|| miette::miette!("invalid syntax tree"))?;
        let lower = hir::lower(root);
        let inferred = infer::infer(&lower);

        if inferred.has_errors() {
            return Err(TypeErrors {
                src: src.clone(),
                errors: inferred.diagnostics,
            }
            .into());
        }

        let mir_module = mir::lower(&lower, &inferred);

        println!(
            "Benchmark: {} ({} iterations, {} warmup)\n",
            self.script.display(),
            self.iterations,
            self.warmup
        );

        // Benchmark VM
        let vm_result = self.bench_mode(ExecutionMode::Vm, &mir_module, &lower, &inferred, "VM")?;

        // Benchmark JIT (measure compile time separately)
        let jit_result = self.bench_jit(&mir_module, &lower, &inferred)?;

        // Benchmark Tiered
        let tiered_result = self.bench_mode(
            ExecutionMode::Tiered,
            &mir_module,
            &lower,
            &inferred,
            "Tiered",
        )?;

        // Print results table
        println!("         {:>10} {:>10} {:>10}", "min", "avg", "max");
        self.print_result(&vm_result);
        self.print_result(&jit_result);
        self.print_result(&tiered_result);

        // Print comparison
        println!();
        let speedup = vm_result.avg.as_nanos() as f64 / jit_result.avg.as_nanos() as f64;
        if speedup > 1.0 {
            println!("JIT is {:.1}x faster than VM", speedup);
        } else {
            println!("VM is {:.1}x faster than JIT", 1.0 / speedup);
        }

        Ok(())
    }

    fn bench_mode(
        &self,
        mode: ExecutionMode,
        mir_module: &mir::Module,
        lower: &hir::LowerResult,
        inferred: &infer::InferenceResult,
        name: &'static str,
    ) -> Result<BenchResult> {
        let mut times = Vec::with_capacity(self.iterations as usize);

        // Warmup
        for _ in 0..self.warmup {
            let mut runtime = Runtime::new(mode);
            let _ = runtime.execute(mir_module, lower, inferred);
        }

        // Benchmark
        for _ in 0..self.iterations {
            let mut runtime = Runtime::new(mode);
            let start = Instant::now();
            let _ = runtime.execute(mir_module, lower, inferred);
            times.push(start.elapsed());
        }

        Ok(BenchResult {
            mode: name,
            min: *times.iter().min().unwrap(),
            avg: times.iter().sum::<Duration>() / times.len() as u32,
            max: *times.iter().max().unwrap(),
            compile_time: None,
        })
    }

    fn bench_jit(
        &self,
        mir_module: &mir::Module,
        lower: &hir::LowerResult,
        inferred: &infer::InferenceResult,
    ) -> Result<BenchResult> {
        // Measure compile time once
        let compile_start = Instant::now();
        let mut runtime = Runtime::new(ExecutionMode::Jit);
        let _ = runtime.execute(mir_module, lower, inferred);
        let compile_time = compile_start.elapsed();

        let mut times = Vec::with_capacity(self.iterations as usize);

        // Warmup (already compiled on first exec, so just run)
        for _ in 0..self.warmup {
            let mut runtime = Runtime::new(ExecutionMode::Jit);
            let _ = runtime.execute(mir_module, lower, inferred);
        }

        // Benchmark
        for _ in 0..self.iterations {
            let mut runtime = Runtime::new(ExecutionMode::Jit);
            let start = Instant::now();
            let _ = runtime.execute(mir_module, lower, inferred);
            times.push(start.elapsed());
        }

        Ok(BenchResult {
            mode: "JIT",
            min: *times.iter().min().unwrap(),
            avg: times.iter().sum::<Duration>() / times.len() as u32,
            max: *times.iter().max().unwrap(),
            compile_time: Some(compile_time),
        })
    }

    fn print_result(&self, result: &BenchResult) {
        let compile_str = result
            .compile_time
            .map(|t| format!("   (compile: {})", format_duration(t)))
            .unwrap_or_default();

        println!(
            "{:<6} {:>10} {:>10} {:>10}{}",
            result.mode,
            format_duration(result.min),
            format_duration(result.avg),
            format_duration(result.max),
            compile_str
        );
    }
}

fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.2}us", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", nanos as f64 / 1_000_000_000.0)
    }
}
