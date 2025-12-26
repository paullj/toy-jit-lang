use std::fmt;

/// Stack slot index (unified: locals at [0, local_count), temps at [local_count, frame_size))
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot(pub u32);

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s{}", self.0)
    }
}

/// Constant pool index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstIdx(pub u32);

impl fmt::Display for ConstIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}

/// Jump target label
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}", self.0)
    }
}

/// Function index in compiled module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncIdx(pub u32);

impl fmt::Display for FuncIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn{}", self.0)
    }
}

/// Legacy bytecode instruction enum (kept for reference, no longer used)
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Loads
    LoadInt {
        dst: Slot,
        value: i64,
    },
    LoadBool {
        dst: Slot,
        value: bool,
    },
    LoadConst {
        dst: Slot,
        idx: ConstIdx,
    },

    // Move
    Move {
        dst: Slot,
        src: Slot,
    },

    // Integer arithmetic
    AddInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    SubInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    MulInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    DivInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    ModInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    NegInt {
        dst: Slot,
        src: Slot,
    },

    // Float arithmetic
    AddFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    SubFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    MulFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    DivFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    NegFloat {
        dst: Slot,
        src: Slot,
    },

    // Integer comparisons
    EqInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    NeInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    LtInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    LeInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    GtInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    GeInt {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },

    // Float comparisons
    LtFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    LeFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    GtFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },
    GeFloat {
        dst: Slot,
        lhs: Slot,
        rhs: Slot,
    },

    // Boolean
    Not {
        dst: Slot,
        src: Slot,
    },

    // Control flow
    Jump {
        target: Label,
    },
    JumpIf {
        cond: Slot,
        target: Label,
    },
    JumpIfNot {
        cond: Slot,
        target: Label,
    },

    // Function calls
    /// Direct call to function at index
    Call {
        dst: Option<Slot>,
        func_idx: FuncIdx,
        arg_base: Slot,
        arg_count: u8,
    },
    /// Indirect call through closure value
    CallIndirect {
        dst: Option<Slot>,
        callee: Slot,
        arg_base: Slot,
        arg_count: u8,
    },
    /// Return from function
    Return {
        src: Option<Slot>,
    },

    // Closures
    /// Create closure value
    MakeClosure {
        dst: Slot,
        func_idx: FuncIdx,
        capture_base: Slot,
        capture_count: u8,
    },
    /// Load from closure environment
    LoadCapture {
        dst: Slot,
        index: u8,
    },
    /// Store to closure environment
    StoreCapture {
        index: u8,
        src: Slot,
    },

    // I/O
    /// Print value to stdout
    Echo {
        src: Slot,
    },

    // End
    Halt,
}
