use crate::opcode;

/// Bytecode opcodes with compact u8 representation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    // Loads (3)
    LoadInt = 0,   // dst:u8, value:i64
    LoadBool = 1,  // dst:u8, value:u8
    LoadConst = 2, // dst:u8, idx:u16

    // Move (1)
    Move = 3, // dst:u8, src:u8

    // Int arithmetic (6)
    AddInt = 4, // dst:u8, lhs:u8, rhs:u8
    SubInt = 5,
    MulInt = 6,
    DivInt = 7,
    ModInt = 8,
    NegInt = 9, // dst:u8, src:u8

    // Float arithmetic (5)
    AddFloat = 10,
    SubFloat = 11,
    MulFloat = 12,
    DivFloat = 13,
    NegFloat = 14,

    // Int comparisons (6)
    EqInt = 15,
    NeInt = 16,
    LtInt = 17,
    LeInt = 18,
    GtInt = 19,
    GeInt = 20,

    // Float comparisons (4)
    LtFloat = 21,
    LeFloat = 22,
    GtFloat = 23,
    GeFloat = 24,

    // Boolean (1)
    Not = 25, // dst:u8, src:u8

    // Control flow (4)
    Jump = 26,      // offset:i16 (signed, relative)
    JumpIf = 27,    // cond:u8, offset:i16
    JumpIfNot = 28, // cond:u8, offset:i16

    // Calls (3)
    Call = 29,         // dst:u8 (0xFF=none), func_idx:u16, arg_base:u8, arg_count:u8
    CallIndirect = 30, // dst:u8, callee:u8, arg_base:u8, arg_count:u8
    Return = 31,       // src:u8 (0xFF=none)

    // Closures (3)
    MakeClosure = 32,  // dst:u8, func_idx:u16, capture_base:u8, capture_count:u8
    LoadCapture = 33,  // dst:u8, index:u8
    StoreCapture = 34, // index:u8, src:u8

    // I/O (1)
    Echo = 35, // src:u8

    // End (1)
    Halt = 36,
}

impl Opcode {
    /// Decode opcode from u8.
    #[inline(always)]
    pub fn from_u8(byte: u8) -> Option<Opcode> {
        if byte <= Opcode::Halt as u8 {
            // NOTE: We verified the byte is in range
            Some(unsafe { std::mem::transmute::<u8, opcode::Opcode>(byte) })
        } else {
            None
        }
    }

    /// Decode opcode from u8 without bounds checking.
    /// # Safety
    /// Caller must ensure byte is a valid opcode value.
    #[inline(always)]
    pub unsafe fn from_u8_unchecked(byte: u8) -> Opcode {
        // NOTE: Caller guarantees byte is valid opcode
        unsafe { std::mem::transmute(byte) }
    }
}

/// Sentinel value for "no slot" in optional slot fields
pub const NO_SLOT: u8 = 0xFF;
