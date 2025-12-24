use std::collections::HashMap;

use lasso::Rodeo;
use mir::{BlockId, FuncId, Inst, LocalId, Module, Operand, VReg};

use crate::bytecode::{FuncIdx, Instruction, Label, Slot};
use crate::chunk::{Chunk, CompiledModule};

pub fn compile(mir: &Module) -> CompiledModule {
    let mut strings = Rodeo::default();

    let chunks: Vec<Chunk> = mir
        .functions
        .iter()
        .map(|func| {
            let mut compiler = Compiler::new(&mut strings);
            compiler.compile_function(func);
            compiler.into_chunk()
        })
        .collect();

    CompiledModule {
        chunks,
        main_idx: mir.main_id.0 as usize,
        strings,
    }
}

struct Compiler<'a> {
    chunk: Chunk,
    interner: &'a mut Rodeo,
    vreg_to_slot: HashMap<VReg, Slot>,
    /// Next temp slot (starts at local_count)
    next_slot: u32,
    /// local_count for this function (slots [0, local_count) are locals)
    local_count: u32,
    block_labels: HashMap<BlockId, Label>,
    next_label: u32,
    /// Pending label patches: (instruction_index, label)
    label_patches: Vec<(usize, Label)>,
}

impl<'a> Compiler<'a> {
    fn new(interner: &'a mut Rodeo) -> Self {
        Self {
            chunk: Chunk::new(),
            interner,
            vreg_to_slot: HashMap::new(),
            next_slot: 0,
            local_count: 0,
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

    /// Allocate a temp slot (after locals)
    fn alloc_slot(&mut self) -> Slot {
        let s = Slot(self.next_slot);
        self.next_slot += 1;
        s
    }

    /// Allocate consecutive temp slots
    fn alloc_consecutive_slots(&mut self, count: usize) -> Slot {
        let base = Slot(self.next_slot);
        self.next_slot += count as u32;
        base
    }

    /// Map VReg to a physical slot
    fn vreg_to_physical(&mut self, vreg: VReg) -> Slot {
        if let Some(&slot) = self.vreg_to_slot.get(&vreg) {
            return slot;
        }
        let slot = self.alloc_slot();
        self.vreg_to_slot.insert(vreg, slot);
        slot
    }

    /// Get slot for a local variable (slots 0 to local_count-1)
    #[inline]
    fn local_slot(&self, local: LocalId) -> Slot {
        Slot(local.0)
    }

    fn emit(&mut self, inst: Instruction) {
        self.chunk.emit(inst);
    }

    fn compile_function(&mut self, func: &mir::Function) {
        self.chunk.param_count = func.param_count as u8;
        self.chunk.local_count = func.local_count;
        self.local_count = func.local_count;
        // Temps start after locals
        self.next_slot = func.local_count;

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

        // register_count = total slots used - local_count (just the temp slots)
        self.chunk.register_count = self.next_slot - self.local_count;
    }

    fn compile_inst(&mut self, inst: &Inst) {
        match inst {
            Inst::AddInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::AddInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::SubInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::SubInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::MulInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::MulInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::DivInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::DivInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::ModInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::ModInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::NegInt { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::NegInt {
                    dst: dst_slot,
                    src: src_slot,
                });
            }

            Inst::AddFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::AddFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::SubFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::SubFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::MulFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::MulFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::DivFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::DivFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::NegFloat { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::NegFloat {
                    dst: dst_slot,
                    src: src_slot,
                });
            }

            Inst::EqInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::EqInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::NeInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::NeInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::LtInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::LtInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::LeInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::LeInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::GtInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::GtInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::GeInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::GeInt {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }

            Inst::LtFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::LtFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::LeFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::LeFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::GtFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::GtFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }
            Inst::GeFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::GeFloat {
                    dst: dst_slot,
                    lhs: lhs_slot,
                    rhs: rhs_slot,
                });
            }

            Inst::Not { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::Not {
                    dst: dst_slot,
                    src: src_slot,
                });
            }

            Inst::Copy { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::Move {
                    dst: dst_slot,
                    src: src_slot,
                });
            }

            // StoreLocal: move from temp slot to local slot
            Inst::StoreLocal { local, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.local_slot(*local);
                // Only emit if src != dst
                if src_slot != dst_slot {
                    self.emit(Instruction::Move {
                        dst: dst_slot,
                        src: src_slot,
                    });
                }
            }
            // LoadLocal: move from local slot to temp slot
            Inst::LoadLocal { dst, local } => {
                let dst_slot = self.vreg_to_physical(*dst);
                let src_slot = self.local_slot(*local);
                // Only emit if src != dst
                if src_slot != dst_slot {
                    self.emit(Instruction::Move {
                        dst: dst_slot,
                        src: src_slot,
                    });
                }
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
                let cond_slot = self.load_operand(cond);
                let then_label = self.get_or_create_label(*then_bb);
                let else_label = self.get_or_create_label(*else_bb);

                // JumpIf cond -> then_bb
                let inst_idx = self.chunk.instructions.len();
                self.emit(Instruction::JumpIf {
                    cond: cond_slot,
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
                // Load args into consecutive slots
                let arg_base = self.alloc_consecutive_slots(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let s = Slot(arg_base.0 + i as u32);
                    self.load_operand_to(arg, s);
                }

                let dst_slot = dst.map(|v| self.vreg_to_physical(v));
                self.emit(Instruction::Call {
                    dst: dst_slot,
                    func_idx: func_id_to_idx(*func),
                    arg_base,
                    arg_count: args.len() as u8,
                });
            }
            Inst::CallIndirect { dst, callee, args } => {
                let callee_slot = self.load_operand(callee);

                // Load args into consecutive slots
                let arg_base = self.alloc_consecutive_slots(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let s = Slot(arg_base.0 + i as u32);
                    self.load_operand_to(arg, s);
                }

                let dst_slot = dst.map(|v| self.vreg_to_physical(v));
                self.emit(Instruction::CallIndirect {
                    dst: dst_slot,
                    callee: callee_slot,
                    arg_base,
                    arg_count: args.len() as u8,
                });
            }
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                // Load captures into consecutive slots
                let capture_base = self.alloc_consecutive_slots(captures.len());
                for (i, cap) in captures.iter().enumerate() {
                    let s = Slot(capture_base.0 + i as u32);
                    self.load_operand_to(cap, s);
                }

                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::MakeClosure {
                    dst: dst_slot,
                    func_idx: func_id_to_idx(*func),
                    capture_base,
                    capture_count: captures.len() as u8,
                });
            }
            Inst::LoadCapture { dst, index } => {
                let dst_slot = self.vreg_to_physical(*dst);
                self.emit(Instruction::LoadCapture {
                    dst: dst_slot,
                    index: *index as u8,
                });
            }
            Inst::StoreCapture { index, src } => {
                let src_slot = self.load_operand(src);
                self.emit(Instruction::StoreCapture {
                    index: *index as u8,
                    src: src_slot,
                });
            }
            Inst::Echo { src } => {
                let src_slot = self.load_operand(src);
                self.emit(Instruction::Echo { src: src_slot });
            }
        }
    }

    fn load_operand(&mut self, op: &Operand) -> Slot {
        match op {
            Operand::VReg(v) => self.vreg_to_physical(*v),
            Operand::IntConst(n) => {
                let slot = self.alloc_slot();
                self.emit(Instruction::LoadInt {
                    dst: slot,
                    value: *n,
                });
                slot
            }
            Operand::FloatConst(f) => {
                let idx = self.chunk.constants.add_float(*f);
                let slot = self.alloc_slot();
                self.emit(Instruction::LoadConst { dst: slot, idx });
                slot
            }
            Operand::BoolConst(b) => {
                let slot = self.alloc_slot();
                self.emit(Instruction::LoadBool {
                    dst: slot,
                    value: *b,
                });
                slot
            }
            Operand::StringConst(s) => {
                let idx = self.chunk.constants.add_string(s, self.interner);
                let slot = self.alloc_slot();
                self.emit(Instruction::LoadConst { dst: slot, idx });
                slot
            }
        }
    }

    fn load_operand_to(&mut self, op: &Operand, dst: Slot) {
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
                let idx = self.chunk.constants.add_string(s, self.interner);
                self.emit(Instruction::LoadConst { dst, idx });
            }
        }
    }

    fn load_binary_operands(&mut self, lhs: &Operand, rhs: &Operand) -> (Slot, Slot) {
        let lhs_slot = self.load_operand(lhs);
        let rhs_slot = self.load_operand(rhs);
        (lhs_slot, rhs_slot)
    }

    fn into_chunk(self) -> Chunk {
        self.chunk
    }
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
