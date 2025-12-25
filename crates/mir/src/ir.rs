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

/// Unique function identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

impl fmt::Display for FuncId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn{}", self.0)
    }
}

/// Captured variable info for closures
#[derive(Debug, Clone)]
pub struct CapturedVar {
    pub name: String,
    pub outer_local: LocalId,
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

    // Function calls
    /// Direct call to known function
    Call {
        dst: Option<VReg>,
        func: FuncId,
        args: Vec<Operand>,
    },
    /// Indirect call through function value (closure/fn ptr)
    CallIndirect {
        dst: Option<VReg>,
        callee: Operand,
        args: Vec<Operand>,
    },

    // Closures
    /// Create closure (function + captured environment)
    MakeClosure {
        dst: VReg,
        func: FuncId,
        captures: Vec<Operand>,
    },
    /// Load captured variable inside closure body
    LoadCapture {
        dst: VReg,
        index: u32,
    },
    /// Store to captured variable (for mutable captures)
    StoreCapture {
        index: u32,
        src: Operand,
    },

    /// Print value to stdout
    Echo {
        src: Operand,
    },
}

impl Inst {
    /// Returns true if this instruction is a block terminator
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Inst::Jump { .. } | Inst::Branch { .. } | Inst::Return { .. }
        )
    }
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

    /// Returns true if this block ends with a terminator instruction
    pub fn is_terminated(&self) -> bool {
        self.insts.last().is_some_and(|i| i.is_terminator())
    }
}

/// A function containing basic blocks
#[derive(Debug, Clone)]
pub struct Function {
    pub id: FuncId,
    pub name: Option<String>,
    pub params: Vec<LocalId>,
    pub param_count: u32,
    pub blocks: Vec<Block>,
    pub local_count: u32,
    pub vreg_count: u32,
    pub captures: Vec<CapturedVar>,
    pub is_closure: bool,
}

impl Function {
    pub fn new(id: FuncId, name: Option<String>) -> Self {
        Self {
            id,
            name,
            params: Vec::new(),
            param_count: 0,
            blocks: Vec::new(),
            local_count: 0,
            vreg_count: 0,
            captures: Vec::new(),
            is_closure: false,
        }
    }

    pub fn entry_block(&self) -> Option<&Block> {
        self.blocks.first()
    }
}

/// A compiled module containing functions
#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
    pub main_id: FuncId,
}

impl Module {
    pub fn main(&self) -> &Function {
        &self.functions[self.main_id.0 as usize]
    }

    pub fn get_func(&self, id: FuncId) -> &Function {
        &self.functions[id.0 as usize]
    }
}
