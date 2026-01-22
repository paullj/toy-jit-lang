use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::{Block, InstBuilder, Value};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{FuncId, Module};
use once_cell::sync::Lazy;

use toy_mir::{BlockId, Inst, LocalId, Operand, VReg, ValueType};

use crate::error::CodegenError;
use crate::runtime::RuntimeFuncs;

/// Translates a single MIR function to Cranelift IR
pub struct FunctionTranslator<'a, M: Module> {
    builder: FunctionBuilder<'a>,
    mir_func: &'a toy_mir::Function,
    variables: HashMap<LocalId, Variable>,
    vregs: HashMap<VReg, Variable>,
    blocks: HashMap<BlockId, Block>,
    module: &'a mut M,
    runtime: &'a RuntimeFuncs,
    func_ids: &'a HashMap<toy_mir::FuncId, FuncId>,
    external_funcs: &'a HashMap<toy_mir::FuncId, FuncId>,
}

// Empty external functions map for backward compatibility
#[allow(dead_code)]
static EMPTY_EXTERNALS: Lazy<HashMap<toy_mir::FuncId, FuncId>> = Lazy::new(HashMap::new);

impl<'a, M: Module> FunctionTranslator<'a, M> {
    #[allow(dead_code)]
    pub fn new(
        builder: FunctionBuilder<'a>,
        mir_func: &'a toy_mir::Function,
        module: &'a mut M,
        runtime: &'a RuntimeFuncs,
        func_ids: &'a HashMap<toy_mir::FuncId, FuncId>,
    ) -> Self {
        // For backward compatibility, use empty external funcs
        Self::new_with_external(
            builder,
            mir_func,
            module,
            runtime,
            func_ids,
            &EMPTY_EXTERNALS,
        )
    }

    pub fn new_with_external(
        builder: FunctionBuilder<'a>,
        mir_func: &'a toy_mir::Function,
        module: &'a mut M,
        runtime: &'a RuntimeFuncs,
        func_ids: &'a HashMap<toy_mir::FuncId, FuncId>,
        external_funcs: &'a HashMap<toy_mir::FuncId, FuncId>,
    ) -> Self {
        Self {
            builder,
            mir_func,
            variables: HashMap::new(),
            vregs: HashMap::new(),
            blocks: HashMap::new(),
            module,
            runtime,
            func_ids,
            external_funcs,
        }
    }

    pub fn translate(mut self) -> Result<(), CodegenError> {
        // Create all blocks first
        for mir_block in &self.mir_func.blocks {
            let block = self.builder.create_block();
            self.blocks.insert(mir_block.id, block);
        }

        // Setup entry block params and locals
        let entry_block = self.blocks[&BlockId(0)];
        self.builder
            .append_block_params_for_function_params(entry_block);

        // Declare variables for locals
        for i in 0..self.mir_func.local_count {
            let local = LocalId(i);
            let var = Variable::from_u32(i);
            self.builder.declare_var(var, I64); // All locals stored as I64 for now
            self.variables.insert(local, var);
        }

        // Declare variables for vregs (offset by local_count to avoid collision)
        for i in 0..self.mir_func.vreg_count {
            let vreg = VReg(i);
            let var = Variable::from_u32(self.mir_func.local_count + i);
            self.builder.declare_var(var, I64);
            self.vregs.insert(vreg, var);
        }

        // Translate all blocks (don't seal yet - need all edges first)
        for (i, mir_block) in self.mir_func.blocks.iter().enumerate() {
            let block = self.blocks[&mir_block.id];
            self.builder.switch_to_block(block);

            // Copy params to their local slots (only for entry block)
            if i == 0 {
                for (j, &local) in self.mir_func.params.iter().enumerate() {
                    let param_val = self.builder.block_params(entry_block)[j];
                    let var = self.variables[&local];
                    self.builder.def_var(var, param_val);
                }
            }

            for inst in &mir_block.insts {
                self.translate_inst(inst)?;
            }
        }

        // Seal all blocks after all edges are known
        self.builder.seal_all_blocks();
        self.builder.finalize();
        Ok(())
    }

    fn operand(&mut self, op: &Operand) -> Value {
        match op {
            Operand::VReg(v) => {
                let var = self.vregs[v];
                self.builder.use_var(var)
            }
            Operand::IntConst(n) => self.builder.ins().iconst(I64, *n),
            Operand::FloatConst(f) => self.builder.ins().f64const(*f),
            Operand::BoolConst(b) => self.builder.ins().iconst(I64, *b as i64),
            Operand::StringConst(_) => panic!("strings not supported in v1"),
        }
    }

    fn def_vreg(&mut self, vreg: &VReg, value: Value) {
        let var = self.vregs[vreg];
        self.builder.def_var(var, value);
    }

    fn translate_inst(&mut self, inst: &Inst) -> Result<(), CodegenError> {
        match inst {
            // Integer arithmetic
            Inst::AddInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().iadd(a, b);
                self.def_vreg(dst, result);
            }
            Inst::SubInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().isub(a, b);
                self.def_vreg(dst, result);
            }
            Inst::MulInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().imul(a, b);
                self.def_vreg(dst, result);
            }
            Inst::DivInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().sdiv(a, b);
                self.def_vreg(dst, result);
            }
            Inst::ModInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().srem(a, b);
                self.def_vreg(dst, result);
            }
            Inst::NegInt { dst, src } => {
                let v = self.operand(src);
                let result = self.builder.ins().ineg(v);
                self.def_vreg(dst, result);
            }

            // Float arithmetic
            Inst::AddFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().fadd(a, b);
                self.def_vreg(dst, result);
            }
            Inst::SubFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().fsub(a, b);
                self.def_vreg(dst, result);
            }
            Inst::MulFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().fmul(a, b);
                self.def_vreg(dst, result);
            }
            Inst::DivFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let result = self.builder.ins().fdiv(a, b);
                self.def_vreg(dst, result);
            }
            Inst::NegFloat { dst, src } => {
                let v = self.operand(src);
                let result = self.builder.ins().fneg(v);
                self.def_vreg(dst, result);
            }

            // Integer comparisons
            Inst::EqInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().icmp(IntCC::Equal, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::NeInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().icmp(IntCC::NotEqual, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::LtInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().icmp(IntCC::SignedLessThan, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::LeInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::GtInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThan, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::GeInt { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self
                    .builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }

            // Float comparisons
            Inst::LtFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::LessThan, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::LeFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::LessThanOrEqual, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::GtFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThan, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }
            Inst::GeFloat { dst, lhs, rhs } => {
                let a = self.operand(lhs);
                let b = self.operand(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThanOrEqual, a, b);
                let result = self.builder.ins().uextend(I64, cmp);
                self.def_vreg(dst, result);
            }

            // Boolean
            Inst::Not { dst, src } => {
                let v = self.operand(src);
                let one = self.builder.ins().iconst(I64, 1);
                let result = self.builder.ins().bxor(v, one);
                self.def_vreg(dst, result);
            }

            // Copy
            Inst::Copy { dst, src } => {
                let v = self.operand(src);
                self.def_vreg(dst, v);
            }

            // Locals
            Inst::StoreLocal { local, src } => {
                let val = self.operand(src);
                let var = self.variables[local];
                self.builder.def_var(var, val);
            }
            Inst::LoadLocal { dst, local } => {
                let var = self.variables[local];
                let val = self.builder.use_var(var);
                self.def_vreg(dst, val);
            }

            // Control flow
            Inst::Jump { target } => {
                let block = self.blocks[target];
                self.builder.ins().jump(block, &[]);
            }
            Inst::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                let c = self.operand(cond);
                let then_block = self.blocks[then_bb];
                let else_block = self.blocks[else_bb];
                self.builder.ins().brif(c, then_block, &[], else_block, &[]);
            }
            Inst::Return { value } => match value {
                Some(v) => {
                    let ret_val = self.operand(v);
                    self.builder.ins().return_(&[ret_val]);
                }
                None => {
                    self.builder.ins().return_(&[]);
                }
            },

            // Function calls
            Inst::Call { dst, func, args } => {
                // Check if function is local or external
                let callee = if let Some(&local_func) = self.func_ids.get(func) {
                    local_func
                } else if let Some(&external_func) = self.external_funcs.get(func) {
                    external_func
                } else {
                    return Err(CodegenError::Unsupported(
                        "function not found in local or external declarations",
                    ));
                };

                let func_ref = self.module.declare_func_in_func(callee, self.builder.func);
                let arg_vals: Vec<_> = args.iter().map(|a| self.operand(a)).collect();
                let call = self.builder.ins().call(func_ref, &arg_vals);
                if let Some(d) = dst {
                    let result = self.builder.inst_results(call)[0];
                    self.def_vreg(d, result);
                }
            }
            Inst::CallIndirect { .. } => {
                // CallIndirect indicates closures/function pointers which are not yet supported
                // This happens due to a bug in HIR where function calls inside function bodies
                // aren't properly resolved to direct calls
                return Err(CodegenError::Unsupported(
                    "indirect calls - closures and recursive functions not yet supported",
                ));
            }

            // Echo
            Inst::Echo { src, ty } => {
                let val = self.operand(src);
                let func = match ty {
                    ValueType::Int => self.runtime.echo_int,
                    ValueType::Float => self.runtime.echo_float,
                    ValueType::Bool => self.runtime.echo_bool,
                    ValueType::Boxed => return Err(CodegenError::Unsupported("boxed types")),
                };
                let func_ref = self.module.declare_func_in_func(func, self.builder.func);
                self.builder.ins().call(func_ref, &[val]);
            }

            // Closures
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                // Zero-capture closures are just function pointers - store func ID as I64
                if captures.is_empty() {
                    let func_id_val = self.builder.ins().iconst(I64, func.0 as i64);
                    self.def_vreg(dst, func_id_val);
                } else {
                    return Err(CodegenError::Unsupported("closures with captures"));
                }
            }
            Inst::LoadCapture { .. } => return Err(CodegenError::Unsupported("closures")),
            Inst::StoreCapture { .. } => return Err(CodegenError::Unsupported("closures")),

            // Lists - not supported in v1
            Inst::ListNew { .. } => return Err(CodegenError::Unsupported("lists")),
            Inst::ListSet { .. } => return Err(CodegenError::Unsupported("lists")),
            Inst::ListGet { .. } => return Err(CodegenError::Unsupported("lists")),
            Inst::ListSlice { .. } => return Err(CodegenError::Unsupported("lists")),
            Inst::ListLen { .. } => return Err(CodegenError::Unsupported("lists")),

            // Tuples - not supported in v1
            Inst::TupleNew { .. } => return Err(CodegenError::Unsupported("tuples")),
            Inst::TupleGet { .. } => return Err(CodegenError::Unsupported("tuples")),
        }
        Ok(())
    }
}
