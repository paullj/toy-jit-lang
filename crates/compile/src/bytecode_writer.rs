use crate::opcode::{NO_SLOT, Opcode};

/// Writer for compact bytecode encoding.
pub struct BytecodeWriter {
    code: Vec<u8>,
}

impl BytecodeWriter {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    pub fn finish(self) -> Vec<u8> {
        self.code
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    #[inline]
    fn write_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    #[inline]
    fn write_i16(&mut self, v: i16) {
        self.code.extend(v.to_le_bytes());
    }

    #[inline]
    fn write_u16(&mut self, v: u16) {
        self.code.extend(v.to_le_bytes());
    }

    #[inline]
    fn write_i64(&mut self, v: i64) {
        self.code.extend(v.to_le_bytes());
    }

    #[inline]
    fn emit_op(&mut self, op: Opcode) {
        self.write_u8(op as u8);
    }

    /// Patch an i16 at a given offset.
    pub fn patch_i16(&mut self, offset: usize, v: i16) {
        self.code[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
    }

    // === Emit methods ===

    /// LoadInt: dst:u8, value:i64
    pub fn emit_load_int(&mut self, dst: u8, value: i64) {
        self.emit_op(Opcode::LoadInt);
        self.write_u8(dst);
        self.write_i64(value);
    }

    /// LoadBool: dst:u8, value:u8
    pub fn emit_load_bool(&mut self, dst: u8, value: bool) {
        self.emit_op(Opcode::LoadBool);
        self.write_u8(dst);
        self.write_u8(value as u8);
    }

    /// LoadConst: dst:u8, idx:u16
    pub fn emit_load_const(&mut self, dst: u8, idx: u16) {
        self.emit_op(Opcode::LoadConst);
        self.write_u8(dst);
        self.write_u16(idx);
    }

    /// Move: dst:u8, src:u8
    pub fn emit_move(&mut self, dst: u8, src: u8) {
        self.emit_op(Opcode::Move);
        self.write_u8(dst);
        self.write_u8(src);
    }

    /// Binary int op: dst:u8, lhs:u8, rhs:u8
    pub fn emit_add_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::AddInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_sub_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::SubInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_mul_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::MulInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_div_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::DivInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_mod_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::ModInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    /// NegInt: dst:u8, src:u8
    pub fn emit_neg_int(&mut self, dst: u8, src: u8) {
        self.emit_op(Opcode::NegInt);
        self.write_u8(dst);
        self.write_u8(src);
    }

    /// Binary float ops
    pub fn emit_add_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::AddFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_sub_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::SubFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_mul_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::MulFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_div_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::DivFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_neg_float(&mut self, dst: u8, src: u8) {
        self.emit_op(Opcode::NegFloat);
        self.write_u8(dst);
        self.write_u8(src);
    }

    /// Int comparisons
    pub fn emit_eq_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::EqInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_ne_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::NeInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_lt_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::LtInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_le_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::LeInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_gt_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::GtInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_ge_int(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::GeInt);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    /// Float comparisons
    pub fn emit_lt_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::LtFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_le_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::LeFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_gt_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::GtFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    pub fn emit_ge_float(&mut self, dst: u8, lhs: u8, rhs: u8) {
        self.emit_op(Opcode::GeFloat);
        self.write_u8(dst);
        self.write_u8(lhs);
        self.write_u8(rhs);
    }

    /// Not: dst:u8, src:u8
    pub fn emit_not(&mut self, dst: u8, src: u8) {
        self.emit_op(Opcode::Not);
        self.write_u8(dst);
        self.write_u8(src);
    }

    /// Jump: offset:i16 (relative). Returns offset to patch.
    pub fn emit_jump(&mut self) -> usize {
        self.emit_op(Opcode::Jump);
        let offset = self.code.len();
        self.write_i16(0); // placeholder
        offset
    }

    /// JumpIf: cond:u8, offset:i16. Returns offset to patch.
    pub fn emit_jump_if(&mut self, cond: u8) -> usize {
        self.emit_op(Opcode::JumpIf);
        self.write_u8(cond);
        let offset = self.code.len();
        self.write_i16(0); // placeholder
        offset
    }

    /// JumpIfNot: cond:u8, offset:i16. Returns offset to patch.
    #[allow(dead_code)]
    pub fn emit_jump_if_not(&mut self, cond: u8) -> usize {
        self.emit_op(Opcode::JumpIfNot);
        self.write_u8(cond);
        let offset = self.code.len();
        self.write_i16(0); // placeholder
        offset
    }

    /// Call: dst:u8 (0xFF=none), func_idx:u16, arg_base:u8, arg_count:u8
    pub fn emit_call(&mut self, dst: Option<u8>, func_idx: u16, arg_base: u8, arg_count: u8) {
        self.emit_op(Opcode::Call);
        self.write_u8(dst.unwrap_or(NO_SLOT));
        self.write_u16(func_idx);
        self.write_u8(arg_base);
        self.write_u8(arg_count);
    }

    /// CallIndirect: dst:u8 (0xFF=none), callee:u8, arg_base:u8, arg_count:u8
    pub fn emit_call_indirect(&mut self, dst: Option<u8>, callee: u8, arg_base: u8, arg_count: u8) {
        self.emit_op(Opcode::CallIndirect);
        self.write_u8(dst.unwrap_or(NO_SLOT));
        self.write_u8(callee);
        self.write_u8(arg_base);
        self.write_u8(arg_count);
    }

    /// Return: src:u8 (0xFF=none)
    pub fn emit_return(&mut self, src: Option<u8>) {
        self.emit_op(Opcode::Return);
        self.write_u8(src.unwrap_or(NO_SLOT));
    }

    /// MakeClosure: dst:u8, func_idx:u16, capture_base:u8, capture_count:u8
    pub fn emit_make_closure(
        &mut self,
        dst: u8,
        func_idx: u16,
        capture_base: u8,
        capture_count: u8,
    ) {
        self.emit_op(Opcode::MakeClosure);
        self.write_u8(dst);
        self.write_u16(func_idx);
        self.write_u8(capture_base);
        self.write_u8(capture_count);
    }

    /// LoadCapture: dst:u8, index:u8
    pub fn emit_load_capture(&mut self, dst: u8, index: u8) {
        self.emit_op(Opcode::LoadCapture);
        self.write_u8(dst);
        self.write_u8(index);
    }

    /// StoreCapture: index:u8, src:u8
    pub fn emit_store_capture(&mut self, index: u8, src: u8) {
        self.emit_op(Opcode::StoreCapture);
        self.write_u8(index);
        self.write_u8(src);
    }

    /// Echo: src:u8
    pub fn emit_echo(&mut self, src: u8) {
        self.emit_op(Opcode::Echo);
        self.write_u8(src);
    }

    /// Halt
    #[allow(dead_code)]
    pub fn emit_halt(&mut self) {
        self.emit_op(Opcode::Halt);
    }
}

impl Default for BytecodeWriter {
    fn default() -> Self {
        Self::new()
    }
}
