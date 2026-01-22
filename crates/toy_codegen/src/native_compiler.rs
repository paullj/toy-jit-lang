use std::collections::HashMap;

use cranelift_codegen::Context;
use cranelift_codegen::ir::AbiParam;
use cranelift_codegen::ir::types::{F64, I64};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::error::CodegenError;
use crate::runtime::RuntimeFuncs;
use crate::translate::FunctionTranslator;

/// Compiled object file bytes
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledObject {
    pub bytes: Vec<u8>,
}

/// Main compiler that drives Cranelift code generation
pub struct NativeCompiler<'a> {
    module: ObjectModule,
    mir: &'a toy_mir::Module,
    func_ids: HashMap<toy_mir::FuncId, FuncId>,
    runtime: RuntimeFuncs,
    #[allow(dead_code)]
    module_id: toy_hir::ModuleId,
    /// External function declarations from other modules
    external_funcs: HashMap<toy_mir::FuncId, FuncId>,
}

impl<'a> NativeCompiler<'a> {
    pub fn new(mir: &'a toy_mir::Module) -> Result<Self, CodegenError> {
        Self::new_with_module_id(mir, toy_hir::ModuleId(0))
    }

    /// Create a compiler for a specific module with external function info
    pub fn new_with_module_id(
        mir: &'a toy_mir::Module,
        module_id: toy_hir::ModuleId,
    ) -> Result<Self, CodegenError> {
        // Setup target ISA
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("is_pic", "true").unwrap();

        let isa_builder = cranelift_native::builder().unwrap();
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        // Create object module with module-specific name
        let module_name = format!("toy_module_{}", module_id.0);
        let object_builder =
            ObjectBuilder::new(isa, module_name, cranelift_module::default_libcall_names())
                .unwrap();
        let mut module = ObjectModule::new(object_builder);

        // Declare runtime functions
        let runtime = RuntimeFuncs::declare(&mut module)?;

        // Declare all user functions
        let mut func_ids = HashMap::new();
        for func in &mir.functions {
            let sig = Self::make_signature(&module, func);
            let name = func.name.as_deref().unwrap_or("_anon");

            // Generate unique symbol name including module ID to avoid conflicts
            let symbol_name = if func.id == mir.main_id {
                // Main function keeps simple name and is exported
                name.to_string()
            } else {
                // Other functions get module prefix for uniqueness
                format!("mod{}_{}", module_id.0, name)
            };

            // Main function gets exported, others are exported for cross-module calls
            let linkage = if func.id == mir.main_id {
                Linkage::Export
            } else {
                // Export all functions so other modules can call them
                Linkage::Export
            };

            let func_id = module.declare_function(&symbol_name, linkage, &sig)?;
            func_ids.insert(func.id, func_id);
        }

        Ok(Self {
            module,
            mir,
            func_ids,
            runtime,
            module_id,
            external_funcs: HashMap::new(),
        })
    }

    /// Declare external functions from other modules
    pub fn declare_external_function(
        &mut self,
        func_id: toy_mir::FuncId,
        name: &str,
        param_count: usize,
        has_return: bool,
    ) -> Result<(), CodegenError> {
        let call_conv = self.module.isa().default_call_conv();
        let mut sig = cranelift_codegen::ir::Signature::new(call_conv);

        // All params are I64 for v1
        for _ in 0..param_count {
            sig.params.push(AbiParam::new(I64));
        }

        // Return type if needed
        if has_return {
            sig.returns.push(AbiParam::new(I64));
        }

        let cranelift_func_id = self.module.declare_function(name, Linkage::Import, &sig)?;
        self.external_funcs.insert(func_id, cranelift_func_id);

        Ok(())
    }

    fn make_signature(
        module: &ObjectModule,
        func: &toy_mir::Function,
    ) -> cranelift_codegen::ir::Signature {
        let call_conv = module.isa().default_call_conv();
        let mut sig = cranelift_codegen::ir::Signature::new(call_conv);

        // All params are I64 for v1 (primitives)
        for _ in 0..func.param_count {
            sig.params.push(AbiParam::new(I64));
        }

        // Return type - check if function has returns with values and determine type
        let return_op = func.blocks.iter().find_map(|b| {
            b.insts.iter().find_map(|i| match i {
                toy_mir::Inst::Return { value: Some(op) } => Some(op),
                _ => None,
            })
        });
        if let Some(op) = return_op {
            let ret_ty = match op {
                toy_mir::Operand::FloatConst(_) => F64,
                toy_mir::Operand::VReg(v) => match func.vreg_types.get(v.0 as usize) {
                    Some(toy_mir::ValueType::Float) => F64,
                    _ => I64,
                },
                _ => I64,
            };
            sig.returns.push(AbiParam::new(ret_ty));
        }

        sig
    }

    pub fn compile(mut self) -> Result<CompiledObject, CodegenError> {
        let mut ctx = Context::new();
        let mut func_ctx = FunctionBuilderContext::new();

        for mir_func in &self.mir.functions {
            let func_id = self.func_ids[&mir_func.id];
            let sig = Self::make_signature(&self.module, mir_func);

            ctx.func.signature = sig.clone();
            ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, mir_func.id.0);

            {
                let builder =
                    cranelift_frontend::FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
                let translator = FunctionTranslator::new_with_external(
                    builder,
                    mir_func,
                    &mut self.module,
                    &self.runtime,
                    &self.func_ids,
                    &self.external_funcs,
                );
                translator.translate()?;
            }

            self.module.define_function(func_id, &mut ctx)?;
            ctx.clear();
        }

        let product = self.module.finish();
        let bytes = product.emit().unwrap();

        Ok(CompiledObject { bytes })
    }
}

/// Compile MIR to native object file
pub fn compile_native(mir: &toy_mir::Module) -> Result<CompiledObject, CodegenError> {
    let compiler = NativeCompiler::new(mir)?;
    compiler.compile()
}

/// Compile MIR module with module ID and external module info
pub fn compile_module(
    mir: &toy_mir::Module,
    module_id: toy_hir::ModuleId,
    external_modules: &[(toy_hir::ModuleId, &toy_mir::Module)],
) -> Result<CompiledObject, CodegenError> {
    let mut compiler = NativeCompiler::new_with_module_id(mir, module_id)?;

    // Declare all external functions from other modules
    for (ext_module_id, ext_mir) in external_modules {
        if *ext_module_id == module_id {
            continue; // Skip self
        }

        for func in &ext_mir.functions {
            let name = func.name.as_deref().unwrap_or("_anon");
            let symbol_name = if func.id == ext_mir.main_id {
                name.to_string()
            } else {
                format!("mod{}_{}", ext_module_id.0, name)
            };

            // Determine if function has return value
            let has_return = func.blocks.iter().any(|b| {
                b.insts
                    .iter()
                    .any(|i| matches!(i, toy_mir::Inst::Return { value: Some(_) }))
            });

            compiler.declare_external_function(
                func.id,
                &symbol_name,
                func.param_count as usize,
                has_return,
            )?;
        }
    }

    compiler.compile()
}

#[cfg(test)]
mod tests {
    use super::*;
    use toy_mir::{Block, BlockId, FuncId, Function, Inst, Operand, VReg, ValueType};

    #[test]
    fn test_compile_simple_add() {
        let mut func = Function::new(FuncId(0), Some("main".to_string()));
        let mut block = Block::new(BlockId(0));
        block.push(Inst::AddInt {
            dst: VReg(0),
            lhs: Operand::IntConst(1),
            rhs: Operand::IntConst(2),
        });
        block.push(Inst::Return { value: None });
        func.blocks.push(block);
        func.vreg_count = 1;

        let module = toy_mir::Module {
            functions: vec![func],
            main_id: FuncId(0),
        };

        let result = compile_native(&module);
        assert!(result.is_ok());
        let obj = result.unwrap();
        assert!(!obj.bytes.is_empty());
    }

    #[test]
    fn test_compile_echo_int() {
        let mut func = Function::new(FuncId(0), Some("main".to_string()));
        let mut block = Block::new(BlockId(0));
        block.push(Inst::Echo {
            src: Operand::IntConst(42),
            ty: ValueType::Int,
        });
        block.push(Inst::Return { value: None });
        func.blocks.push(block);

        let module = toy_mir::Module {
            functions: vec![func],
            main_id: FuncId(0),
        };

        let result = compile_native(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_function_call() {
        // Helper function: fn add(a, b) -> a + b
        let mut add_func = Function::new(FuncId(0), Some("add".to_string()));
        add_func.param_count = 2;
        add_func.local_count = 2;
        add_func.params = vec![toy_mir::LocalId(0), toy_mir::LocalId(1)];
        let mut add_block = Block::new(BlockId(0));
        add_block.push(Inst::LoadLocal {
            dst: VReg(0),
            local: toy_mir::LocalId(0),
        });
        add_block.push(Inst::LoadLocal {
            dst: VReg(1),
            local: toy_mir::LocalId(1),
        });
        add_block.push(Inst::AddInt {
            dst: VReg(2),
            lhs: Operand::VReg(VReg(0)),
            rhs: Operand::VReg(VReg(1)),
        });
        add_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(2))),
        });
        add_func.blocks.push(add_block);
        add_func.vreg_count = 3;

        // Main function: call add(1, 2)
        let mut main_func = Function::new(FuncId(1), Some("main".to_string()));
        let mut main_block = Block::new(BlockId(0));
        main_block.push(Inst::Call {
            dst: Some(VReg(0)),
            func: FuncId(0),
            args: vec![Operand::IntConst(1), Operand::IntConst(2)],
        });
        main_block.push(Inst::Return { value: None });
        main_func.blocks.push(main_block);
        main_func.vreg_count = 1;

        let module = toy_mir::Module {
            functions: vec![add_func, main_func],
            main_id: FuncId(1),
        };

        let result = compile_native(&module);
        assert!(result.is_ok());
    }
}
