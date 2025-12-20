use std::fmt;

/// Physical register
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub u32);

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
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

/// Local variable slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalSlot(pub u32);

impl fmt::Display for LocalSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slot{}", self.0)
    }
}

/// Bytecode instruction
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Loads
    LoadInt { dst: Reg, value: i64 },
    LoadBool { dst: Reg, value: bool },
    LoadConst { dst: Reg, idx: ConstIdx },

    // Move
    Move { dst: Reg, src: Reg },

    // Integer arithmetic
    AddInt { dst: Reg, lhs: Reg, rhs: Reg },
    SubInt { dst: Reg, lhs: Reg, rhs: Reg },
    MulInt { dst: Reg, lhs: Reg, rhs: Reg },
    DivInt { dst: Reg, lhs: Reg, rhs: Reg },
    ModInt { dst: Reg, lhs: Reg, rhs: Reg },
    NegInt { dst: Reg, src: Reg },

    // Float arithmetic
    AddFloat { dst: Reg, lhs: Reg, rhs: Reg },
    SubFloat { dst: Reg, lhs: Reg, rhs: Reg },
    MulFloat { dst: Reg, lhs: Reg, rhs: Reg },
    DivFloat { dst: Reg, lhs: Reg, rhs: Reg },
    NegFloat { dst: Reg, src: Reg },

    // Integer comparisons
    EqInt { dst: Reg, lhs: Reg, rhs: Reg },
    NeInt { dst: Reg, lhs: Reg, rhs: Reg },
    LtInt { dst: Reg, lhs: Reg, rhs: Reg },
    LeInt { dst: Reg, lhs: Reg, rhs: Reg },
    GtInt { dst: Reg, lhs: Reg, rhs: Reg },
    GeInt { dst: Reg, lhs: Reg, rhs: Reg },

    // Float comparisons
    LtFloat { dst: Reg, lhs: Reg, rhs: Reg },
    LeFloat { dst: Reg, lhs: Reg, rhs: Reg },
    GtFloat { dst: Reg, lhs: Reg, rhs: Reg },
    GeFloat { dst: Reg, lhs: Reg, rhs: Reg },

    // Boolean
    Not { dst: Reg, src: Reg },

    // Local variables
    StoreLocal { slot: LocalSlot, src: Reg },
    LoadLocal { dst: Reg, slot: LocalSlot },

    // Control flow
    Jump { target: Label },
    JumpIf { cond: Reg, target: Label },
    JumpIfNot { cond: Reg, target: Label },

    // End
    Halt,
}
