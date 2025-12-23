//! Criterion benchmarks for pipeline stages.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use ast::AstNode;

fn bench_lex(c: &mut Criterion) {
    let mut group = c.benchmark_group("lex");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        group.bench_with_input(BenchmarkId::new("lex", name), source, |b, src| {
            b.iter(|| {
                let _: Vec<_> = lex::Lexer::new(src).collect();
            });
        });
    }

    group.finish();
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        group.bench_with_input(BenchmarkId::new("parse", name), source, |b, src| {
            b.iter(|| parse::parse(src));
        });
    }

    group.finish();
}

fn bench_full_frontend(c: &mut Criterion) {
    let mut group = c.benchmark_group("frontend");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        group.bench_with_input(BenchmarkId::new("full", name), source, |b, src| {
            b.iter(|| {
                let (syntax, _) = parse::parse(src);
                let root = ast::Root::cast(syntax).unwrap();
                let lower = hir::lower(root);
                infer::infer(&lower)
            });
        });
    }

    group.finish();
}

fn bench_mir(c: &mut Criterion) {
    let mut group = c.benchmark_group("mir");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        // Pre-compute frontend
        let (syntax, _) = parse::parse(source);
        let root = ast::Root::cast(syntax).unwrap();
        let lower = hir::lower(root);
        let inferred = infer::infer(&lower);

        group.bench_with_input(
            BenchmarkId::new("lower", name),
            &(&lower, &inferred),
            |b, (lower, inferred)| {
                b.iter(|| mir::lower(lower, inferred));
            },
        );
    }

    group.finish();
}

fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile");
    let scripts = bench::load_scripts();

    for (name, source) in &scripts {
        // Pre-compute frontend + MIR
        let (syntax, _) = parse::parse(source);
        let root = ast::Root::cast(syntax).unwrap();
        let lower = hir::lower(root);
        let inferred = infer::infer(&lower);
        let mir_module = mir::lower(&lower, &inferred);

        group.bench_with_input(BenchmarkId::new("bytecode", name), &mir_module, |b, mir| {
            b.iter(|| compile::compile(mir));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_lex,
    bench_parse,
    bench_full_frontend,
    bench_mir,
    bench_compile
);
criterion_main!(benches);
