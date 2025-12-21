use std::collections::HashMap;

use cranelift::codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::Module;
use mir::{Block as MirBlock, BlockId, Inst, LocalId, Operand, VReg};

use crate::ReturnType;

pub struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    #[allow(dead_code)]
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

    pub fn finalize(self) {
        self.builder.finalize();
    }
}
