use std::collections::HashMap;

use cranelift::codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::{FuncId as CraneliftFuncId, Linkage, Module};
use mir::{BlockId, FuncId, Inst, LocalId, Operand, VReg};

use crate::JitError;

pub struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    module: &'a mut JITModule,
    int_type: types::Type,

    locals: HashMap<LocalId, Variable>,
    blocks: HashMap<BlockId, Block>,

    /// Track last computed value for implicit return
    last_value: Option<Value>,

    func_ids: &'a HashMap<FuncId, CraneliftFuncId>,
    env_ptr: Option<Value>,
    /// Runtime context pointer (for main function, enables list ops)
    context_ptr: Option<Value>,
    #[allow(dead_code)]
    is_closure: bool,
    is_main: bool,
}

impl<'a> FunctionTranslator<'a> {
    pub fn new(
        builder: FunctionBuilder<'a>,
        module: &'a mut JITModule,
        func_ids: &'a HashMap<FuncId, CraneliftFuncId>,
        func: &mir::Function,
        mir_module: &'a mir::Module,
    ) -> Self {
        let int_type = module.target_config().pointer_type();
        let is_main = func.id == mir_module.main_id;

        Self {
            builder,
            module,
            int_type,
            locals: HashMap::new(),
            blocks: HashMap::new(),
            last_value: None,
            func_ids,
            env_ptr: None,
            context_ptr: None,
            is_closure: func.is_closure,
            is_main,
        }
    }

    /// Translate entire function (for multi-function mode)
    pub fn translate(mut self, func: &mir::Function) -> Result<(), JitError> {
        // Create all blocks first
        for block in &func.blocks {
            let cl_block = self.builder.create_block();
            self.blocks.insert(block.id, cl_block);
        }

        // Entry block with params
        let entry = self.blocks[&BlockId(0)];
        self.builder.append_block_params_for_function_params(entry);
        self.builder.switch_to_block(entry);

        // Extract context pointer for main (enables runtime calls for list ops)
        let mut param_offset = 0;
        if self.is_main {
            self.context_ptr = Some(self.builder.block_params(entry)[0]);
            param_offset = 1;
        }

        // Extract env pointer for closures
        if func.is_closure {
            self.env_ptr = Some(self.builder.block_params(entry)[param_offset]);
            param_offset += 1;
        }

        // Bind params to locals
        let params = self.builder.block_params(entry).to_vec();
        for (i, &local_id) in func.params.iter().enumerate() {
            let var = self.builder.declare_var(self.int_type);
            self.builder.def_var(var, params[i + param_offset]);
            self.locals.insert(local_id, var);
        }

        // Declare other locals (not params)
        for i in func.params.len()..func.local_count as usize {
            let local_id = LocalId(i as u32);
            let var = self.builder.declare_var(self.int_type);
            let zero = self.builder.ins().iconst(self.int_type, 0);
            self.builder.def_var(var, zero);
            self.locals.insert(local_id, var);
        }

        // Pre-declare all vregs as Cranelift Variables
        // This is needed because MIR vregs can be written in multiple blocks
        // (like in if-then-else), but Cranelift SSA values can't
        let mut vreg_vars: HashMap<VReg, Variable> = HashMap::new();
        for block in &func.blocks {
            for inst in &block.insts {
                if let Some(dst) = self.get_inst_dst(inst)
                    && let std::collections::hash_map::Entry::Vacant(e) = vreg_vars.entry(dst)
                {
                    let var = self.builder.declare_var(self.int_type);
                    let zero = self.builder.ins().iconst(self.int_type, 0);
                    self.builder.def_var(var, zero);
                    e.insert(var);
                }
            }
        }

        // Translate blocks
        for (block_idx, mir_block) in func.blocks.iter().enumerate() {
            let cl_block = self.blocks[&mir_block.id];

            if block_idx != 0 {
                self.builder.switch_to_block(cl_block);
            }

            for inst in &mir_block.insts {
                self.translate_inst_with_vars(inst, &vreg_vars);
            }
        }

        // Seal all blocks and finalize
        self.builder.seal_all_blocks();
        self.builder.finalize();

        Ok(())
    }

    /// Get destination vreg from an instruction, if any
    fn get_inst_dst(&self, inst: &Inst) -> Option<VReg> {
        match inst {
            Inst::AddInt { dst, .. }
            | Inst::SubInt { dst, .. }
            | Inst::MulInt { dst, .. }
            | Inst::DivInt { dst, .. }
            | Inst::ModInt { dst, .. }
            | Inst::NegInt { dst, .. }
            | Inst::AddFloat { dst, .. }
            | Inst::SubFloat { dst, .. }
            | Inst::MulFloat { dst, .. }
            | Inst::DivFloat { dst, .. }
            | Inst::NegFloat { dst, .. }
            | Inst::EqInt { dst, .. }
            | Inst::NeInt { dst, .. }
            | Inst::LtInt { dst, .. }
            | Inst::LeInt { dst, .. }
            | Inst::GtInt { dst, .. }
            | Inst::GeInt { dst, .. }
            | Inst::LtFloat { dst, .. }
            | Inst::LeFloat { dst, .. }
            | Inst::GtFloat { dst, .. }
            | Inst::GeFloat { dst, .. }
            | Inst::Not { dst, .. }
            | Inst::Copy { dst, .. }
            | Inst::LoadLocal { dst, .. }
            | Inst::LoadCapture { dst, .. }
            | Inst::MakeClosure { dst, .. }
            | Inst::ListNew { dst, .. }
            | Inst::ListGet { dst, .. }
            | Inst::ListSlice { dst, .. }
            | Inst::TupleNew { dst, .. }
            | Inst::TupleGet { dst, .. } => Some(*dst),
            Inst::Call { dst, .. } | Inst::CallIndirect { dst, .. } => *dst,
            _ => None,
        }
    }

    /// Translate instruction using Variables for vregs (for multi-function mode)
    fn translate_inst_with_vars(&mut self, inst: &Inst, vreg_vars: &HashMap<VReg, Variable>) {
        match inst {
            Inst::AddInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().iadd(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::SubInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().isub(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::MulInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().imul(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::DivInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().sdiv(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::ModInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().srem(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::NegInt { dst, src } => {
                let v = self.operand_to_int_with_vars(src, vreg_vars);
                let result = self.builder.ins().ineg(v);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }

            Inst::AddFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().fadd(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::SubFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().fsub(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::MulFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().fmul(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::DivFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let result = self.builder.ins().fdiv(l, r);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::NegFloat { dst, src } => {
                let v = self.operand_to_float_with_vars(src, vreg_vars);
                let result = self.builder.ins().fneg(v);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }

            Inst::EqInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().icmp(IntCC::Equal, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::NeInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().icmp(IntCC::NotEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::LtInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().icmp(IntCC::SignedLessThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::LeInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::GtInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::GeInt { dst, lhs, rhs } => {
                let l = self.operand_to_int_with_vars(lhs, vreg_vars);
                let r = self.operand_to_int_with_vars(rhs, vreg_vars);
                let cmp = self
                    .builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }

            Inst::LtFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().fcmp(FloatCC::LessThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::LeFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().fcmp(FloatCC::LessThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::GtFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::GeFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float_with_vars(lhs, vreg_vars);
                let r = self.operand_to_float_with_vars(rhs, vreg_vars);
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }

            Inst::Not { dst, src } => {
                let v = self.operand_to_int_with_vars(src, vreg_vars);
                let one = self.builder.ins().iconst(self.int_type, 1);
                let result = self.builder.ins().bxor(v, one);
                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }

            Inst::Copy { dst, src } => {
                let v = self.operand_to_value_with_vars(src, vreg_vars);
                self.builder.def_var(vreg_vars[dst], v);
                self.last_value = Some(v);
            }

            Inst::StoreLocal { local, src } => {
                let v = self.operand_to_value_with_vars(src, vreg_vars);
                let var = self.locals[local];
                self.builder.def_var(var, v);
            }
            Inst::LoadLocal { dst, local } => {
                let var = self.locals[local];
                let v = self.builder.use_var(var);
                self.builder.def_var(vreg_vars[dst], v);
                self.last_value = Some(v);
            }

            Inst::Jump { target } => {
                let block = self.blocks[target];
                self.builder.ins().jump(block, &[]);
            }
            Inst::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                let c = self.operand_to_int_with_vars(cond, vreg_vars);
                let then_block = self.blocks[then_bb];
                let else_block = self.blocks[else_bb];
                self.builder.ins().brif(c, then_block, &[], else_block, &[]);
            }
            Inst::Return { value } => {
                if let Some(v) = value {
                    let val = self.operand_to_value_with_vars(v, vreg_vars);
                    self.builder.ins().return_(&[val]);
                } else if let Some(last) = self.last_value {
                    self.builder.ins().return_(&[last]);
                } else {
                    let default = self.builder.ins().iconst(self.int_type, 0);
                    self.builder.ins().return_(&[default]);
                }
            }

            // Function calls
            Inst::Call { dst, func, args } => {
                let cranelift_id = self.func_ids[func];
                let func_ref = self
                    .module
                    .declare_func_in_func(cranelift_id, self.builder.func);

                let arg_vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.operand_to_value_with_vars(a, vreg_vars))
                    .collect();

                let call = self.builder.ins().call(func_ref, &arg_vals);

                if let Some(d) = dst {
                    let result = self.builder.inst_results(call)[0];
                    self.builder.def_var(vreg_vars[d], result);
                    self.last_value = Some(result);
                }
            }
            Inst::CallIndirect { dst, callee, args } => {
                let closure_val = self.operand_to_value_with_vars(callee, vreg_vars);
                let ptr_type = self.int_type;

                let func_ptr = self
                    .builder
                    .ins()
                    .load(ptr_type, MemFlags::new(), closure_val, 0);
                let env_ptr = self
                    .builder
                    .ins()
                    .load(ptr_type, MemFlags::new(), closure_val, 8);

                let mut call_args = vec![env_ptr];
                for arg in args {
                    call_args.push(self.operand_to_value_with_vars(arg, vreg_vars));
                }

                let sig = self.build_closure_sig(args.len());
                let sig_ref = self.builder.import_signature(sig);
                let call = self
                    .builder
                    .ins()
                    .call_indirect(sig_ref, func_ptr, &call_args);

                if let Some(d) = dst {
                    let result = self.builder.inst_results(call)[0];
                    self.builder.def_var(vreg_vars[d], result);
                    self.last_value = Some(result);
                }
            }
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                let ptr_type = self.int_type;
                let closure_size = 16i64;
                let closure_ptr = self.call_malloc(closure_size);

                let cranelift_id = self.func_ids[func];
                let func_ref = self
                    .module
                    .declare_func_in_func(cranelift_id, self.builder.func);
                let func_ptr_val = self.builder.ins().func_addr(ptr_type, func_ref);

                let env_size = (captures.len() * 8) as i64;
                let env_ptr = if env_size > 0 {
                    let env = self.call_malloc(env_size);
                    for (i, cap) in captures.iter().enumerate() {
                        let val = self.operand_to_value_with_vars(cap, vreg_vars);
                        let offset = (i * 8) as i32;
                        self.builder.ins().store(MemFlags::new(), val, env, offset);
                    }
                    env
                } else {
                    self.builder.ins().iconst(ptr_type, 0)
                };

                self.builder
                    .ins()
                    .store(MemFlags::new(), func_ptr_val, closure_ptr, 0);
                self.builder
                    .ins()
                    .store(MemFlags::new(), env_ptr, closure_ptr, 8);

                self.builder.def_var(vreg_vars[dst], closure_ptr);
                self.last_value = Some(closure_ptr);
            }
            Inst::LoadCapture { dst, index } => {
                let env_ptr = self.env_ptr.expect("LoadCapture without env");
                let offset = (*index * 8) as i32;
                let val = self
                    .builder
                    .ins()
                    .load(self.int_type, MemFlags::new(), env_ptr, offset);
                self.builder.def_var(vreg_vars[dst], val);
                self.last_value = Some(val);
            }
            Inst::StoreCapture { index, src } => {
                let env_ptr = self.env_ptr.expect("StoreCapture without env");
                let val = self.operand_to_value_with_vars(src, vreg_vars);
                let offset = (*index * 8) as i32;
                self.builder
                    .ins()
                    .store(MemFlags::new(), val, env_ptr, offset);
            }
            Inst::Echo { .. } => {
                // Not supported in JIT yet
            }

            // List operations - call runtime helpers
            Inst::ListNew { dst, capacity } => {
                let ctx = self.context_ptr.expect("list ops need context (main only)");
                let cap = self.builder.ins().iconst(self.int_type, *capacity as i64);

                let func_ref = self.get_runtime_fn("rt_list_new");
                let call = self.builder.ins().call(func_ref, &[ctx, cap]);
                let result = self.builder.inst_results(call)[0];

                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::ListSet { list, index, value } => {
                let ctx = self.context_ptr.expect("list ops need context (main only)");
                let list_val = self.operand_to_value_with_vars(list, vreg_vars);
                let idx_val = self.operand_to_int_with_vars(index, vreg_vars);
                // Elements must be NaN-boxed for heap storage
                let elem_val = self.operand_to_nan_boxed_with_vars(value, vreg_vars);

                let func_ref = self.get_runtime_fn("rt_list_set");
                self.builder
                    .ins()
                    .call(func_ref, &[ctx, list_val, idx_val, elem_val]);
            }
            Inst::ListGet { dst, list, index } => {
                let ctx = self.context_ptr.expect("list ops need context (main only)");
                let list_val = self.operand_to_value_with_vars(list, vreg_vars);
                let idx_val = self.operand_to_int_with_vars(index, vreg_vars);

                let func_ref = self.get_runtime_fn("rt_list_get");
                let call = self.builder.ins().call(func_ref, &[ctx, list_val, idx_val]);
                let result = self.builder.inst_results(call)[0];

                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
            Inst::ListSlice {
                dst,
                list,
                start,
                end,
            } => {
                let ctx = self.context_ptr.expect("list ops need context (main only)");
                let list_val = self.operand_to_value_with_vars(list, vreg_vars);
                // i64::MIN sentinel for missing bounds
                let start_val = start
                    .as_ref()
                    .map(|s| self.operand_to_int_with_vars(s, vreg_vars))
                    .unwrap_or_else(|| self.builder.ins().iconst(self.int_type, i64::MIN));
                let end_val = end
                    .as_ref()
                    .map(|e| self.operand_to_int_with_vars(e, vreg_vars))
                    .unwrap_or_else(|| self.builder.ins().iconst(self.int_type, i64::MIN));

                let func_ref = self.get_runtime_fn("rt_list_slice");
                let call = self
                    .builder
                    .ins()
                    .call(func_ref, &[ctx, list_val, start_val, end_val]);
                let result = self.builder.inst_results(call)[0];

                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }

            // Tuple operations
            Inst::TupleNew { dst, elements } => {
                let ctx = self.context_ptr.expect("tuple ops need context (main only)");
                let count = elements.len();

                if count == 0 {
                    // Empty tuple - pass null pointer
                    let null_ptr = self.builder.ins().iconst(self.int_type, 0);
                    let count_val = self.builder.ins().iconst(types::I64, 0);
                    let func_ref = self.get_runtime_fn("rt_tuple_new");
                    let call = self.builder.ins().call(func_ref, &[ctx, null_ptr, count_val]);
                    let result = self.builder.inst_results(call)[0];
                    self.builder.def_var(vreg_vars[dst], result);
                    self.last_value = Some(result);
                } else {
                    // Allocate stack slot for elements
                    let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        (count * 8) as u32,
                        8, // alignment
                    ));

                    // Store each element (NaN-boxed) into the stack slot
                    for (i, elem) in elements.iter().enumerate() {
                        let elem_val = self.operand_to_nan_boxed_with_vars(elem, vreg_vars);
                        let offset = (i * 8) as i32;
                        self.builder.ins().stack_store(elem_val, slot, offset);
                    }

                    // Get pointer to stack slot
                    let elem_ptr = self.builder.ins().stack_addr(self.int_type, slot, 0);
                    let count_val = self.builder.ins().iconst(types::I64, count as i64);

                    let func_ref = self.get_runtime_fn("rt_tuple_new");
                    let call = self
                        .builder
                        .ins()
                        .call(func_ref, &[ctx, elem_ptr, count_val]);
                    let result = self.builder.inst_results(call)[0];

                    self.builder.def_var(vreg_vars[dst], result);
                    self.last_value = Some(result);
                }
            }

            Inst::TupleGet { dst, tuple, index } => {
                let ctx = self.context_ptr.expect("tuple ops need context (main only)");
                let tuple_val = self.operand_to_value_with_vars(tuple, vreg_vars);
                let idx_val = self.builder.ins().iconst(types::I64, *index as i64);

                let func_ref = self.get_runtime_fn("rt_tuple_get");
                let call = self.builder.ins().call(func_ref, &[ctx, tuple_val, idx_val]);
                let result = self.builder.inst_results(call)[0];

                self.builder.def_var(vreg_vars[dst], result);
                self.last_value = Some(result);
            }
        }
    }

    /// Get or declare a runtime helper function
    fn get_runtime_fn(&mut self, name: &str) -> cranelift::codegen::ir::FuncRef {
        // Build signature based on function name
        let mut sig = self.module.make_signature();
        let ptr_type = self.int_type;

        match name {
            "rt_list_new" => {
                // fn(ctx: *mut, capacity: i64) -> i64
                sig.params.push(AbiParam::new(ptr_type)); // ctx
                sig.params.push(AbiParam::new(types::I64)); // capacity
                sig.returns.push(AbiParam::new(types::I64)); // list value
            }
            "rt_list_set" => {
                // fn(ctx: *mut, list: i64, index: i64, value: i64)
                sig.params.push(AbiParam::new(ptr_type)); // ctx
                sig.params.push(AbiParam::new(types::I64)); // list
                sig.params.push(AbiParam::new(types::I64)); // index
                sig.params.push(AbiParam::new(types::I64)); // value
            }
            "rt_list_get" => {
                // fn(ctx: *mut, list: i64, index: i64) -> i64
                sig.params.push(AbiParam::new(ptr_type)); // ctx
                sig.params.push(AbiParam::new(types::I64)); // list
                sig.params.push(AbiParam::new(types::I64)); // index
                sig.returns.push(AbiParam::new(types::I64)); // element value
            }
            "rt_list_slice" => {
                // fn(ctx: *mut, list: i64, start: i64, end: i64) -> i64
                sig.params.push(AbiParam::new(ptr_type)); // ctx
                sig.params.push(AbiParam::new(types::I64)); // list
                sig.params.push(AbiParam::new(types::I64)); // start
                sig.params.push(AbiParam::new(types::I64)); // end
                sig.returns.push(AbiParam::new(types::I64)); // new list value
            }
            "rt_tuple_new" => {
                // fn(ctx: *mut, elements_ptr: *const i64, count: i64) -> i64
                sig.params.push(AbiParam::new(ptr_type)); // ctx
                sig.params.push(AbiParam::new(ptr_type)); // elements_ptr
                sig.params.push(AbiParam::new(types::I64)); // count
                sig.returns.push(AbiParam::new(types::I64)); // tuple value
            }
            "rt_tuple_get" => {
                // fn(ctx: *mut, tuple: i64, index: i64) -> i64
                sig.params.push(AbiParam::new(ptr_type)); // ctx
                sig.params.push(AbiParam::new(types::I64)); // tuple
                sig.params.push(AbiParam::new(types::I64)); // index
                sig.returns.push(AbiParam::new(types::I64)); // element value
            }
            _ => panic!("unknown runtime function: {}", name),
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .expect("failed to declare runtime function");

        self.module.declare_func_in_func(func_id, self.builder.func)
    }

    fn operand_to_int_with_vars(
        &mut self,
        op: &Operand,
        vreg_vars: &HashMap<VReg, Variable>,
    ) -> Value {
        match op {
            Operand::VReg(v) => self.builder.use_var(vreg_vars[v]),
            Operand::IntConst(n) => self.builder.ins().iconst(self.int_type, *n),
            Operand::BoolConst(b) => self.builder.ins().iconst(self.int_type, *b as i64),
            Operand::FloatConst(_) | Operand::StringConst(_) => {
                panic!("expected int operand, got {:?}", op)
            }
        }
    }

    /// Convert operand to NaN-boxed representation (for list element storage)
    fn operand_to_nan_boxed_with_vars(
        &mut self,
        op: &Operand,
        vreg_vars: &HashMap<VReg, Variable>,
    ) -> Value {
        // NaN-boxing constants:
        // QNAN = 0xFFF8_0000_0000_0000
        // TAG_INT = 0x0001_0000_0000_0000 -> QNAN_INT = 0xFFF9_0000_0000_0000
        // TAG_BOOL = 0x0002_0000_0000_0000 -> QNAN_BOOL = 0xFFFA_0000_0000_0000
        // Float: raw IEEE 754 bits
        const QNAN_INT: i64 = 0xFFF9_0000_0000_0000_u64 as i64;
        const QNAN_BOOL: i64 = 0xFFFA_0000_0000_0000_u64 as i64;
        const PAYLOAD_MASK: i64 = 0x0000_FFFF_FFFF_FFFF_u64 as i64;

        match op {
            Operand::VReg(v) => {
                // VRegs may already hold NaN-boxed values (from list operations)
                // or raw values (from arithmetic). For now, assume raw and box.
                // TODO: Track value types for proper handling
                self.builder.use_var(vreg_vars[v])
            }
            Operand::IntConst(n) => {
                let boxed = QNAN_INT | (*n & PAYLOAD_MASK);
                self.builder.ins().iconst(self.int_type, boxed)
            }
            Operand::BoolConst(b) => {
                let boxed = QNAN_BOOL | (*b as i64);
                self.builder.ins().iconst(self.int_type, boxed)
            }
            Operand::FloatConst(f) => {
                // Floats are stored as raw IEEE 754 bits (not NaN-boxed)
                let bits = f.to_bits() as i64;
                self.builder.ins().iconst(self.int_type, bits)
            }
            Operand::StringConst(_) => {
                panic!("string constants not yet supported in JIT lists")
            }
        }
    }

    fn operand_to_float_with_vars(
        &mut self,
        op: &Operand,
        vreg_vars: &HashMap<VReg, Variable>,
    ) -> Value {
        match op {
            Operand::VReg(v) => self.builder.use_var(vreg_vars[v]),
            Operand::FloatConst(n) => self.builder.ins().f64const(*n),
            _ => panic!("expected float operand, got {:?}", op),
        }
    }

    fn operand_to_value_with_vars(
        &mut self,
        op: &Operand,
        vreg_vars: &HashMap<VReg, Variable>,
    ) -> Value {
        match op {
            Operand::VReg(v) => self.builder.use_var(vreg_vars[v]),
            Operand::IntConst(n) => self.builder.ins().iconst(self.int_type, *n),
            Operand::FloatConst(n) => self.builder.ins().f64const(*n),
            Operand::BoolConst(b) => self.builder.ins().iconst(self.int_type, *b as i64),
            Operand::StringConst(_) => {
                panic!("string constants not yet supported in JIT")
            }
        }
    }

    /// Build signature for closure calls (env ptr + N args -> i64)
    fn build_closure_sig(&self, arg_count: usize) -> Signature {
        let mut sig = self.module.make_signature();
        // Env pointer
        sig.params.push(AbiParam::new(self.int_type));
        // Regular args
        for _ in 0..arg_count {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns.push(AbiParam::new(types::I64));
        sig
    }

    /// Call malloc to allocate memory
    fn call_malloc(&mut self, size: i64) -> Value {
        // Declare malloc if not already declared
        let mut malloc_sig = self.module.make_signature();
        malloc_sig.params.push(AbiParam::new(self.int_type));
        malloc_sig.returns.push(AbiParam::new(self.int_type));

        let malloc_id = self
            .module
            .declare_function("malloc", Linkage::Import, &malloc_sig)
            .expect("Failed to declare malloc");

        let malloc_ref = self
            .module
            .declare_func_in_func(malloc_id, self.builder.func);
        let size_val = self.builder.ins().iconst(self.int_type, size);
        let call = self.builder.ins().call(malloc_ref, &[size_val]);
        self.builder.inst_results(call)[0]
    }
}
