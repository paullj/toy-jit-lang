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

    // Control flow - jumps (6)
    JumpFwd = 26,       // offset:u16 (forward, add to PC)
    JumpBack = 27,      // offset:u16 (backward, subtract from PC)
    JumpIfFwd = 28,     // cond:u8, offset:u16
    JumpIfBack = 29,    // cond:u8, offset:u16
    JumpIfNotFwd = 30,  // cond:u8, offset:u16
    JumpIfNotBack = 31, // cond:u8, offset:u16

    // Calls (3)
    Call = 32,         // dst:u8 (0xFF=none), func_idx:u16, arg_base:u8, arg_count:u8
    CallIndirect = 33, // dst:u8, callee:u8, arg_base:u8, arg_count:u8
    Return = 34,       // src:u8 (0xFF=none)

    // Closures (3)
    MakeClosure = 35,  // dst:u8, func_idx:u16, capture_base:u8, capture_count:u8
    LoadCapture = 36,  // dst:u8, index:u8
    StoreCapture = 37, // index:u8, src:u8

    // I/O (1)
    Echo = 38, // src:u8

    // List operations (4)
    ListNew = 39,   // dst:u8, capacity:u8
    ListSet = 40,   // list:u8, index:u8, value:u8
    ListGet = 41,   // dst:u8, list:u8, index:u8
    ListSlice = 42, // dst:u8, list:u8, start:u8, end:u8

    // End (1)
    Halt = 43,
}

impl Opcode {
    /// Decode opcode from u8.
    #[inline(always)]
    pub fn from_u8(byte: u8) -> Option<Opcode> {
        if byte <= Self::Halt as u8 {
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

/// Sentinel value for "missing" slice bounds.
/// Must fit in 48-bit NaN-boxed payload (so NOT i64::MIN).
/// Uses largest magnitude negative 48-bit value: -(2^47) = -140737488355328
pub const SLICE_MISSING: i64 = -(1i64 << 47);
