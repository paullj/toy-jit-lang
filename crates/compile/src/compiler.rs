use std::collections::HashMap;

use lasso::Rodeo;
use mir::{BlockId, Inst, LocalId, Module, Operand, VReg};

use crate::bytecode_writer::BytecodeWriter;
use crate::chunk::{Chunk, CompiledModule};
use crate::opcode::Opcode;

/// Kind of jump instruction for patching
#[derive(Clone, Copy)]
enum JumpKind {
    Jump,
    JumpIf,
    #[allow(dead_code)]
    JumpIfNot,
}

impl JumpKind {
    fn forward_opcode(self) -> Opcode {
        match self {
            JumpKind::Jump => Opcode::JumpFwd,
            JumpKind::JumpIf => Opcode::JumpIfFwd,
            JumpKind::JumpIfNot => Opcode::JumpIfNotFwd,
        }
    }

    fn backward_opcode(self) -> Opcode {
        match self {
            JumpKind::Jump => Opcode::JumpBack,
            JumpKind::JumpIf => Opcode::JumpIfBack,
            JumpKind::JumpIfNot => Opcode::JumpIfNotBack,
        }
    }

    /// Offset from opcode byte to the u16 offset field
    fn offset_delta(self) -> usize {
        match self {
            JumpKind::Jump => 1,                         // opcode + offset
            JumpKind::JumpIf | JumpKind::JumpIfNot => 2, // opcode + cond + offset
        }
    }

    /// Total instruction size
    fn inst_size(self) -> usize {
        match self {
            JumpKind::Jump => 3,                         // opcode + u16
            JumpKind::JumpIf | JumpKind::JumpIfNot => 4, // opcode + cond + u16
        }
    }
}

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
    writer: BytecodeWriter,
    interner: &'a mut Rodeo,
    vreg_to_slot: HashMap<VReg, u8>,
    /// Next temp slot (starts at local_count)
    next_slot: u8,
    /// local_count for this function (slots [0, local_count) are locals)
    local_count: u8,
    block_labels: HashMap<BlockId, u32>,
    next_label: u32,
    /// Pending jump patches: (opcode offset, label, jump kind)
    label_patches: Vec<(usize, u32, JumpKind)>,
    /// Label -> bytecode offset mapping
    label_positions: HashMap<u32, usize>,
}

impl<'a> Compiler<'a> {
    fn new(interner: &'a mut Rodeo) -> Self {
        Self {
            chunk: Chunk::new(),
            writer: BytecodeWriter::new(),
            interner,
            vreg_to_slot: HashMap::new(),
            next_slot: 0,
            local_count: 0,
            block_labels: HashMap::new(),
            next_label: 0,
            label_patches: Vec::new(),
            label_positions: HashMap::new(),
        }
    }

    fn get_or_create_label(&mut self, block_id: BlockId) -> u32 {
        if let Some(&label) = self.block_labels.get(&block_id) {
            return label;
        }
        let label = self.next_label;
        self.next_label += 1;
        self.block_labels.insert(block_id, label);
        label
    }

    /// Allocate a temp slot (after locals)
    fn alloc_slot(&mut self) -> u8 {
        let s = self.next_slot;
        self.next_slot += 1;
        s
    }

    /// Allocate consecutive temp slots
    fn alloc_consecutive_slots(&mut self, count: usize) -> u8 {
        let base = self.next_slot;
        self.next_slot += count as u8;
        base
    }

    /// Map VReg to a physical slot
    fn vreg_to_physical(&mut self, vreg: VReg) -> u8 {
        if let Some(&slot) = self.vreg_to_slot.get(&vreg) {
            return slot;
        }
        let slot = self.alloc_slot();
        self.vreg_to_slot.insert(vreg, slot);
        slot
    }

    /// Get slot for a local variable (slots 0 to local_count-1)
    #[inline]
    fn local_slot(&self, local: LocalId) -> u8 {
        local.0 as u8
    }

    fn compile_function(&mut self, func: &mir::Function) {
        self.chunk.param_count = func.param_count as u8;
        self.chunk.local_count = func.local_count as u8;
        self.local_count = func.local_count as u8;
        // Temps start after locals
        self.next_slot = func.local_count as u8;

        // First pass: assign labels to blocks and emit code
        for block in &func.blocks {
            // Record the position for this block's label
            let label = self.get_or_create_label(block.id);
            self.label_positions.insert(label, self.writer.len());

            for inst in &block.insts {
                self.compile_inst(inst);
            }
        }

        // Second pass: patch jump targets with forward/backward opcodes and u16 offsets
        for (opcode_offset, label, kind) in &self.label_patches {
            if let Some(&target_pos) = self.label_positions.get(label) {
                // Calculate relative offset from instruction end
                let inst_end = opcode_offset + kind.inst_size();
                let from = inst_end as isize;
                let to = target_pos as isize;
                let relative = to - from;

                let (opcode, offset) = if relative >= 0 {
                    (kind.forward_opcode(), relative as u16)
                } else {
                    (kind.backward_opcode(), (-relative) as u16)
                };

                self.writer.patch_u8(*opcode_offset, opcode as u8);
                self.writer
                    .patch_u16(*opcode_offset + kind.offset_delta(), offset);
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
                self.writer.emit_add_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::SubInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_sub_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::MulInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_mul_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::DivInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_div_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::ModInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_mod_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::NegInt { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_neg_int(dst_slot, src_slot);
            }

            Inst::AddFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_add_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::SubFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_sub_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::MulFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_mul_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::DivFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_div_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::NegFloat { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_neg_float(dst_slot, src_slot);
            }

            Inst::EqInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_eq_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::NeInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_ne_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::LtInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_lt_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::LeInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_le_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::GtInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_gt_int(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::GeInt { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_ge_int(dst_slot, lhs_slot, rhs_slot);
            }

            Inst::LtFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_lt_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::LeFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_le_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::GtFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_gt_float(dst_slot, lhs_slot, rhs_slot);
            }
            Inst::GeFloat { dst, lhs, rhs } => {
                let (lhs_slot, rhs_slot) = self.load_binary_operands(lhs, rhs);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_ge_float(dst_slot, lhs_slot, rhs_slot);
            }

            Inst::Not { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_not(dst_slot, src_slot);
            }

            Inst::Copy { dst, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_move(dst_slot, src_slot);
            }

            // StoreLocal: move from temp slot to local slot
            Inst::StoreLocal { local, src } => {
                let src_slot = self.load_operand(src);
                let dst_slot = self.local_slot(*local);
                // Only emit if src != dst
                if src_slot != dst_slot {
                    self.writer.emit_move(dst_slot, src_slot);
                }
            }
            // LoadLocal: move from local slot to temp slot
            Inst::LoadLocal { dst, local } => {
                let dst_slot = self.vreg_to_physical(*dst);
                let src_slot = self.local_slot(*local);
                // Only emit if src != dst
                if src_slot != dst_slot {
                    self.writer.emit_move(dst_slot, src_slot);
                }
            }

            Inst::Jump { target } => {
                let label = self.get_or_create_label(*target);
                let opcode_offset = self.writer.emit_jump_placeholder();
                self.label_patches
                    .push((opcode_offset, label, JumpKind::Jump));
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
                let opcode_offset = self.writer.emit_jump_if_placeholder(cond_slot);
                self.label_patches
                    .push((opcode_offset, then_label, JumpKind::JumpIf));

                // Jump -> else_bb
                let opcode_offset = self.writer.emit_jump_placeholder();
                self.label_patches
                    .push((opcode_offset, else_label, JumpKind::Jump));
            }

            Inst::Return { value } => {
                let src = value.as_ref().map(|v| self.load_operand(v));
                self.writer.emit_return(src);
            }

            // Function calls
            Inst::Call { dst, func, args } => {
                // Load args into consecutive slots
                let arg_base = self.alloc_consecutive_slots(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let s = arg_base + i as u8;
                    self.load_operand_to(arg, s);
                }

                let dst_slot = dst.map(|v| self.vreg_to_physical(v));
                self.writer
                    .emit_call(dst_slot, func.0 as u16, arg_base, args.len() as u8);
            }
            Inst::CallIndirect { dst, callee, args } => {
                let callee_slot = self.load_operand(callee);

                // Load args into consecutive slots
                let arg_base = self.alloc_consecutive_slots(args.len());
                for (i, arg) in args.iter().enumerate() {
                    let s = arg_base + i as u8;
                    self.load_operand_to(arg, s);
                }

                let dst_slot = dst.map(|v| self.vreg_to_physical(v));
                self.writer
                    .emit_call_indirect(dst_slot, callee_slot, arg_base, args.len() as u8);
            }
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                // Load captures into consecutive slots
                let capture_base = self.alloc_consecutive_slots(captures.len());
                for (i, cap) in captures.iter().enumerate() {
                    let s = capture_base + i as u8;
                    self.load_operand_to(cap, s);
                }

                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_make_closure(
                    dst_slot,
                    func.0 as u16,
                    capture_base,
                    captures.len() as u8,
                );
            }
            Inst::LoadCapture { dst, index } => {
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_load_capture(dst_slot, *index as u8);
            }
            Inst::StoreCapture { index, src } => {
                let src_slot = self.load_operand(src);
                self.writer.emit_store_capture(*index as u8, src_slot);
            }
            Inst::Echo { src, ty: _ } => {
                // VM already uses NaN-boxed values, ignore type
                let src_slot = self.load_operand(src);
                self.writer.emit_echo(src_slot);
            }

            // List operations
            Inst::ListNew { dst, capacity } => {
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_list_new(dst_slot, *capacity as u8);
            }
            Inst::ListSet { list, index, value } => {
                let list_slot = self.load_operand(list);
                let index_slot = self.load_operand(index);
                let value_slot = self.load_operand(value);
                self.writer.emit_list_set(list_slot, index_slot, value_slot);
            }
            Inst::ListGet { dst, list, index } => {
                let list_slot = self.load_operand(list);
                let index_slot = self.load_operand(index);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_list_get(dst_slot, list_slot, index_slot);
            }
            Inst::ListSlice {
                dst,
                list,
                start,
                end,
            } => {
                let list_slot = self.load_operand(list);
                // Use SLICE_MISSING sentinel for "missing" bounds
                let start_slot = match start {
                    Some(s) => self.load_operand(s),
                    None => {
                        let slot = self.alloc_slot();
                        self.writer.emit_load_int(slot, crate::SLICE_MISSING);
                        slot
                    }
                };
                let end_slot = match end {
                    Some(e) => self.load_operand(e),
                    None => {
                        let slot = self.alloc_slot();
                        self.writer.emit_load_int(slot, crate::SLICE_MISSING);
                        slot
                    }
                };
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer
                    .emit_list_slice(dst_slot, list_slot, start_slot, end_slot);
            }
            Inst::ListLen { dst, list } => {
                let list_slot = self.load_operand(list);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer.emit_list_len(dst_slot, list_slot);
            }

            // Tuple operations
            Inst::TupleNew { dst, elements } => {
                let elem_base = self.next_slot;
                for (i, elem) in elements.iter().enumerate() {
                    self.load_operand_to(elem, elem_base + i as u8);
                }
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer
                    .emit_tuple_new(dst_slot, elem_base, elements.len() as u8);
            }
            Inst::TupleGet { dst, tuple, index } => {
                let tuple_slot = self.load_operand(tuple);
                let dst_slot = self.vreg_to_physical(*dst);
                self.writer
                    .emit_tuple_get(dst_slot, tuple_slot, *index as u8);
            }
        }
    }

    fn load_operand(&mut self, op: &Operand) -> u8 {
        match op {
            Operand::VReg(v) => self.vreg_to_physical(*v),
            Operand::IntConst(n) => {
                let slot = self.alloc_slot();
                self.writer.emit_load_int(slot, *n);
                slot
            }
            Operand::FloatConst(f) => {
                let idx = self.chunk.constants.add_float(*f);
                let slot = self.alloc_slot();
                self.writer.emit_load_const(slot, idx.0 as u16);
                slot
            }
            Operand::BoolConst(b) => {
                let slot = self.alloc_slot();
                self.writer.emit_load_bool(slot, *b);
                slot
            }
            Operand::StringConst(s) => {
                let idx = self.chunk.constants.add_string(s, self.interner);
                let slot = self.alloc_slot();
                self.writer.emit_load_const(slot, idx.0 as u16);
                slot
            }
        }
    }

    fn load_operand_to(&mut self, op: &Operand, dst: u8) {
        match op {
            Operand::VReg(v) => {
                let src = self.vreg_to_physical(*v);
                if src != dst {
                    self.writer.emit_move(dst, src);
                }
            }
            Operand::IntConst(n) => {
                self.writer.emit_load_int(dst, *n);
            }
            Operand::FloatConst(f) => {
                let idx = self.chunk.constants.add_float(*f);
                self.writer.emit_load_const(dst, idx.0 as u16);
            }
            Operand::BoolConst(b) => {
                self.writer.emit_load_bool(dst, *b);
            }
            Operand::StringConst(s) => {
                let idx = self.chunk.constants.add_string(s, self.interner);
                self.writer.emit_load_const(dst, idx.0 as u16);
            }
        }
    }

    fn load_binary_operands(&mut self, lhs: &Operand, rhs: &Operand) -> (u8, u8) {
        let lhs_slot = self.load_operand(lhs);
        let rhs_slot = self.load_operand(rhs);
        (lhs_slot, rhs_slot)
    }

    fn into_chunk(mut self) -> Chunk {
        self.chunk.code = self.writer.finish();
        self.chunk
    }
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

        assert!(!compiled.main().code.is_empty());
    }
}
