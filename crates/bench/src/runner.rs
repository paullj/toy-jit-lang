//! Benchmark runner for pipeline stages.

use crate::timings::{CodeSizeMetrics, PipelineTimings};
use std::time::Instant;

use ast::AstNode;
use hir::LowerResult;
use infer::InferenceResult;

/// Execution mode for benchmarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Run only VM.
    Vm,
    /// Run only JIT.
    Jit,
    /// Run both VM and JIT.
    Both,
}

/// Result of a benchmark run.
pub struct BenchResult {
    pub timings: PipelineTimings,
    pub code_size: CodeSizeMetrics,
    pub value: Result<(vm::Value, vm::Heap), Box<dyn std::error::Error + Send + Sync>>,
}

/// Benchmark runner for the toy language pipeline.
pub struct BenchRunner;

impl BenchRunner {
    /// Run the full pipeline with timing for each stage.
    pub fn run_timed(source: &str, mode: ExecutionMode) -> BenchResult {
        let mut timings = PipelineTimings::default();
        let mut code_size = CodeSizeMetrics::default();

        // Lex
        let start = Instant::now();
        let _tokens: Vec<_> = lex::Lexer::new(source).collect();
        timings.lex = start.elapsed();

        // Parse
        let start = Instant::now();
        let (syntax, parse_errors) = parse::parse(source);
        timings.parse = start.elapsed();

        if !parse_errors.is_empty() {
            return BenchResult {
                timings,
                code_size,
                value: Err("parse error".into()),
            };
        }

        // HIR
        let start = Instant::now();
        let root = match ast::Root::cast(syntax) {
            Some(r) => r,
            None => {
                return BenchResult {
                    timings,
                    code_size,
                    value: Err("invalid syntax tree".into()),
                }
            }
        };
        let lower_result = hir::lower(root);
        timings.hir = start.elapsed();

        // Infer
        let start = Instant::now();
        let infer_result = infer::infer(&lower_result);
        timings.infer = start.elapsed();

        if infer_result.has_errors() {
            return BenchResult {
                timings,
                code_size,
                value: Err("type error".into()),
            };
        }

        // MIR
        let start = Instant::now();
        let mir_module = mir::lower(&lower_result, &infer_result);
        timings.mir = start.elapsed();

        match mode {
            ExecutionMode::Vm => Self::run_vm(&mut timings, &mut code_size, &mir_module),
            ExecutionMode::Jit => {
                Self::run_jit(&mut timings, &mir_module, &lower_result, &infer_result)
            }
            ExecutionMode::Both => {
                // Run VM first
                let vm_result = Self::run_vm(&mut timings, &mut code_size, &mir_module);

                // Run JIT (keep VM result)
                let jit_result =
                    Self::run_jit(&mut timings, &mir_module, &lower_result, &infer_result);

                // Return VM result but with both timings
                BenchResult {
                    timings: vm_result.timings.add(&PipelineTimings {
                        jit_warmup: jit_result.timings.jit_warmup,
                        jit_compile: jit_result.timings.jit_compile,
                        jit_exec: jit_result.timings.jit_exec,
                        ..Default::default()
                    }),
                    code_size: vm_result.code_size,
                    value: vm_result.value,
                }
            }
        }
    }

    fn run_vm(
        timings: &mut PipelineTimings,
        code_size: &mut CodeSizeMetrics,
        mir_module: &mir::Module,
    ) -> BenchResult {
        // Compile to bytecode
        let start = Instant::now();
        let compiled = compile::compile(mir_module);
        timings.compile = start.elapsed();

        // Collect code size metrics
        code_size.function_count = compiled.chunks.len();
        code_size.bytecode_instructions = compiled.chunks.iter().map(|c| c.code.len()).sum();
        code_size.constant_count = compiled.chunks.iter().map(|c| c.constants.len()).sum();

        // Execute
        let start = Instant::now();
        let result = vm::run(&compiled);
        timings.vm_exec = Some(start.elapsed());

        // Extract GC stats from heap
        if let Ok((_, ref heap)) = result {
            let gc_stats = heap.stats();
            timings.vm_gc_time = Some(gc_stats.gc_time);
            timings.vm_gc_collections = Some(gc_stats.collections);
        }

        BenchResult {
            timings: timings.clone(),
            code_size: code_size.clone(),
            value: result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    fn run_jit(
        timings: &mut PipelineTimings,
        mir_module: &mir::Module,
        lower_result: &LowerResult,
        infer_result: &InferenceResult,
    ) -> BenchResult {
        // JIT warmup - first compilation (cold)
        let start = Instant::now();
        let mut runtime_warmup = runtime::Runtime::new(runtime::ExecutionMode::Jit);
        let _ = runtime_warmup.execute(mir_module, lower_result, infer_result);
        timings.jit_warmup = Some(start.elapsed());

        // JIT compile (warm - new instance)
        let start = Instant::now();
        let mut jit = jit::Jit::new();
        jit.compile_module(mir_module).ok();
        timings.jit_compile = Some(start.elapsed());

        // JIT execute
        let start = Instant::now();
        let mut runtime = runtime::Runtime::new(runtime::ExecutionMode::Jit);
        let result = runtime.execute(mir_module, lower_result, infer_result);
        timings.jit_exec = Some(start.elapsed());

        BenchResult {
            timings: timings.clone(),
            code_size: CodeSizeMetrics::default(),
            value: result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    }
}
