use std::collections::HashMap;

use mir::{BlockId, FuncId, Inst, LocalId, Module, Operand, VReg};

use crate::bytecode::{FuncIdx, Instruction, Label, LocalSlot, Reg};
use crate::chunk::{Chunk, CompiledModule};

pub fn compile(mir: &Module) -> CompiledModule {
    let chunks: Vec<Chunk> = mir
        .functions
        .iter()
        .map(|func| {
            let mut compiler = Compiler::new();
            compiler.compile_function(func);
            compiler.into_chunk()
        })
        .collect();

    CompiledModule {
        chunks,
        main_idx: mir.main_id.0 as usize,
    }
}

struct Compiler {
    chunk: Chunk,
    vreg_to_reg: HashMap<VReg, Reg>,
    next_reg: u32,
    block_labels: HashMap<BlockId, Label>,
    next_label: u32,
    /// Pending label patches: (instruction_index, label)
    label_patches: Vec<(usize, Label)>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            vreg_to_reg: HashMap::new(),
            next_reg: 0,
            block_labels: HashMap::new(),
            next_label: 0,
            label_patches: Vec::new(),
        }
    }

    fn get_or_create_label(&mut self, block_id: BlockId) -> Label {
        if let Some(&label) = self.block_labels.get(&block_id) {
            return label;
        }
        let label = Label(self.next_label);
        self.next_label += 1;
        self.block_labels.insert(block_id, label);
        label
    }

    fn alloc_reg(&mut self) -> Reg {
        let r = Reg(self.next_reg);
        self.next_reg += 1;
        r
    }

    fn alloc_consecutive_regs(&mut self, count: usize) -> Reg {
        let base = Reg(self.next_reg);
        self.next_reg += count as u32;
        base
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
        self.chunk.param_count = func.param_count as u8;
        self.chunk.local_count = func.local_count;

        // First pass: assign labels to blocks and emit code
        let mut label_positions: HashMap<Label, usize> = HashMap::new();

        for block in &func.blocks {
            // Record the position for this block's label
            let label = self.get_or_create_label(block.id);
            label_positions.insert(label, self.chunk.instructions.len());

            for inst in &block.insts {
                self.compile_inst(inst);
            }
        }

        // Second pass: patch jump targets with absolute positions
        for (inst_idx, label) in &self.label_patches {
            if let Some(&target_pos) = label_positions.get(label) {
                let patched_label = Label(target_pos as u32);
                match &mut self.chunk.instructions[*inst_idx] {
                    Instruction::Jump { target } => *target = patched_label,
                    Instruction::JumpIf { target, .. } => *target = patched_label,
                    Instruction::JumpIfNot { target, .. } => *target = patched_label,
                    _ => {}
                }
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

            Inst::Jump { target } => {
                let label = self.get_or_create_label(*target);
                let inst_idx = self.chunk.instructions.len();
                self.emit(Instruction::Jump {
                    target: Label(0), // Placeholder
                });
                self.label_patches.push((inst_idx, label));
            }
            Inst::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                let cond_reg = self.load_operand(cond);
                let then_label = self.get_or_create_label(*then_bb);
                let else_label = self.get_or_create_label(*else_bb);

                // JumpIf cond -> then_bb
                let inst_idx = self.chunk.instructions.len();
                self.emit(Instruction::JumpIf {
                    cond: cond_reg,
                    target: Label(0), // Placeholder
                });
                self.label_patches.push((inst_idx, then_label));

                // Jump -> else_bb
                let inst_idx = self.chunk.instructions.len();
                self.emit(Instruction::Jump {
                    target: Label(0), // Placeholder
                });
                self.label_patches.push((inst_idx, else_label));
            }

            Inst::Return { value } => {
                let src = value.as_ref().map(|v| self.load_operand(v));
                self.emit(Instruction::Return { src });
            }

            // Function calls
            Inst::Call { dst, func, args } => {
                // Load args into consecutive registers
                let arg_base = self.alloc_consecutive_regs(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let r = Reg(arg_base.0 + i as u32);
                    self.load_operand_to(arg, r);
                }

                let dst_reg = dst.map(|v| self.vreg_to_physical(v));
                self.emit(Instruction::Call {
                    dst: dst_reg,
                    func_idx: func_id_to_idx(*func),
                    arg_base,
                    arg_count: args.len() as u8,
                });
            }
            Inst::CallIndirect { dst, callee, args } => {
                let callee_reg = self.load_operand(callee);

                // Load args into consecutive registers
                let arg_base = self.alloc_consecutive_regs(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let r = Reg(arg_base.0 + i as u32);
                    self.load_operand_to(arg, r);
                }

                let dst_reg = dst.map(|v| self.vreg_to_physical(v));
                self.emit(Instruction::CallIndirect {
                    dst: dst_reg,
                    callee: callee_reg,
                    arg_base,
                    arg_count: args.len() as u8,
                });
            }
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                // Load captures into consecutive registers
                let capture_base = self.alloc_consecutive_regs(captures.len());
                for (i, cap) in captures.iter().enumerate() {
                    let r = Reg(capture_base.0 + i as u32);
                    self.load_operand_to(cap, r);
                }

                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::MakeClosure {
                    dst: dst_reg,
                    func_idx: func_id_to_idx(*func),
                    capture_base,
                    capture_count: captures.len() as u8,
                });
            }
            Inst::LoadCapture { dst, index } => {
                let dst_reg = self.vreg_to_physical(*dst);
                self.emit(Instruction::LoadCapture {
                    dst: dst_reg,
                    index: *index as u8,
                });
            }
            Inst::StoreCapture { index, src } => {
                let src_reg = self.load_operand(src);
                self.emit(Instruction::StoreCapture {
                    index: *index as u8,
                    src: src_reg,
                });
            }
            Inst::Echo { src } => {
                let src_reg = self.load_operand(src);
                self.emit(Instruction::Echo { src: src_reg });
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

    fn load_operand_to(&mut self, op: &Operand, dst: Reg) {
        match op {
            Operand::VReg(v) => {
                let src = self.vreg_to_physical(*v);
                if src != dst {
                    self.emit(Instruction::Move { dst, src });
                }
            }
            Operand::IntConst(n) => {
                self.emit(Instruction::LoadInt { dst, value: *n });
            }
            Operand::FloatConst(f) => {
                let idx = self.chunk.constants.add_float(*f);
                self.emit(Instruction::LoadConst { dst, idx });
            }
            Operand::BoolConst(b) => {
                self.emit(Instruction::LoadBool { dst, value: *b });
            }
            Operand::StringConst(s) => {
                let idx = self.chunk.constants.add_string(s.clone());
                self.emit(Instruction::LoadConst { dst, idx });
            }
        }
    }

    fn load_binary_operands(&mut self, lhs: &Operand, rhs: &Operand) -> (Reg, Reg) {
        let lhs_reg = self.load_operand(lhs);
        let rhs_reg = self.load_operand(rhs);
        (lhs_reg, rhs_reg)
    }

    fn into_chunk(self) -> Chunk {
        self.chunk
    }
}

fn local_to_slot(local: LocalId) -> LocalSlot {
    LocalSlot(local.0)
}

fn func_id_to_idx(func: FuncId) -> FuncIdx {
    FuncIdx(func.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir::{Block, BlockId, FuncId, Function, Inst, Operand};

    #[test]
    fn test_compile_simple() {
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

        let module = Module {
            functions: vec![func],
            main_id: FuncId(0),
        };
        let compiled = compile(&module);

        assert!(!compiled.main().instructions.is_empty());
    }
}
