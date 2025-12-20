mod error;
mod translate;

pub use error::JitError;

use cranelift::codegen;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

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
        }
    }

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
