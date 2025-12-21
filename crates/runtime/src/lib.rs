//! Tiered runtime for the toy language.
//!
//! Supports multiple execution modes:
//! - VM: Bytecode interpreter (default)
//! - JIT: Native compilation via Cranelift
//! - Tiered: VM with hot-path JIT compilation (scaffolding for future)

mod profiler;

use hir::{Definition, Expression, InfixOp, Item, Literal, LowerResult, PrefixOp};
use infer::{InferenceResult, Type};
use thiserror::Error;

pub use profiler::Profiler;

/// Execution mode for the runtime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Bytecode interpreter only
    Vm,
    /// Native JIT compilation only
    Jit,
    /// VM with hot-path profiling and JIT (default, future)
    #[default]
    Tiered,
}

/// Runtime value (re-exported from vm)
pub use vm::Value;

/// Runtime error
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("vm error: {0}")]
    Vm(#[from] vm::RuntimeError),
    #[error("jit error: {0}")]
    Jit(#[from] jit::JitError),
}

/// Tiered runtime that can execute code via VM, JIT, or a combination.
pub struct Runtime {
    mode: ExecutionMode,
    jit: Option<jit::Jit>,
    profiler: Profiler,
    // Future: compiled function cache
    // compiled_cache: HashMap<FunctionId, *const u8>,
}

impl Runtime {
    /// Create a new runtime with the specified execution mode.
    pub fn new(mode: ExecutionMode) -> Self {
        let jit = match mode {
            ExecutionMode::Vm => None,
            ExecutionMode::Jit | ExecutionMode::Tiered => Some(jit::Jit::new()),
        };

        Self {
            mode,
            jit,
            profiler: Profiler::new(),
        }
    }

    /// Execute a MIR module and return the result.
    pub fn execute(
        &mut self,
        mir_module: &mir::Module,
        hir: &LowerResult,
        inferred: &InferenceResult,
    ) -> Result<Value, RuntimeError> {
        match self.mode {
            ExecutionMode::Vm => self.execute_vm(mir_module),
            ExecutionMode::Jit => self.execute_jit(mir_module, hir, inferred),
            ExecutionMode::Tiered => self.execute_tiered(mir_module, hir, inferred),
        }
    }

    fn execute_vm(&self, mir_module: &mir::Module) -> Result<Value, RuntimeError> {
        let compiled = compile::compile(mir_module);
        let result = vm::run(&compiled)?;
        Ok(result)
    }

    fn execute_jit(
        &mut self,
        mir_module: &mir::Module,
        hir: &LowerResult,
        inferred: &InferenceResult,
    ) -> Result<Value, RuntimeError> {
        let jit_compiler = self.jit.as_mut().expect("JIT not initialized");

        let result_type = hir.items.last().map(|item| get_item_type(item, inferred));

        let jit_ret_type = match result_type {
            Some(Type::Float) => jit::ReturnType::Float,
            Some(Type::Boolean) => jit::ReturnType::Boolean,
            _ => jit::ReturnType::Integer,
        };

        let ptr = jit_compiler.compile(&mir_module.main, jit_ret_type)?;

        let value = match result_type {
            Some(Type::Float) => {
                let result: f64 = unsafe {
                    let func: fn() -> f64 = std::mem::transmute(ptr);
                    func()
                };
                Value::Float(result)
            }
            Some(Type::Boolean) => {
                let result: i64 = unsafe {
                    let func: fn() -> i64 = std::mem::transmute(ptr);
                    func()
                };
                Value::Bool(result != 0)
            }
            _ => {
                let result: i64 = unsafe {
                    let func: fn() -> i64 = std::mem::transmute(ptr);
                    func()
                };
                Value::Int(result)
            }
        };

        Ok(value)
    }

    fn execute_tiered(
        &mut self,
        mir_module: &mir::Module,
        _hir: &LowerResult,
        _inferred: &InferenceResult,
    ) -> Result<Value, RuntimeError> {
        // For now, tiered mode just uses VM since we don't have functions yet.
        // When functions are added, this will:
        // 1. Execute via VM
        // 2. Profile function call counts
        // 3. JIT compile hot functions above threshold
        // 4. Replace VM calls with JIT calls for hot paths

        self.profiler.record_execution();

        // TODO: When functions exist, check if any are hot and JIT compile them
        // For now, just run via VM
        self.execute_vm(mir_module)
    }

    /// Get the current execution mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Get a reference to the profiler.
    pub fn profiler(&self) -> &Profiler {
        &self.profiler
    }
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
        Expression::Block { tail, .. } => match tail {
            Some(idx) => inferred
                .expression_types
                .get(*idx)
                .cloned()
                .unwrap_or(Type::Unit),
            None => Type::Unit,
        },
    }
}
