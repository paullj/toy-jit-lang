mod diagnostic;
mod lower;
mod symbols;

pub use diagnostic::HirDiagnostic;
pub use lower::{LowerResult, lower};
pub use symbols::{Symbol, SymbolKind, SymbolTable};
pub use syntax::TextRange;

use la_arena::Idx;
use lasso::{Key, Spur};
use std::fmt;

/// Re-export Rodeo for downstream crates
pub use lasso::Rodeo as Interner;

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

#[derive(Debug, Clone)]
pub enum Item {
    Definition(Definition),
    StructDefinition(StructDef),
    Assignment {
        name: Ident,
        value: Expression,
    },
    IndexAssignment {
        collection: ExprIdx,
        index: ExprIdx,
        value: Expression,
    },
    FieldAssignment {
        object: ExprIdx,
        field: Ident,
        value: Expression,
    },
    Expression(Expression),
}

/// Struct field definition (name and type)
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: Ident,
    pub ty: Ident,
}

/// Struct type definition
#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: Ident,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub name: Ident,
    pub ty: Option<Ident>,
    pub default: Option<ExprIdx>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
        name: Ident,
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
    /// Struct literal expression: `Point { x: 1, y: 2 }`
    Struct {
        name: Ident,
        fields: Vec<(Ident, ExprIdx)>,
        spread: Option<ExprIdx>,
    },
    /// Field access expression: `point.x`
    FieldAccess {
        object: ExprIdx,
        field: Ident,
    },
}

/// Item inside a block expression
#[derive(Debug, Clone)]
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
    /// Field assignment in a block: `point.x = 10`
    FieldAssignment {
        object: ExprIdx,
        field: Ident,
        value: ExprIdx,
    },
}

#[derive(Debug, Clone)]
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
