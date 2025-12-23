use thiserror::Error;

#[derive(Debug, Error)]
#[allow(clippy::result_large_err)]
pub enum JitError {
    #[error("module error: {0:#}")]
    Module(#[from] Box<cranelift_module::ModuleError>),

    #[error("codegen error: {0:#}")]
    Codegen(#[from] cranelift::codegen::CodegenError),

    #[error("unsupported operand: {0}")]
    UnsupportedOperand(String),
}

impl From<cranelift_module::ModuleError> for JitError {
    fn from(err: cranelift_module::ModuleError) -> Self {
        JitError::Module(Box::new(err))
    }
}
