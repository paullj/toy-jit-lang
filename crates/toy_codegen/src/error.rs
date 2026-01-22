use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("cranelift module error: {0}")]
    Module(#[from] Box<cranelift_module::ModuleError>),

    #[error("unsupported feature in v1: {0}")]
    Unsupported(&'static str),

    #[error("unknown function: {0}")]
    UnknownFunction(toy_mir::FuncId),

    #[error("unknown vreg: {0}")]
    UnknownVReg(toy_mir::VReg),

    #[error("unknown block: {0}")]
    UnknownBlock(toy_mir::BlockId),

    #[error("unknown local: {0}")]
    UnknownLocal(toy_mir::LocalId),
}

impl From<cranelift_module::ModuleError> for CodegenError {
    fn from(err: cranelift_module::ModuleError) -> Self {
        CodegenError::Module(Box::new(err))
    }
}
