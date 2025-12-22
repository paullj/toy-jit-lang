mod diagnostic;
mod lower;
mod symbols;

pub use diagnostic::HirDiagnostic;
pub use lower::{LowerResult, lower};
pub use symbols::{Symbol, SymbolKind, SymbolTable};
pub use syntax::TextRange;

use la_arena::Idx;

pub type ExprIdx = Idx<Expression>;

#[derive(Debug, Clone)]
pub enum Item {
    Definition(Definition),
    Assignment { name: String, value: Expression },
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub enum Definition {
    Variable { name: String, value: Expression },
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
        name: String,
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
}

/// Item inside a block expression
#[derive(Debug, Clone)]
pub enum BlockItem {
    Definition { name: String, value: ExprIdx },
    Assignment { name: String, value: ExprIdx },
    Expression(ExprIdx),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(u64),
    Float(f64),
    Boolean(bool),
    String(String),
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
