use std::fmt;

/// Virtual register
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VReg(pub u32);

impl fmt::Display for VReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// Basic block identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// Local variable slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

impl fmt::Display for LocalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "local{}", self.0)
    }
}

/// Operand: either a virtual register or an immediate constant
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    VReg(VReg),
    IntConst(i64),
    FloatConst(f64),
    BoolConst(bool),
    StringConst(String),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::VReg(v) => write!(f, "{}", v),
            Operand::IntConst(n) => write!(f, "{}", n),
            Operand::FloatConst(n) => write!(f, "{}", n),
            Operand::BoolConst(b) => write!(f, "{}", b),
            Operand::StringConst(s) => write!(f, "\"{}\"", s.escape_default()),
        }
    }
}

/// MIR instruction
#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    // Integer arithmetic
    AddInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    SubInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    MulInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    DivInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    ModInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    NegInt {
        dst: VReg,
        src: Operand,
    },

    // Float arithmetic
    AddFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    SubFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    MulFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    DivFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    NegFloat {
        dst: VReg,
        src: Operand,
    },

    // Integer comparisons
    EqInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    NeInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    LtInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    LeInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    GtInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    GeInt {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },

    // Float comparisons
    LtFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    LeFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    GtFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },
    GeFloat {
        dst: VReg,
        lhs: Operand,
        rhs: Operand,
    },

    // Boolean
    Not {
        dst: VReg,
        src: Operand,
    },

    // Copy operand to vreg (for constants/moves)
    Copy {
        dst: VReg,
        src: Operand,
    },

    // Local variables
    StoreLocal {
        local: LocalId,
        src: Operand,
    },
    LoadLocal {
        dst: VReg,
        local: LocalId,
    },

    // Control flow
    Jump {
        target: BlockId,
    },
    Branch {
        cond: Operand,
        then_bb: BlockId,
        else_bb: BlockId,
    },
    Return {
        value: Option<Operand>,
    },
}

/// Basic block containing a sequence of instructions
#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub insts: Vec<Inst>,
}

impl Block {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            insts: Vec::new(),
        }
    }

    pub fn push(&mut self, inst: Inst) {
        self.insts.push(inst);
    }
}

/// A function containing basic blocks
#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>,
    pub blocks: Vec<Block>,
    pub local_count: u32,
    pub vreg_count: u32,
}

impl Function {
    pub fn new(name: Option<String>) -> Self {
        Self {
            name,
            blocks: Vec::new(),
            local_count: 0,
            vreg_count: 0,
        }
    }

    pub fn entry_block(&self) -> Option<&Block> {
        self.blocks.first()
    }
}

/// A compiled module containing the main function
#[derive(Debug, Clone)]
pub struct Module {
    pub main: Function,
}
