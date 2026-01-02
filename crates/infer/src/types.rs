use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub(crate) u32);

impl TypeVar {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ML style: 'a, 'b, ..., 'z, 'aa, 'ab, ...
        let mut n = self.0;
        let mut chars = Vec::new();
        loop {
            chars.push((b'a' + (n % 26) as u8) as char);
            n /= 26;
            if n == 0 {
                break;
            }
            n -= 1;
        }
        chars.reverse();
        write!(f, "'")?;
        for c in chars {
            write!(f, "{}", c)?;
        }
        Ok(())
    }
}

/// Struct ID for identifying struct types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructId(pub u32);

impl fmt::Display for StructId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct#{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Integer,
    Float,
    Boolean,
    String,
    Unit,
    Var(TypeVar),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    List(Box<Type>),
    Tuple(Vec<Type>),
    /// Struct type, identified by StructId
    Struct(StructId),
    Error,
}

impl Type {
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    pub fn is_var(&self) -> bool {
        matches!(self, Type::Var(_))
    }

    pub fn free_vars(&self) -> std::collections::HashSet<TypeVar> {
        let mut vars = std::collections::HashSet::new();
        self.collect_free_vars(&mut vars);
        vars
    }

    fn collect_free_vars(&self, vars: &mut std::collections::HashSet<TypeVar>) {
        match self {
            Type::Var(v) => {
                vars.insert(*v);
            }
            Type::Function { params, ret } => {
                for p in params {
                    p.collect_free_vars(vars);
                }
                ret.collect_free_vars(vars);
            }
            Type::List(elem) => {
                elem.collect_free_vars(vars);
            }
            Type::Tuple(elems) => {
                for elem in elems {
                    elem.collect_free_vars(vars);
                }
            }
            Type::Struct(_)
            | Type::Integer
            | Type::Float
            | Type::Boolean
            | Type::String
            | Type::Unit
            | Type::Error => {}
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Integer => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Boolean => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Unit => write!(f, "unit"),
            Type::Var(v) => write!(f, "{}", v),
            Type::Function { params, ret } => {
                if params.is_empty() {
                    write!(f, "() -> {}", ret)
                } else if params.len() == 1 {
                    let p = &params[0];
                    if matches!(p, Type::Function { .. }) {
                        write!(f, "({}) -> {}", p, ret)
                    } else {
                        write!(f, "{} -> {}", p, ret)
                    }
                } else {
                    write!(f, "(")?;
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ") -> {}", ret)
                }
            }
            Type::List(elem) => write!(f, "list[{}]", elem),
            Type::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            Type::Struct(id) => write!(f, "{}", id),
            Type::Error => write!(f, "<error>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typevar_display() {
        assert_eq!(TypeVar(0).to_string(), "'a");
        assert_eq!(TypeVar(1).to_string(), "'b");
        assert_eq!(TypeVar(25).to_string(), "'z");
        assert_eq!(TypeVar(26).to_string(), "'aa");
        assert_eq!(TypeVar(27).to_string(), "'ab");
    }

    #[test]
    fn test_type_display() {
        assert_eq!(Type::Integer.to_string(), "int");
        assert_eq!(Type::Float.to_string(), "float");
        assert_eq!(
            Type::Function {
                params: vec![Type::Integer],
                ret: Box::new(Type::Boolean)
            }
            .to_string(),
            "int -> bool"
        );
        assert_eq!(
            Type::Function {
                params: vec![Type::Integer, Type::Float],
                ret: Box::new(Type::Boolean)
            }
            .to_string(),
            "(int, float) -> bool"
        );
    }
}
