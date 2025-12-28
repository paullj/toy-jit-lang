use std::fmt;

use crate::chunk::{Chunk, CompiledModule};
use crate::opcode::{NO_SLOT, Opcode};

/// Disassemble a single instruction at the given offset, returning the next offset.
fn disassemble_inst(code: &[u8], offset: usize, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:4}: ", offset)?;

    let op_byte = code[offset];
    let op = Opcode::from_u8(op_byte).expect("invalid opcode");

    match op {
        Opcode::LoadInt => {
            let dst = code[offset + 1];
            let value = i64::from_le_bytes(code[offset + 2..offset + 10].try_into().unwrap());
            writeln!(f, "load.i s{} {}", dst, value)
        }
        Opcode::LoadBool => {
            let dst = code[offset + 1];
            let value = code[offset + 2] != 0;
            writeln!(f, "load.b s{} {}", dst, value)
        }
        Opcode::LoadConst => {
            let dst = code[offset + 1];
            let idx = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            writeln!(f, "load.c s{} c{}", dst, idx)
        }
        Opcode::Move => {
            let dst = code[offset + 1];
            let src = code[offset + 2];
            writeln!(f, "move s{} s{}", dst, src)
        }
        Opcode::AddInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "add.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::SubInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "sub.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::MulInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "mul.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::DivInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "div.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::ModInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "mod.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::NegInt => {
            let dst = code[offset + 1];
            let src = code[offset + 2];
            writeln!(f, "neg.i s{} s{}", dst, src)
        }
        Opcode::AddFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "add.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::SubFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "sub.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::MulFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "mul.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::DivFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "div.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::NegFloat => {
            let dst = code[offset + 1];
            let src = code[offset + 2];
            writeln!(f, "neg.f s{} s{}", dst, src)
        }
        Opcode::EqInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "eq.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::NeInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "ne.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::LtInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "lt.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::LeInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "le.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::GtInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "gt.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::GeInt => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "ge.i s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::LtFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "lt.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::LeFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "le.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::GtFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "gt.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::GeFloat => {
            let dst = code[offset + 1];
            let lhs = code[offset + 2];
            let rhs = code[offset + 3];
            writeln!(f, "ge.f s{} s{} s{}", dst, lhs, rhs)
        }
        Opcode::Not => {
            let dst = code[offset + 1];
            let src = code[offset + 2];
            writeln!(f, "not s{} s{}", dst, src)
        }
        Opcode::JumpFwd => {
            let rel_offset = u16::from_le_bytes([code[offset + 1], code[offset + 2]]);
            let target = offset + 3 + rel_offset as usize;
            writeln!(f, "jump.fwd @{}", target)
        }
        Opcode::JumpBack => {
            let rel_offset = u16::from_le_bytes([code[offset + 1], code[offset + 2]]);
            let target = offset + 3 - rel_offset as usize;
            writeln!(f, "jump.back @{}", target)
        }
        Opcode::JumpIfFwd => {
            let cond = code[offset + 1];
            let rel_offset = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            let target = offset + 4 + rel_offset as usize;
            writeln!(f, "jump.if.fwd s{} @{}", cond, target)
        }
        Opcode::JumpIfBack => {
            let cond = code[offset + 1];
            let rel_offset = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            let target = offset + 4 - rel_offset as usize;
            writeln!(f, "jump.if.back s{} @{}", cond, target)
        }
        Opcode::JumpIfNotFwd => {
            let cond = code[offset + 1];
            let rel_offset = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            let target = offset + 4 + rel_offset as usize;
            writeln!(f, "jump.ifn.fwd s{} @{}", cond, target)
        }
        Opcode::JumpIfNotBack => {
            let cond = code[offset + 1];
            let rel_offset = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            let target = offset + 4 - rel_offset as usize;
            writeln!(f, "jump.ifn.back s{} @{}", cond, target)
        }
        Opcode::Call => {
            let dst = code[offset + 1];
            let func_idx = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            let arg_base = code[offset + 4];
            let arg_count = code[offset + 5];
            if dst == NO_SLOT {
                writeln!(f, "call _ fn{} s{} {}", func_idx, arg_base, arg_count)
            } else {
                writeln!(
                    f,
                    "call s{} fn{} s{} {}",
                    dst, func_idx, arg_base, arg_count
                )
            }
        }
        Opcode::CallIndirect => {
            let dst = code[offset + 1];
            let callee = code[offset + 2];
            let arg_base = code[offset + 3];
            let arg_count = code[offset + 4];
            if dst == NO_SLOT {
                writeln!(f, "call.i _ s{} s{} {}", callee, arg_base, arg_count)
            } else {
                writeln!(f, "call.i s{} s{} s{} {}", dst, callee, arg_base, arg_count)
            }
        }
        Opcode::Return => {
            let src = code[offset + 1];
            if src == NO_SLOT {
                writeln!(f, "ret")
            } else {
                writeln!(f, "ret s{}", src)
            }
        }
        Opcode::MakeClosure => {
            let dst = code[offset + 1];
            let func_idx = u16::from_le_bytes([code[offset + 2], code[offset + 3]]);
            let capture_base = code[offset + 4];
            let capture_count = code[offset + 5];
            writeln!(
                f,
                "closure s{} fn{} s{} {}",
                dst, func_idx, capture_base, capture_count
            )
        }
        Opcode::LoadCapture => {
            let dst = code[offset + 1];
            let index = code[offset + 2];
            writeln!(f, "load.cap s{} {}", dst, index)
        }
        Opcode::StoreCapture => {
            let index = code[offset + 1];
            let src = code[offset + 2];
            writeln!(f, "store.cap {} s{}", index, src)
        }
        Opcode::Echo => {
            let src = code[offset + 1];
            writeln!(f, "echo s{}", src)
        }
        Opcode::ListNew => {
            let dst = code[offset + 1];
            let capacity = code[offset + 2];
            writeln!(f, "list.new s{} {}", dst, capacity)
        }
        Opcode::ListSet => {
            let list = code[offset + 1];
            let index = code[offset + 2];
            let value = code[offset + 3];
            writeln!(f, "list.set s{} {} s{}", list, index, value)
        }
        Opcode::ListGet => {
            let dst = code[offset + 1];
            let list = code[offset + 2];
            let index = code[offset + 3];
            writeln!(f, "list.get s{} s{} s{}", dst, list, index)
        }
        Opcode::ListSlice => {
            let dst = code[offset + 1];
            let list = code[offset + 2];
            let start = code[offset + 3];
            let end = code[offset + 4];
            writeln!(f, "list.slice s{} s{} s{} s{}", dst, list, start, end)
        }
        Opcode::ListLen => {
            let dst = code[offset + 1];
            let list = code[offset + 2];
            writeln!(f, "list.len s{} s{}", dst, list)
        }
        Opcode::TupleNew => {
            let dst = code[offset + 1];
            let elem_base = code[offset + 2];
            let elem_count = code[offset + 3];
            writeln!(f, "tuple.new s{} s{} {}", dst, elem_base, elem_count)
        }
        Opcode::TupleGet => {
            let dst = code[offset + 1];
            let tuple = code[offset + 2];
            let index = code[offset + 3];
            writeln!(f, "tuple.get s{} s{} {}", dst, tuple, index)
        }
        Opcode::Halt => {
            writeln!(f, "halt")
        }
    }
}

/// Get instruction size in bytes
fn inst_size(op: Opcode) -> usize {
    match op {
        Opcode::LoadInt => 10,  // op + dst + i64
        Opcode::LoadBool => 3,  // op + dst + bool
        Opcode::LoadConst => 4, // op + dst + idx:u16
        Opcode::Move => 3,      // op + dst + src
        Opcode::AddInt
        | Opcode::SubInt
        | Opcode::MulInt
        | Opcode::DivInt
        | Opcode::ModInt
        | Opcode::AddFloat
        | Opcode::SubFloat
        | Opcode::MulFloat
        | Opcode::DivFloat
        | Opcode::EqInt
        | Opcode::NeInt
        | Opcode::LtInt
        | Opcode::LeInt
        | Opcode::GtInt
        | Opcode::GeInt
        | Opcode::LtFloat
        | Opcode::LeFloat
        | Opcode::GtFloat
        | Opcode::GeFloat => 4, // op + dst + lhs + rhs
        Opcode::NegInt | Opcode::NegFloat | Opcode::Not => 3, // op + dst + src
        Opcode::JumpFwd | Opcode::JumpBack => 3, // op + offset:u16
        Opcode::JumpIfFwd | Opcode::JumpIfBack | Opcode::JumpIfNotFwd | Opcode::JumpIfNotBack => 4, // op + cond + offset:u16
        Opcode::Call => 6,         // op + dst + func:u16 + base + count
        Opcode::CallIndirect => 5, // op + dst + callee + base + count
        Opcode::Return => 2,       // op + src
        Opcode::MakeClosure => 6,  // op + dst + func:u16 + base + count
        Opcode::LoadCapture => 3,  // op + dst + idx
        Opcode::StoreCapture => 3, // op + idx + src
        Opcode::Echo => 2,         // op + src
        Opcode::ListNew => 3,      // op + dst + capacity
        Opcode::ListSet => 4,      // op + list + index + value
        Opcode::ListGet => 4,      // op + dst + list + index
        Opcode::ListSlice => 5,    // op + dst + list + start + end
        Opcode::ListLen => 3,      // op + dst + list
        Opcode::TupleNew => 4,     // op + dst + elem_base + elem_count
        Opcode::TupleGet => 4,     // op + dst + tuple + index
        Opcode::Halt => 1,         // op
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "; params: {}, locals: {}, registers: {}",
            self.param_count, self.local_count, self.register_count
        )?;

        if !self.constants.is_empty() {
            writeln!(f, "; constants:")?;
            for (idx, c) in self.constants.iter() {
                writeln!(f, ";   {} = {}", idx, c)?;
            }
        }

        writeln!(f)?;

        let mut offset = 0;
        while offset < self.code.len() {
            disassemble_inst(&self.code, offset, f)?;
            let op = Opcode::from_u8(self.code[offset]).expect("invalid opcode");
            offset += inst_size(op);
        }
        Ok(())
    }
}

impl fmt::Display for CompiledModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, chunk) in self.chunks.iter().enumerate() {
            if i == self.main_idx {
                writeln!(f, "=== fn{} (main) ===", i)?;
            } else {
                writeln!(f, "=== fn{} ===", i)?;
            }
            write!(f, "{}", chunk)?;
            if i < self.chunks.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
