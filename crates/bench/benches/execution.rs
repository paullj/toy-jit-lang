//! Criterion benchmarks for VM vs JIT execution.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use ast::AstNode;

struct CompiledScript {
    mir_module: mir::Module,
    compiled: compile::CompiledModule,
    lower: hir::LowerResult,
    inferred: infer::InferenceResult,
}

fn compile_script(source: &str) -> CompiledScript {
    let (syntax, _) = parse::parse(source);
    let root = ast::Root::cast(syntax).unwrap();
    let lower = hir::lower(root);
    let inferred = infer::infer(&lower);
    let mir_module = mir::lower(&lower, &inferred);
    let compiled = compile::compile(&mir_module);

    CompiledScript {
        mir_module,
        compiled,
        lower,
        inferred,
    }
}

fn bench_vm_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        let compiled = compile_script(source);

        group.bench_with_input(
            BenchmarkId::new("exec", name),
            &compiled.compiled,
            |b, compiled| {
                b.iter(|| vm::run(compiled).unwrap());
            },
        );
    }

    group.finish();
}

fn bench_jit_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        let compiled = compile_script(source);

        group.bench_with_input(
            BenchmarkId::new("compile", name),
            &compiled.mir_module,
            |b, mir| {
                b.iter(|| {
                    let mut jit = jit::Jit::new();
                    jit.compile_module(mir).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_jit_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        let compiled = compile_script(source);

        group.bench_with_input(BenchmarkId::new("exec", name), &compiled, |b, script| {
            b.iter(|| {
                let mut runtime = runtime::Runtime::new(runtime::ExecutionMode::Jit);
                runtime
                    .execute(&script.mir_module, &script.lower, &script.inferred)
                    .unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vm_execution,
    bench_jit_compile,
    bench_jit_execution
);
criterion_main!(benches);
