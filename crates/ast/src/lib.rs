mod node;

pub use node::AstNode;

use node::ast_node;
use syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

ast_node!(Root, SyntaxKind::Root);

impl Root {
    pub fn items(&self) -> impl Iterator<Item = Item> {
        self.0.children().filter_map(Item::cast)
    }
}

#[derive(Debug)]
pub enum Item {
    VariableDefinition(VariableDefinition),
    VariableAssignment(VariableAssignment),
    Expression(Expression),
}

impl Item {
    pub fn cast(node: SyntaxNode) -> Option<Item> {
        let result = match node.kind() {
            SyntaxKind::VariableDefinition => Self::VariableDefinition(VariableDefinition(node)),
            SyntaxKind::VariableAssignment => Self::VariableAssignment(VariableAssignment(node)),
            _ => Self::Expression(Expression::cast(node)?),
        };
        Some(result)
    }
}

ast_node!(VariableAssignment, SyntaxKind::VariableAssignment);

impl VariableAssignment {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }

    pub fn value(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

ast_node!(VariableDefinition, SyntaxKind::VariableDefinition);

impl VariableDefinition {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }

    pub fn value(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

#[derive(Debug)]
pub enum Expression {
    Infix(InfixExpression),
    Literal(Literal),
    Parenthesis(ParenthesisExpression),
    Prefix(PrefixExpression),
    VariableReference(VariableReference),
}

impl Expression {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        let result = match node.kind() {
            SyntaxKind::InfixExpression => Self::Infix(InfixExpression(node)),
            SyntaxKind::Literal => Self::Literal(Literal(node)),
            SyntaxKind::ParenthesisExpression => Self::Parenthesis(ParenthesisExpression(node)),
            SyntaxKind::PrefixExpression => Self::Prefix(PrefixExpression(node)),
            SyntaxKind::VariableReference => Self::VariableReference(VariableReference(node)),
            _ => return None,
        };
        Some(result)
    }
}

ast_node!(InfixExpression, SyntaxKind::InfixExpression);

impl InfixExpression {
    pub fn lhs(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    pub fn rhs(&self) -> Option<Expression> {
        self.0.children().filter_map(Expression::cast).nth(1)
    }

    pub fn operation(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| {
                matches!(
                    token.kind(),
                    // Integer arithmetic
                    SyntaxKind::Plus
                        | SyntaxKind::Minus
                        | SyntaxKind::Asterisk
                        | SyntaxKind::Slash
                        | SyntaxKind::Percent
                        // Float arithmetic
                        | SyntaxKind::PlusDot
                        | SyntaxKind::MinusDot
                        | SyntaxKind::AsteriskDot
                        | SyntaxKind::SlashDot
                        // Comparison
                        | SyntaxKind::EqualsEquals
                        | SyntaxKind::NotEquals
                        | SyntaxKind::GreaterThan
                        | SyntaxKind::LessThan
                        | SyntaxKind::GreaterThanOrEqual
                        | SyntaxKind::LessThanOrEqual
                        // Float comparison
                        | SyntaxKind::GreaterThanDot
                        | SyntaxKind::LessThanDot
                        | SyntaxKind::GreaterThanOrEqualDot
                        | SyntaxKind::LessThanOrEqualDot
                        // Boolean
                        | SyntaxKind::AndKeyword
                        | SyntaxKind::OrKeyword,
                )
            })
    }
}

ast_node!(Literal, SyntaxKind::Literal);

impl Literal {
    pub fn parse(&self) -> u64 {
        let token = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Integer)
            .expect("Literal node should contain an Integer token");

        match token.text().parse::<u64>() {
            Ok(value) => value,
            Err(err) => {
                panic!("Failed to parse literal '{}': {}", token.text(), err)
            }
        }
    }
}

ast_node!(ParenthesisExpression, SyntaxKind::ParenthesisExpression);

impl ParenthesisExpression {
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

ast_node!(PrefixExpression, SyntaxKind::PrefixExpression);

impl PrefixExpression {
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    pub fn operation(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| matches!(token.kind(), SyntaxKind::Minus | SyntaxKind::Bang))
    }
}

ast_node!(VariableReference, SyntaxKind::VariableReference);

impl VariableReference {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }
}

ast_node!(TypeAnnotation, SyntaxKind::TypeAnnotation);

impl TypeAnnotation {
    pub fn type_token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_root(input: &str) -> Root {
        let (node, _) = parse::parse(input);
        Root::cast(node).unwrap()
    }

    #[test]
    fn variable_definition_complete() {
        let root = parse_root("x := 42");
        let item = root.items().next().unwrap();
        let Item::VariableDefinition(def) = item else {
            panic!("expected VariableDefinition")
        };
        assert_eq!(def.name().unwrap().text(), "x");
        assert!(def.value().is_some());
    }

    #[test]
    fn variable_definition_missing_value() {
        // Parser recovers: node exists but value() returns None
        let root = parse_root("x :=");
        let item = root.items().next().unwrap();
        let Item::VariableDefinition(def) = item else {
            panic!("expected VariableDefinition")
        };
        assert_eq!(def.name().unwrap().text(), "x");
        assert!(def.value().is_none(), "missing value should return None");
    }

    #[test]
    fn infix_integer_operators() {
        for (input, expected) in [
            ("1 + 2", SyntaxKind::Plus),
            ("1 - 2", SyntaxKind::Minus),
            ("1 * 2", SyntaxKind::Asterisk),
            ("1 / 2", SyntaxKind::Slash),
            ("1 % 2", SyntaxKind::Percent),
        ] {
            let root = parse_root(&format!("x := {input}"));
            let item = root.items().next().unwrap();
            let Item::VariableDefinition(def) = item else {
                panic!("expected VariableDefinition")
            };
            let Expression::Infix(infix) = def.value().unwrap() else {
                panic!("expected InfixExpression for {input}")
            };
            assert_eq!(infix.operation().unwrap().kind(), expected, "{input}");
        }
    }

    #[test]
    fn infix_float_operators() {
        for (input, expected) in [
            ("1.0 +. 2.0", SyntaxKind::PlusDot),
            ("1.0 -. 2.0", SyntaxKind::MinusDot),
            ("1.0 *. 2.0", SyntaxKind::AsteriskDot),
            ("1.0 /. 2.0", SyntaxKind::SlashDot),
        ] {
            let root = parse_root(&format!("x := {input}"));
            let item = root.items().next().unwrap();
            let Item::VariableDefinition(def) = item else {
                panic!("expected VariableDefinition")
            };
            let Expression::Infix(infix) = def.value().unwrap() else {
                panic!("expected InfixExpression for {input}")
            };
            assert_eq!(infix.operation().unwrap().kind(), expected, "{input}");
        }
    }

    #[test]
    fn infix_comparison_operators() {
        for (input, expected) in [
            ("a == b", SyntaxKind::EqualsEquals),
            ("a != b", SyntaxKind::NotEquals),
            ("a > b", SyntaxKind::GreaterThan),
            ("a < b", SyntaxKind::LessThan),
            ("a >= b", SyntaxKind::GreaterThanOrEqual),
            ("a <= b", SyntaxKind::LessThanOrEqual),
        ] {
            let root = parse_root(&format!("x := {input}"));
            let item = root.items().next().unwrap();
            let Item::VariableDefinition(def) = item else {
                panic!("expected VariableDefinition")
            };
            let Expression::Infix(infix) = def.value().unwrap() else {
                panic!("expected InfixExpression for {input}")
            };
            assert_eq!(infix.operation().unwrap().kind(), expected, "{input}");
        }
    }

    #[test]
    fn infix_boolean_operators() {
        for (input, expected) in [
            ("a and b", SyntaxKind::AndKeyword),
            ("a or b", SyntaxKind::OrKeyword),
        ] {
            let root = parse_root(&format!("x := {input}"));
            let item = root.items().next().unwrap();
            let Item::VariableDefinition(def) = item else {
                panic!("expected VariableDefinition")
            };
            let Expression::Infix(infix) = def.value().unwrap() else {
                panic!("expected InfixExpression for {input}")
            };
            assert_eq!(infix.operation().unwrap().kind(), expected, "{input}");
        }
    }

    #[test]
    fn prefix_operators() {
        for (input, expected) in [("-5", SyntaxKind::Minus), ("!true", SyntaxKind::Bang)] {
            let root = parse_root(&format!("x := {input}"));
            let item = root.items().next().unwrap();
            let Item::VariableDefinition(def) = item else {
                panic!("expected VariableDefinition")
            };
            let Expression::Prefix(prefix) = def.value().unwrap() else {
                panic!("expected PrefixExpression for {input}")
            };
            assert_eq!(prefix.operation().unwrap().kind(), expected, "{input}");
        }
    }

    #[test]
    fn infix_missing_rhs() {
        // "1 +" with missing rhs - lhs exists, rhs is None
        let root = parse_root("x := 1 +");
        let item = root.items().next().unwrap();
        let Item::VariableDefinition(def) = item else {
            panic!("expected VariableDefinition")
        };
        let Expression::Infix(infix) = def.value().unwrap() else {
            panic!("expected InfixExpression")
        };
        assert!(infix.lhs().is_some());
        assert!(infix.rhs().is_none(), "missing rhs should return None");
    }

    #[test]
    fn parenthesis_missing_inner() {
        // "()" with missing inner expression
        let root = parse_root("x := ()");
        let item = root.items().next().unwrap();
        let Item::VariableDefinition(def) = item else {
            panic!("expected VariableDefinition")
        };
        let Expression::Parenthesis(paren) = def.value().unwrap() else {
            panic!("expected ParenthesisExpression")
        };
        assert!(
            paren.expression().is_none(),
            "empty parens should return None"
        );
    }

    #[test]
    fn unclosed_paren_still_casts() {
        // "(1 + 2" unclosed - should still produce ParenthesisExpression
        let root = parse_root("x := (1 + 2");
        let item = root.items().next().unwrap();
        let Item::VariableDefinition(def) = item else {
            panic!("expected VariableDefinition")
        };
        let Expression::Parenthesis(paren) = def.value().unwrap() else {
            panic!("expected ParenthesisExpression")
        };
        // Inner expression should exist despite missing close paren
        assert!(paren.expression().is_some());
    }

    #[test]
    fn cast_wrong_kind_returns_none() {
        let root = parse_root("x := 1");
        let syntax = root.syntax().clone();
        // Root node cannot cast to VariableDefinition
        assert!(VariableDefinition::cast(syntax).is_none());
    }
}
