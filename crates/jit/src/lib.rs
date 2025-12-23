mod error;
mod translate;

pub use error::JitError;

use std::collections::HashMap;

use cranelift::codegen;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId as CraneliftFuncId, Linkage, Module};

use translate::FunctionTranslator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReturnType {
    #[default]
    Integer,
    Float,
    Boolean,
}

pub struct Jit {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: JITModule,
    /// Map MIR func IDs to Cranelift func IDs
    func_ids: HashMap<mir::FuncId, CraneliftFuncId>,
    /// Map MIR func IDs to compiled function pointers
    func_ptrs: HashMap<mir::FuncId, *const u8>,
}

impl Default for Jit {
    fn default() -> Self {
        Self::new()
    }
}

impl Jit {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);

        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
            func_ids: HashMap::new(),
            func_ptrs: HashMap::new(),
        }
    }

    fn ptr_type(&self) -> types::Type {
        self.module.target_config().pointer_type()
    }

    /// Build signature for a MIR function
    fn build_signature(&self, func: &mir::Function) -> Signature {
        let mut sig = self.module.make_signature();

        // Closure env pointer as hidden first param
        if func.is_closure {
            sig.params.push(AbiParam::new(self.ptr_type()));
        }

        // Regular params (all i64 for now)
        for _ in 0..func.param_count {
            sig.params.push(AbiParam::new(types::I64));
        }

        // Return value (always i64 for now)
        sig.returns.push(AbiParam::new(types::I64));

        sig
    }

    /// Compile entire module (all functions)
    pub fn compile_module(&mut self, mir: &mir::Module) -> Result<(), JitError> {
        // Pass 1: Declare all functions
        for func in &mir.functions {
            let sig = self.build_signature(func);
            let name = func
                .name
                .as_deref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("_fn{}", func.id.0));

            let linkage = if func.id == mir.main_id {
                Linkage::Export
            } else {
                Linkage::Local
            };

            let id = self.module.declare_function(&name, linkage, &sig)?;
            self.func_ids.insert(func.id, id);
        }

        // Pass 2: Define all functions
        for func in &mir.functions {
            self.compile_function(func, mir)?;
        }

        // Finalize
        self.module.finalize_definitions()?;

        // Get function pointers
        for func in &mir.functions {
            let cranelift_id = self.func_ids[&func.id];
            let ptr = self.module.get_finalized_function(cranelift_id);
            self.func_ptrs.insert(func.id, ptr);
        }

        Ok(())
    }

    fn compile_function(
        &mut self,
        func: &mir::Function,
        mir_module: &mir::Module,
    ) -> Result<(), JitError> {
        self.ctx.func.signature = self.build_signature(func);

        let cranelift_id = self.func_ids[&func.id];

        {
            let builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

            let translator = FunctionTranslator::new_with_module_context(
                builder,
                &mut self.module,
                &self.func_ids,
                func,
                mir_module,
            );
            translator.translate(func)?;
        }

        self.module.define_function(cranelift_id, &mut self.ctx)?;
        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    /// Get main function pointer for execution
    pub fn get_main(&self, main_id: mir::FuncId) -> Option<*const u8> {
        self.func_ptrs.get(&main_id).copied()
    }

    /// Legacy single-function compile for backwards compatibility
    pub fn compile(
        &mut self,
        func: &mir::Function,
        ret_type: ReturnType,
    ) -> Result<*const u8, JitError> {
        let return_ty = match ret_type {
            ReturnType::Float => types::F64,
            ReturnType::Integer | ReturnType::Boolean => self.module.target_config().pointer_type(),
        };
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(return_ty));

        let builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        let mut translator =
            FunctionTranslator::new(builder, &mut self.module, func.local_count, ret_type);
        translator.create_blocks(func);

        for block in &func.blocks {
            translator.translate_block(block);
        }

        translator.finalize();

        let name = func.name.as_deref().unwrap_or("_main");
        let id = self
            .module
            .declare_function(name, Linkage::Export, &self.ctx.func.signature)?;

        self.module.define_function(id, &mut self.ctx)?;
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions()?;

        Ok(self.module.get_finalized_function(id))
    }
}
