mod error;
mod runtime;
mod translate;

pub use error::JitError;
pub use runtime::RuntimeContext;

use std::collections::HashMap;

use cranelift::codegen;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId as CraneliftFuncId, Linkage, Module};

use translate::FunctionTranslator;

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
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("enable_alias_analysis", "true").unwrap();

        #[cfg(not(debug_assertions))]
        flag_builder.set("enable_verifier", "false").unwrap();

        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register runtime helpers for list operations
        builder.symbol("rt_list_new", runtime::rt_list_new as *const u8);
        builder.symbol("rt_list_set", runtime::rt_list_set as *const u8);
        builder.symbol("rt_list_get", runtime::rt_list_get as *const u8);
        builder.symbol("rt_list_slice", runtime::rt_list_slice as *const u8);

        // Register runtime helpers for tuple operations
        builder.symbol("rt_tuple_new", runtime::rt_tuple_new as *const u8);
        builder.symbol("rt_tuple_get", runtime::rt_tuple_get as *const u8);

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
    fn build_signature(&self, func: &mir::Function, is_main: bool) -> Signature {
        let mut sig = self.module.make_signature();

        // Main function gets context pointer as first param
        if is_main {
            sig.params.push(AbiParam::new(self.ptr_type()));
        }

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
            let is_main = func.id == mir.main_id;
            let sig = self.build_signature(func, is_main);
            let name = func
                .name
                .as_deref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("_fn{}", func.id.0));

            let linkage = if is_main {
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
        let is_main = func.id == mir_module.main_id;
        self.ctx.func.signature = self.build_signature(func, is_main);

        let cranelift_id = self.func_ids[&func.id];

        {
            let builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

            let translator = FunctionTranslator::new(
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
}
