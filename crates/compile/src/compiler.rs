use std::collections::HashMap;

use mir::{Inst, LocalId, Module, Operand, VReg};

use crate::bytecode::{Instruction, LocalSlot, Reg};
use crate::chunk::{Chunk, CompiledModule};

pub fn compile(mir: &Module) -> CompiledModule {
    let mut compiler = Compiler::new();
    compiler.compile_function(&mir.main);
    compiler.finish()
}

struct Compiler {
    chunk: Chunk,
    vreg_to_reg: HashMap<VReg, Reg>,
    next_reg: u32,
}

impl Compiler {
    fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            vreg_to_reg: HashMap::new(),
            next_reg: 0,
        }
    }

    fn alloc_reg(&mut self) -> Reg {
        let r = Reg(self.next_reg);
        self.next_reg += 1;
        r
    }

    fn vreg_to_physical(&mut self, vreg: VReg) -> Reg {
        if let Some(&reg) = self.vreg_to_reg.get(&vreg) {
            return reg;
        }
        let reg = self.alloc_reg();
        self.vreg_to_reg.insert(vreg, reg);
        reg
    }

    fn emit(&mut self, inst: Instruction) {
        self.chunk.emit(inst);
    }

    fn compile_function(&mut self, func: &mir::Function) {
        self.chunk.local_count = func.local_count;

        for block in &func.blocks {
            for inst in &block.insts {
                self.compile_inst(inst);
            }
        }

        self.chunk.register_count = self.next_reg;
    }

    fn compile_inst(&mut self, inst: &Inst) {
        match inst {
            Inst::AddInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::AddInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::SubInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::SubInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::MulInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::MulInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::DivInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::DivInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::ModInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::ModInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::NegInt { dst, src } => {
                let src_reg = self.load_operand(src);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::NegInt {
                    dst: dst_reg,
                    src: src_reg,
                });
            }

            Inst::AddFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::AddFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::SubFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::SubFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::MulFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::MulFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::DivFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::DivFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::NegFloat { dst, src } => {
                let src_reg = self.load_operand(src);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::NegFloat {
                    dst: dst_reg,
                    src: src_reg,
                });
            }

            Inst::EqInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::EqInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::NeInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::NeInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::LtInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::LtInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::LeInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::LeInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::GtInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::GtInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::GeInt { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::GeInt {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }

            Inst::LtFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::LtFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::LeFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::LeFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::GtFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::GtFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }
            Inst::GeFloat { dst, lhs, rhs } => {
                let (lhs_reg, rhs_reg) = self.load_binary_operands(lhs, rhs);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::GeFloat {
                    dst: dst_reg,
                    lhs: lhs_reg,
                    rhs: rhs_reg,
                });
            }

            Inst::Not { dst, src } => {
                let src_reg = self.load_operand(src);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::Not {
                    dst: dst_reg,
                    src: src_reg,
                });
            }

            Inst::Copy { dst, src } => {
                let src_reg = self.load_operand(src);
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::Move {
                    dst: dst_reg,
                    src: src_reg,
                });
            }

            Inst::StoreLocal { local, src } => {
                let src_reg = self.load_operand(src);
                self.emit(Instruction::StoreLocal {
                    slot: local_to_slot(*local),
                    src: src_reg,
                });
            }
            Inst::LoadLocal { dst, local } => {
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::LoadLocal {
                    dst: dst_reg,
                    slot: local_to_slot(*local),
                });
            }

            Inst::Jump { .. } | Inst::Branch { .. } => {
                // Control flow not yet implemented
                // Would need label resolution
            }

            Inst::Return { .. } => {
                self.emit(Instruction::Halt);
            }
        }
    }

    fn load_operand(&mut self, op: &Operand) -> Reg {
        match op {
            Operand::VReg(v) => self.vreg_to_physical(*v),
            Operand::IntConst(n) => {
                let reg = self.alloc_reg();
                self.emit(Instruction::LoadInt {
                    dst: reg,
                    value: *n,
                });
                reg
            }
            Operand::FloatConst(f) => {
                let idx = self.chunk.constants.add_float(*f);
                let reg = self.alloc_reg();
                self.emit(Instruction::LoadConst { dst: reg, idx });
                reg
            }
            Operand::BoolConst(b) => {
                let reg = self.alloc_reg();
                self.emit(Instruction::LoadBool {
                    dst: reg,
                    value: *b,
                });
                reg
            }
            Operand::StringConst(s) => {
                let idx = self.chunk.constants.add_string(s.clone());
                let reg = self.alloc_reg();
                self.emit(Instruction::LoadConst { dst: reg, idx });
                reg
            }
        }
    }

    fn load_binary_operands(&mut self, lhs: &Operand, rhs: &Operand) -> (Reg, Reg) {
        let lhs_reg = self.load_operand(lhs);
        let rhs_reg = self.load_operand(rhs);
        (lhs_reg, rhs_reg)
    }

    fn finish(self) -> CompiledModule {
        CompiledModule { main: self.chunk }
    }
}

fn local_to_slot(local: LocalId) -> LocalSlot {
    LocalSlot(local.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir::{Block, BlockId, Function, Inst, Operand};

    #[test]
    fn test_compile_simple() {
        let mut func = Function::new(None);
        let mut block = Block::new(BlockId(0));
        block.push(Inst::AddInt {
            dst: VReg(0),
            lhs: Operand::IntConst(1),
            rhs: Operand::IntConst(2),
        });
        block.push(Inst::Return { value: None });
        func.blocks.push(block);
        func.vreg_count = 1;

        let module = Module { main: func };
        let compiled = compile(&module);

        assert!(!compiled.main.instructions.is_empty());
    }
}
