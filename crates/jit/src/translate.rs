use std::collections::HashMap;

use cranelift::codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::{FuncId as CraneliftFuncId, Linkage, Module};
use mir::{Block as MirBlock, BlockId, FuncId, Inst, LocalId, Operand, VReg};

use crate::{JitError, ReturnType};

pub struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    module: &'a mut JITModule,
    int_type: types::Type,
    #[allow(dead_code)]
    float_type: types::Type,
    ret_type: ReturnType,

    vregs: HashMap<VReg, Value>,
    locals: HashMap<LocalId, Variable>,
    blocks: HashMap<BlockId, Block>,

    /// Track last computed value for implicit return
    last_value: Option<Value>,

    // Multi-function support
    func_ids: Option<&'a HashMap<FuncId, CraneliftFuncId>>,
    #[allow(dead_code)]
    mir_module: Option<&'a mir::Module>,
    env_ptr: Option<Value>,
    #[allow(dead_code)]
    is_closure: bool,
}

impl<'a> FunctionTranslator<'a> {
    pub fn new(
        builder: FunctionBuilder<'a>,
        module: &'a mut JITModule,
        _local_count: u32,
        ret_type: ReturnType,
    ) -> Self {
        let int_type = module.target_config().pointer_type();
        let float_type = types::F64;

        Self {
            builder,
            module,
            int_type,
            float_type,
            ret_type,
            vregs: HashMap::new(),
            locals: HashMap::new(),
            blocks: HashMap::new(),
            last_value: None,
            func_ids: None,
            mir_module: None,
            env_ptr: None,
            is_closure: false,
        }
    }

    /// Create translator with full module context for multi-function compilation
    pub fn new_with_module_context(
        builder: FunctionBuilder<'a>,
        module: &'a mut JITModule,
        func_ids: &'a HashMap<FuncId, CraneliftFuncId>,
        func: &mir::Function,
        mir_module: &'a mir::Module,
    ) -> Self {
        let int_type = module.target_config().pointer_type();
        let float_type = types::F64;

        Self {
            builder,
            module,
            int_type,
            float_type,
            ret_type: ReturnType::Integer,
            vregs: HashMap::new(),
            locals: HashMap::new(),
            blocks: HashMap::new(),
            last_value: None,
            func_ids: Some(func_ids),
            mir_module: Some(mir_module),
            env_ptr: None,
            is_closure: func.is_closure,
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

        // Extract env pointer for closures
        let mut param_offset = 0;
        if func.is_closure {
            self.env_ptr = Some(self.builder.block_params(entry)[0]);
            param_offset = 1;
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
            | Inst::MakeClosure { dst, .. } => Some(*dst),
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
                if let Some(func_ids) = self.func_ids {
                    let cranelift_id = func_ids[func];
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
                } else {
                    let result = self.builder.ins().iconst(self.int_type, 0);
                    if let Some(d) = dst {
                        self.builder.def_var(vreg_vars[d], result);
                    }
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

                if let Some(func_ids) = self.func_ids {
                    let cranelift_id = func_ids[func];
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
                }

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
        }
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

    pub fn create_blocks(&mut self, func: &mir::Function) {
        for block in &func.blocks {
            let cl_block = self.builder.create_block();
            self.blocks.insert(block.id, cl_block);
        }

        if let Some(entry) = func.blocks.first() {
            let entry_block = self.blocks[&entry.id];
            self.builder.switch_to_block(entry_block);
            self.builder.seal_block(entry_block);
        }

        for i in 0..func.local_count {
            let var = self.builder.declare_var(self.int_type);
            let zero = self.builder.ins().iconst(self.int_type, 0);
            self.builder.def_var(var, zero);
            self.locals.insert(LocalId(i), var);
        }
    }

    pub fn translate_block(&mut self, block: &MirBlock) {
        let cl_block = self.blocks[&block.id];

        if block.id.0 != 0 {
            self.builder.switch_to_block(cl_block);
        }

        for inst in &block.insts {
            self.translate_inst(inst);
        }

        if block.id.0 != 0 {
            self.builder.seal_block(cl_block);
        }
    }

    fn translate_inst(&mut self, inst: &Inst) {
        match inst {
            Inst::AddInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let result = self.builder.ins().iadd(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::SubInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let result = self.builder.ins().isub(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::MulInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let result = self.builder.ins().imul(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::DivInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let result = self.builder.ins().sdiv(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::ModInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let result = self.builder.ins().srem(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::NegInt { dst, src } => {
                let v = self.operand_to_int(src);
                let result = self.builder.ins().ineg(v);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }

            Inst::AddFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let result = self.builder.ins().fadd(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::SubFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let result = self.builder.ins().fsub(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::MulFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let result = self.builder.ins().fmul(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::DivFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let result = self.builder.ins().fdiv(l, r);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::NegFloat { dst, src } => {
                let v = self.operand_to_float(src);
                let result = self.builder.ins().fneg(v);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }

            Inst::EqInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let cmp = self.builder.ins().icmp(IntCC::Equal, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::NeInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let cmp = self.builder.ins().icmp(IntCC::NotEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::LtInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let cmp = self.builder.ins().icmp(IntCC::SignedLessThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::LeInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let cmp = self.builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::GtInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::GeInt { dst, lhs, rhs } => {
                let l = self.operand_to_int(lhs);
                let r = self.operand_to_int(rhs);
                let cmp = self
                    .builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }

            Inst::LtFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::LessThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::LeFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::LessThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::GtFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThan, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }
            Inst::GeFloat { dst, lhs, rhs } => {
                let l = self.operand_to_float(lhs);
                let r = self.operand_to_float(rhs);
                let cmp = self.builder.ins().fcmp(FloatCC::GreaterThanOrEqual, l, r);
                let result = self.builder.ins().uextend(self.int_type, cmp);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }

            Inst::Not { dst, src } => {
                let v = self.operand_to_int(src);
                let one = self.builder.ins().iconst(self.int_type, 1);
                let result = self.builder.ins().bxor(v, one);
                self.vregs.insert(*dst, result);
                self.last_value = Some(result);
            }

            Inst::Copy { dst, src } => {
                let v = self.operand_to_value(src);
                self.vregs.insert(*dst, v);
                self.last_value = Some(v);
            }

            Inst::StoreLocal { local, src } => {
                let v = self.operand_to_value(src);
                let var = self.locals[local];
                self.builder.def_var(var, v);
            }
            Inst::LoadLocal { dst, local } => {
                let var = self.locals[local];
                let v = self.builder.use_var(var);
                self.vregs.insert(*dst, v);
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
                let c = self.operand_to_int(cond);
                let then_block = self.blocks[then_bb];
                let else_block = self.blocks[else_bb];
                self.builder.ins().brif(c, then_block, &[], else_block, &[]);
            }
            Inst::Return { value } => {
                if let Some(v) = value {
                    let val = self.operand_to_value(v);
                    self.builder.ins().return_(&[val]);
                } else if let Some(last) = self.last_value {
                    // Return the last computed value
                    self.builder.ins().return_(&[last]);
                } else {
                    // Return default based on return type
                    let default = match self.ret_type {
                        ReturnType::Float => self.builder.ins().f64const(0.0),
                        ReturnType::Integer | ReturnType::Boolean => {
                            self.builder.ins().iconst(self.int_type, 0)
                        }
                    };
                    self.builder.ins().return_(&[default]);
                }
            }

            // Function calls
            Inst::Call { dst, func, args } => {
                if let Some(func_ids) = self.func_ids {
                    // Multi-function mode: proper function calls
                    let cranelift_id = func_ids[func];
                    let func_ref = self
                        .module
                        .declare_func_in_func(cranelift_id, self.builder.func);

                    let arg_vals: Vec<Value> =
                        args.iter().map(|a| self.operand_to_value(a)).collect();

                    let call = self.builder.ins().call(func_ref, &arg_vals);

                    if let Some(d) = dst {
                        let result = self.builder.inst_results(call)[0];
                        self.vregs.insert(*d, result);
                        self.last_value = Some(result);
                    }
                } else {
                    // Legacy single-function mode: stub
                    let result = self.builder.ins().iconst(self.int_type, 0);
                    if let Some(d) = dst {
                        self.vregs.insert(*d, result);
                    }
                    self.last_value = Some(result);
                }
            }
            Inst::CallIndirect { dst, callee, args } => {
                // Closure call: extract func ptr and env from closure struct
                let closure_val = self.operand_to_value(callee);
                let ptr_type = self.int_type;

                // Load func ptr (offset 0) and env ptr (offset 8)
                let func_ptr = self
                    .builder
                    .ins()
                    .load(ptr_type, MemFlags::new(), closure_val, 0);
                let env_ptr = self
                    .builder
                    .ins()
                    .load(ptr_type, MemFlags::new(), closure_val, 8);

                // Build args with env as first
                let mut call_args = vec![env_ptr];
                for arg in args {
                    call_args.push(self.operand_to_value(arg));
                }

                // Build closure signature (env ptr + args -> i64)
                let sig = self.build_closure_sig(args.len());
                let sig_ref = self.builder.import_signature(sig);
                let call = self
                    .builder
                    .ins()
                    .call_indirect(sig_ref, func_ptr, &call_args);

                if let Some(d) = dst {
                    let result = self.builder.inst_results(call)[0];
                    self.vregs.insert(*d, result);
                    self.last_value = Some(result);
                }
            }
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                let ptr_type = self.int_type;

                // Allocate closure struct: { func_ptr: *u8, env_ptr: *u8 }
                let closure_size = 16i64; // 2 pointers
                let closure_ptr = self.call_malloc(closure_size);

                // Get function pointer
                if let Some(func_ids) = self.func_ids {
                    let cranelift_id = func_ids[func];
                    let func_ref = self
                        .module
                        .declare_func_in_func(cranelift_id, self.builder.func);
                    let func_ptr_val = self.builder.ins().func_addr(ptr_type, func_ref);

                    // Allocate and populate env
                    let env_size = (captures.len() * 8) as i64;
                    let env_ptr = if env_size > 0 {
                        let env = self.call_malloc(env_size);
                        for (i, cap) in captures.iter().enumerate() {
                            let val = self.operand_to_value(cap);
                            let offset = (i * 8) as i32;
                            self.builder.ins().store(MemFlags::new(), val, env, offset);
                        }
                        env
                    } else {
                        self.builder.ins().iconst(ptr_type, 0)
                    };

                    // Store func_ptr and env_ptr in closure struct
                    self.builder
                        .ins()
                        .store(MemFlags::new(), func_ptr_val, closure_ptr, 0);
                    self.builder
                        .ins()
                        .store(MemFlags::new(), env_ptr, closure_ptr, 8);
                }

                self.vregs.insert(*dst, closure_ptr);
                self.last_value = Some(closure_ptr);
            }
            Inst::LoadCapture { dst, index } => {
                let env_ptr = self.env_ptr.expect("LoadCapture without env");
                let offset = (*index * 8) as i32;
                let val = self
                    .builder
                    .ins()
                    .load(self.int_type, MemFlags::new(), env_ptr, offset);
                self.vregs.insert(*dst, val);
                self.last_value = Some(val);
            }
            Inst::StoreCapture { index, src } => {
                let env_ptr = self.env_ptr.expect("StoreCapture without env");
                let val = self.operand_to_value(src);
                let offset = (*index * 8) as i32;
                self.builder
                    .ins()
                    .store(MemFlags::new(), val, env_ptr, offset);
            }
            Inst::Echo { src } => {
                // TODO: Implement echo in JIT (would need runtime call)
                let _ = src;
            }
        }
    }

    fn operand_to_int(&mut self, op: &Operand) -> Value {
        match op {
            Operand::VReg(v) => self.vregs[v],
            Operand::IntConst(n) => self.builder.ins().iconst(self.int_type, *n),
            Operand::BoolConst(b) => self.builder.ins().iconst(self.int_type, *b as i64),
            Operand::FloatConst(_) | Operand::StringConst(_) => {
                panic!("expected int operand, got {:?}", op)
            }
        }
    }

    fn operand_to_float(&mut self, op: &Operand) -> Value {
        match op {
            Operand::VReg(v) => self.vregs[v],
            Operand::FloatConst(n) => self.builder.ins().f64const(*n),
            _ => panic!("expected float operand, got {:?}", op),
        }
    }

    fn operand_to_value(&mut self, op: &Operand) -> Value {
        match op {
            Operand::VReg(v) => self.vregs[v],
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

    pub fn finalize(self) {
        self.builder.finalize();
    }
}
