use std::path::PathBuf;

use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;

use ast::AstNode;
use hir::{Definition, Expression, InfixOp, Item, Literal, PrefixOp};
use infer::{InferDiagnostic, InferenceResult, Type};
use parse::ParseError;

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
pub struct RunCmd {
    /// Script to run
    pub script: PathBuf,
}

impl RunCmd {
    pub fn run(self) -> Result<()> {
        let src = std::fs::read_to_string(&self.script).map_err(|e| miette::miette!("{e}"))?;
        process(&src)
    }
}

pub fn process(src: &str) -> Result<()> {
    let (tree, errors) = parse::parse(src);
    if !errors.is_empty() {
        return Err(ParseErrors {
            src: src.to_string(),
            errors,
        }
        .into());
    }

    let root = ast::Root::cast(tree).ok_or_else(|| miette::miette!("invalid syntax tree"))?;
    let lower = hir::lower(root);
    let inferred = infer::infer(&lower);

    if inferred.has_errors() {
        return Err(TypeErrors {
            src: src.to_string(),
            errors: inferred.diagnostics,
        }
        .into());
    }

    let mir_module = mir::lower(&lower, &inferred);
    let result_type = lower
        .items
        .last()
        .map(|item| get_item_type(item, &inferred));

    let jit_ret_type = match result_type {
        Some(Type::Float) => jit::ReturnType::Float,
        Some(Type::Boolean) => jit::ReturnType::Boolean,
        _ => jit::ReturnType::Integer,
    };

    let mut jit_compiler = jit::Jit::new();
    let ptr = jit_compiler
        .compile(&mir_module.main, jit_ret_type)
        .map_err(|e| miette::miette!("{e}"))?;

    match result_type {
        Some(Type::Float) => {
            let result: f64 = unsafe {
                let func: fn() -> f64 = std::mem::transmute(ptr);
                func()
            };
            println!("{}", result);
        }
        Some(Type::Boolean) => {
            let result: i64 = unsafe {
                let func: fn() -> i64 = std::mem::transmute(ptr);
                func()
            };
            println!("{}", if result != 0 { "true" } else { "false" });
        }
        _ => {
            let result: i64 = unsafe {
                let func: fn() -> i64 = std::mem::transmute(ptr);
                func()
            };
            println!("{}", result);
        }
    }

    Ok(())
}

fn get_item_type(item: &Item, inferred: &InferenceResult) -> Type {
    match item {
        Item::Definition(Definition::Variable { name, .. }) => inferred
            .get_variable_type(name)
            .cloned()
            .unwrap_or(Type::Integer),
        Item::Assignment { name, .. } => inferred
            .get_variable_type(name)
            .cloned()
            .unwrap_or(Type::Integer),
        Item::Expression(expr) => get_expr_type(expr, inferred),
    }
}

fn get_expr_type(expr: &Expression, inferred: &InferenceResult) -> Type {
    match expr {
        Expression::Missing => Type::Integer,
        Expression::Literal(lit) => match lit {
            Literal::Integer(_) => Type::Integer,
            Literal::Float(_) => Type::Float,
            Literal::Boolean(_) => Type::Boolean,
            Literal::String(_) => Type::String,
        },
        Expression::Infix { op, .. } => match op {
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div | InfixOp::Mod => {
                Type::Integer
            }
            InfixOp::AddFloat | InfixOp::SubFloat | InfixOp::MulFloat | InfixOp::DivFloat => {
                Type::Float
            }
            InfixOp::Eq
            | InfixOp::NotEq
            | InfixOp::Gt
            | InfixOp::Lt
            | InfixOp::Gte
            | InfixOp::Lte
            | InfixOp::GtFloat
            | InfixOp::LtFloat
            | InfixOp::GteFloat
            | InfixOp::LteFloat
            | InfixOp::And
            | InfixOp::Or => Type::Boolean,
        },
        Expression::Prefix { op, .. } => match op {
            PrefixOp::Neg => Type::Integer,
            PrefixOp::Not => Type::Boolean,
        },
        Expression::VariableRef { name } => inferred
            .get_variable_type(name)
            .cloned()
            .unwrap_or(Type::Integer),
    }
}
