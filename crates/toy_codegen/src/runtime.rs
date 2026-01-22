use cranelift_codegen::ir::AbiParam;
use cranelift_codegen::ir::types::{F64, I64};
use cranelift_module::{FuncId, Linkage, Module};

use crate::error::CodegenError;

/// External runtime function IDs
pub struct RuntimeFuncs {
    pub echo_int: FuncId,
    pub echo_float: FuncId,
    pub echo_bool: FuncId,
}

impl RuntimeFuncs {
    /// Declare all runtime functions in the module
    pub fn declare<M: Module>(module: &mut M) -> Result<Self, CodegenError> {
        let echo_int = {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(I64));
            module.declare_function("toy_echo_int", Linkage::Import, &sig)?
        };

        let echo_float = {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(F64));
            module.declare_function("toy_echo_float", Linkage::Import, &sig)?
        };

        let echo_bool = {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(I64));
            module.declare_function("toy_echo_bool", Linkage::Import, &sig)?
        };

        Ok(Self {
            echo_int,
            echo_float,
            echo_bool,
        })
    }
}
