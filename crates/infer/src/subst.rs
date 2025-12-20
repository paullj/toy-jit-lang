use std::collections::HashMap;

use crate::types::{Type, TypeVar};

#[derive(Debug, Clone, Default)]
pub struct Subst {
    map: HashMap<TypeVar, Type>,
}

impl Subst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn single(var: TypeVar, ty: Type) -> Self {
        let mut s = Self::new();
        s.map.insert(var, ty);
        s
    }

    pub fn insert(&mut self, var: TypeVar, ty: Type) {
        self.map.insert(var, ty);
    }

    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => match self.map.get(v) {
                Some(t) => self.apply(t),
                None => ty.clone(),
            },
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(ret)),
            },
            _ => ty.clone(),
        }
    }

    /// Compose two substitutions: self after other
    /// (self.compose(other)).apply(t) == self.apply(other.apply(t))
    pub fn compose(&self, other: &Subst) -> Subst {
        let mut result = Subst::new();

        // Apply self to each binding in other
        for (var, ty) in &other.map {
            result.map.insert(*var, self.apply(ty));
        }

        // Add bindings from self that aren't in other
        for (var, ty) in &self.map {
            result.map.entry(*var).or_insert_with(|| ty.clone());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_ground() {
        let s = Subst::new();
        assert_eq!(s.apply(&Type::Integer), Type::Integer);
    }

    #[test]
    fn test_apply_var() {
        let mut s = Subst::new();
        s.insert(TypeVar(0), Type::Integer);
        assert_eq!(s.apply(&Type::Var(TypeVar(0))), Type::Integer);
        assert_eq!(s.apply(&Type::Var(TypeVar(1))), Type::Var(TypeVar(1)));
    }

    #[test]
    fn test_apply_function() {
        let mut s = Subst::new();
        s.insert(TypeVar(0), Type::Integer);
        s.insert(TypeVar(1), Type::Boolean);

        let fn_ty = Type::Function {
            params: vec![Type::Var(TypeVar(0))],
            ret: Box::new(Type::Var(TypeVar(1))),
        };

        let expected = Type::Function {
            params: vec![Type::Integer],
            ret: Box::new(Type::Boolean),
        };

        assert_eq!(s.apply(&fn_ty), expected);
    }

    #[test]
    fn test_compose() {
        // s1: 'a -> int
        // s2: 'b -> 'a
        // compose(s1, s2): 'b -> int, 'a -> int
        let s1 = Subst::single(TypeVar(0), Type::Integer);
        let s2 = Subst::single(TypeVar(1), Type::Var(TypeVar(0)));

        let composed = s1.compose(&s2);

        assert_eq!(composed.apply(&Type::Var(TypeVar(1))), Type::Integer);
        assert_eq!(composed.apply(&Type::Var(TypeVar(0))), Type::Integer);
    }
}
