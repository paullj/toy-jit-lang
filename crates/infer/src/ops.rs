use crate::types::Type;
use hir::{InfixOp, PrefixOp};

/// Type signature for a binary operator: (operand, operand) -> result
pub struct InfixSig {
    pub operand: Type,
    pub result: Type,
}

/// Returns the float equivalent of an int operator, if any
pub fn int_to_float_op(op: InfixOp) -> Option<InfixOp> {
    match op {
        InfixOp::Add => Some(InfixOp::AddFloat),
        InfixOp::Sub => Some(InfixOp::SubFloat),
        InfixOp::Mul => Some(InfixOp::MulFloat),
        InfixOp::Div => Some(InfixOp::DivFloat),
        InfixOp::Gt => Some(InfixOp::GtFloat),
        InfixOp::Lt => Some(InfixOp::LtFloat),
        InfixOp::Gte => Some(InfixOp::GteFloat),
        InfixOp::Lte => Some(InfixOp::LteFloat),
        _ => None,
    }
}

/// Returns the int equivalent of a float operator, if any
pub fn float_to_int_op(op: InfixOp) -> Option<InfixOp> {
    match op {
        InfixOp::AddFloat => Some(InfixOp::Add),
        InfixOp::SubFloat => Some(InfixOp::Sub),
        InfixOp::MulFloat => Some(InfixOp::Mul),
        InfixOp::DivFloat => Some(InfixOp::Div),
        InfixOp::GtFloat => Some(InfixOp::Gt),
        InfixOp::LtFloat => Some(InfixOp::Lt),
        InfixOp::GteFloat => Some(InfixOp::Gte),
        InfixOp::LteFloat => Some(InfixOp::Lte),
        _ => None,
    }
}

/// Get the display string for an operator
pub fn op_symbol(op: InfixOp) -> &'static str {
    match op {
        InfixOp::Add => "+",
        InfixOp::Sub => "-",
        InfixOp::Mul => "*",
        InfixOp::Div => "/",
        InfixOp::Mod => "%",
        InfixOp::AddFloat => "+.",
        InfixOp::SubFloat => "-.",
        InfixOp::MulFloat => "*.",
        InfixOp::DivFloat => "/.",
        InfixOp::Eq => "==",
        InfixOp::NotEq => "!=",
        InfixOp::Gt => ">",
        InfixOp::Lt => "<",
        InfixOp::Gte => ">=",
        InfixOp::Lte => "<=",
        InfixOp::GtFloat => ">.",
        InfixOp::LtFloat => "<.",
        InfixOp::GteFloat => ">=.",
        InfixOp::LteFloat => "<=.",
        InfixOp::And => "and",
        InfixOp::Or => "or",
    }
}

/// Type signature for a unary operator
pub enum PrefixSig {
    /// Operand and result have same type (must be one of the given types)
    Preserve(&'static [Type]),
    /// Fixed operand type, fixed result type
    Fixed { operand: Type, result: Type },
}

pub fn infix_signature(op: InfixOp) -> InfixSig {
    use Type::*;
    match op {
        // Integer arithmetic -> Integer
        InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div | InfixOp::Mod => InfixSig {
            operand: Integer,
            result: Integer,
        },
        // Float arithmetic -> Float
        InfixOp::AddFloat | InfixOp::SubFloat | InfixOp::MulFloat | InfixOp::DivFloat => InfixSig {
            operand: Float,
            result: Float,
        },
        // Integer comparison -> Boolean
        InfixOp::Eq | InfixOp::NotEq | InfixOp::Gt | InfixOp::Lt | InfixOp::Gte | InfixOp::Lte => {
            InfixSig {
                operand: Integer,
                result: Boolean,
            }
        }
        // Float comparison -> Boolean
        InfixOp::GtFloat | InfixOp::LtFloat | InfixOp::GteFloat | InfixOp::LteFloat => InfixSig {
            operand: Float,
            result: Boolean,
        },
        // Boolean logic -> Boolean
        InfixOp::And | InfixOp::Or => InfixSig {
            operand: Boolean,
            result: Boolean,
        },
    }
}

// Static arrays for Preserve signatures
static NUMERIC_TYPES: &[Type] = &[Type::Integer, Type::Float];

pub fn prefix_signature(op: PrefixOp) -> PrefixSig {
    use Type::*;
    match op {
        PrefixOp::Neg => PrefixSig::Preserve(NUMERIC_TYPES),
        PrefixOp::Not => PrefixSig::Fixed {
            operand: Boolean,
            result: Boolean,
        },
    }
}
