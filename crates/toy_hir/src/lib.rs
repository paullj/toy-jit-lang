mod error;
mod lower;
mod symbols;

pub use error::HirError;
pub use lower::{
    LowerItemResult, LowerResult, ResolutionData, lower, lower_single_item,
    lower_single_item_with_resolution, lower_with_resolution,
};
pub use symbols::{Symbol, SymbolKind, SymbolTable};
pub use toy_cst::TextRange;

use la_arena::Idx;
use lasso::{Key, Spur};
use std::fmt;

/// Re-export ThreadedRodeo for downstream crates (lock-free reads, synchronized writes)
pub use lasso::ThreadedRodeo as Interner;

pub type ExprIdx = Idx<Expression>;

/// Interned identifier.
///
/// This is a newtype around `Spur` to distinguish identifiers from other interned strings.
/// Use `LowerResult::resolve(&self, ident)` to get the string value.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(Spur);

impl Ident {
    /// Create a new Ident from a Spur
    pub fn new(spur: Spur) -> Self {
        Self(spur)
    }

    /// Get the underlying Spur
    pub fn spur(self) -> Spur {
        self.0
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ident({})", self.0.into_usize())
    }
}

/// Module identifier for cross-module references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

/// Resolved identifier with module context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolvedIdent {
    /// Local identifier in current module
    Local(Ident),
    /// External identifier from another module
    External { module_id: ModuleId, name: Ident },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Definition(Definition),
    Assignment {
        name: Ident,
        value: Expression,
    },
    IndexAssignment {
        collection: ExprIdx,
        index: ExprIdx,
        value: Expression,
    },
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam {
    pub name: Ident,
    pub ty: Option<Ident>,
    pub default: Option<ExprIdx>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    Variable {
        name: Ident,
        value: Expression,
    },
    Function {
        name: Ident,
        params: Vec<FunctionParam>,
        return_type: Option<Ident>,
        body: ExprIdx,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Missing,
    Literal(Literal),
    Infix {
        op: InfixOp,
        lhs: ExprIdx,
        rhs: ExprIdx,
    },
    Prefix {
        op: PrefixOp,
        expr: ExprIdx,
    },
    VariableRef {
        name: ResolvedIdent,
    },
    Block {
        items: Vec<BlockItem>,
        tail: Option<ExprIdx>,
    },
    If {
        condition: ExprIdx,
        then_branch: ExprIdx,
        else_branch: Option<ExprIdx>,
    },
    Function {
        params: Vec<FunctionParam>,
        return_type: Option<Ident>,
        body: ExprIdx,
        captures: Vec<Ident>,
    },
    Call {
        callee: ExprIdx,
        args: Vec<ExprIdx>,
    },
    Return {
        value: Option<ExprIdx>,
    },
    Echo {
        value: ExprIdx,
    },
    Loop {
        label: Option<Ident>,
        body: ExprIdx,
    },
    While {
        condition: ExprIdx,
        label: Option<Ident>,
        body: ExprIdx,
    },
    For {
        binding: Ident,
        iterable: ExprIdx,
        label: Option<Ident>,
        body: ExprIdx,
    },
    Range {
        start: ExprIdx,
        end: ExprIdx,
    },
    Break {
        label: Option<Ident>,
    },
    Continue {
        label: Option<Ident>,
    },
    List {
        elements: Vec<ExprIdx>,
    },
    Index {
        collection: ExprIdx,
        index: ExprIdx,
    },
    Slice {
        collection: ExprIdx,
        start: Option<ExprIdx>,
        end: Option<ExprIdx>,
    },
    Tuple {
        elements: Vec<ExprIdx>,
    },
    TupleAccess {
        tuple: ExprIdx,
        index: u32,
    },
}

/// Item inside a block expression
#[derive(Debug, Clone, PartialEq)]
pub enum BlockItem {
    Definition {
        name: Ident,
        value: ExprIdx,
    },
    Assignment {
        name: Ident,
        value: ExprIdx,
    },
    IndexAssignment {
        collection: ExprIdx,
        index: ExprIdx,
        value: ExprIdx,
    },
    Expression(ExprIdx),
    Return {
        value: Option<ExprIdx>,
    },
    Echo {
        value: ExprIdx,
    },
    Break {
        label: Option<Ident>,
    },
    Continue {
        label: Option<Ident>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(u64),
    Float(f64),
    Boolean(bool),
    String(String), // String literals stay as String (not identifiers)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    // Int arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Float arithmetic
    AddFloat,
    SubFloat,
    MulFloat,
    DivFloat,
    // Comparison (int)
    Eq,
    NotEq,
    Gt,
    Lt,
    Gte,
    Lte,
    // Comparison (float)
    GtFloat,
    LtFloat,
    GteFloat,
    LteFloat,
    // Boolean
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    Neg,
    Not,
}
