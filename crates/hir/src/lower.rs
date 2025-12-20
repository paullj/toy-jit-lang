use crate::{Definition, ExprIdx, Expression, InfixOp, Item, Literal, PrefixOp};
use la_arena::{Arena, ArenaMap};
use syntax::{SyntaxKind, TextRange};

#[derive(Debug, Default)]
pub struct LowerResult {
    pub items: Vec<Item>,
    pub expressions: Arena<Expression>,
    pub expr_spans: ArenaMap<ExprIdx, TextRange>,
    pub item_spans: Vec<TextRange>,
}

struct Ctx {
    expressions: Arena<Expression>,
    expr_spans: ArenaMap<ExprIdx, TextRange>,
}

impl Ctx {
    fn new() -> Self {
        Self {
            expressions: Arena::new(),
            expr_spans: ArenaMap::new(),
        }
    }

    fn alloc(&mut self, expr: Expression, span: TextRange) -> ExprIdx {
        let idx = self.expressions.alloc(expr);
        self.expr_spans.insert(idx, span);
        idx
    }
}

pub fn lower(root: ast::Root) -> LowerResult {
    let mut ctx = Ctx::new();
    let mut items = Vec::new();
    let mut item_spans = Vec::new();

    for item in root.items() {
        let span = item.syntax().text_range();
        if let Some(lowered) = lower_item(&mut ctx, item) {
            items.push(lowered);
            item_spans.push(span);
        }
    }

    LowerResult {
        items,
        expressions: ctx.expressions,
        expr_spans: ctx.expr_spans,
        item_spans,
    }
}

fn lower_item(ctx: &mut Ctx, ast: ast::Item) -> Option<Item> {
    match ast {
        ast::Item::VariableDefinition(def) => {
            let name = def.name()?.text().to_string();
            let value = lower_expression(ctx, def.value());
            Some(Item::Definition(Definition::Variable { name, value }))
        }
        ast::Item::VariableAssignment(asgn) => {
            let name = asgn.name()?.text().to_string();
            let value = lower_expression(ctx, asgn.value());
            Some(Item::Assignment { name, value })
        }
        ast::Item::Expression(expr) => {
            let value = lower_expression(ctx, Some(expr));
            Some(Item::Expression(value))
        }
    }
}

fn lower_expression(ctx: &mut Ctx, ast: Option<ast::Expression>) -> Expression {
    let Some(ast) = ast else {
        return Expression::Missing;
    };

    match ast {
        ast::Expression::Infix(infix) => lower_infix(ctx, infix),
        ast::Expression::Literal(lit) => Expression::Literal(lower_literal(lit)),
        ast::Expression::Parenthesis(paren) => lower_expression(ctx, paren.expression()),
        ast::Expression::Prefix(prefix) => lower_prefix(ctx, prefix),
        ast::Expression::VariableReference(var) => {
            if let Some(name) = var.name() {
                Expression::VariableRef {
                    name: name.text().to_string(),
                }
            } else {
                Expression::Missing
            }
        }
    }
}

fn lower_infix(ctx: &mut Ctx, ast: ast::InfixExpression) -> Expression {
    let Some(op_token) = ast.operation() else {
        return Expression::Missing;
    };

    let op = match op_token.kind() {
        // Int arithmetic
        SyntaxKind::Plus => InfixOp::Add,
        SyntaxKind::Minus => InfixOp::Sub,
        SyntaxKind::Asterisk => InfixOp::Mul,
        SyntaxKind::Slash => InfixOp::Div,
        SyntaxKind::Percent => InfixOp::Mod,
        // Float arithmetic
        SyntaxKind::PlusDot => InfixOp::AddFloat,
        SyntaxKind::MinusDot => InfixOp::SubFloat,
        SyntaxKind::AsteriskDot => InfixOp::MulFloat,
        SyntaxKind::SlashDot => InfixOp::DivFloat,
        // Comparison (int)
        SyntaxKind::EqualsEquals => InfixOp::Eq,
        SyntaxKind::NotEquals => InfixOp::NotEq,
        SyntaxKind::GreaterThan => InfixOp::Gt,
        SyntaxKind::LessThan => InfixOp::Lt,
        SyntaxKind::GreaterThanOrEqual => InfixOp::Gte,
        SyntaxKind::LessThanOrEqual => InfixOp::Lte,
        // Comparison (float)
        SyntaxKind::GreaterThanDot => InfixOp::GtFloat,
        SyntaxKind::LessThanDot => InfixOp::LtFloat,
        SyntaxKind::GreaterThanOrEqualDot => InfixOp::GteFloat,
        SyntaxKind::LessThanOrEqualDot => InfixOp::LteFloat,
        // Boolean
        SyntaxKind::AndKeyword => InfixOp::And,
        SyntaxKind::OrKeyword => InfixOp::Or,
        _ => return Expression::Missing,
    };

    let lhs_ast = ast.lhs();
    let rhs_ast = ast.rhs();
    let lhs_span = lhs_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let rhs_span = rhs_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();

    let lhs = lower_expression(ctx, lhs_ast);
    let rhs = lower_expression(ctx, rhs_ast);

    Expression::Infix {
        op,
        lhs: ctx.alloc(lhs, lhs_span),
        rhs: ctx.alloc(rhs, rhs_span),
    }
}

fn lower_prefix(ctx: &mut Ctx, ast: ast::PrefixExpression) -> Expression {
    let Some(op_token) = ast.operation() else {
        return Expression::Missing;
    };

    let op = match op_token.kind() {
        SyntaxKind::Minus => PrefixOp::Neg,
        SyntaxKind::Bang => PrefixOp::Not,
        _ => return Expression::Missing,
    };

    let expr_ast = ast.expression();
    let expr_span = expr_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let expr = lower_expression(ctx, expr_ast);

    Expression::Prefix {
        op,
        expr: ctx.alloc(expr, expr_span),
    }
}

fn lower_literal(ast: ast::Literal) -> Literal {
    match ast.value() {
        Some(ast::LiteralValue::Integer(n)) => Literal::Integer(n),
        Some(ast::LiteralValue::Float(n)) => Literal::Float(n),
        Some(ast::LiteralValue::Boolean(b)) => Literal::Boolean(b),
        Some(ast::LiteralValue::String(s)) => Literal::String(s),
        None => Literal::Integer(0), // fallback for malformed literals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Definition, Expression, InfixOp, Item, Literal, PrefixOp};
    use ast::AstNode;

    fn lower_src(src: &str) -> LowerResult {
        let (node, _errors) = parse::parse(src);
        let root = ast::Root::cast(node).unwrap();
        lower(root)
    }

    // === Basic lowering ===

    #[test]
    fn lower_integer_literal() {
        let result = lower_src("x := 42");
        assert_eq!(result.items.len(), 1);
        let Item::Definition(Definition::Variable { name, value }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert_eq!(name, "x");
        assert!(matches!(value, Expression::Literal(Literal::Integer(42))));
    }

    #[test]
    fn lower_float_literal() {
        let result = lower_src("x := 2.5");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        let Expression::Literal(Literal::Float(f)) = value else {
            panic!("expected float literal");
        };
        assert!((f - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn lower_boolean_literals() {
        let result = lower_src("x := true\ny := false");
        assert_eq!(result.items.len(), 2);

        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Literal(Literal::Boolean(true))));

        let Item::Definition(Definition::Variable { value, .. }) = &result.items[1] else {
            panic!("expected variable definition");
        };
        assert!(matches!(
            value,
            Expression::Literal(Literal::Boolean(false))
        ));
    }

    #[test]
    fn lower_string_literal() {
        let result = lower_src(r#"x := "hello""#);
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Literal(Literal::String(s)) if s == "hello"));
    }

    #[test]
    fn lower_string_with_escapes() {
        let result = lower_src(r#"x := "hello\nworld""#);
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Literal(Literal::String(s)) if s == "hello\nworld"));
    }

    #[test]
    fn lower_variable_assignment() {
        let result = lower_src("x = 10");
        assert_eq!(result.items.len(), 1);
        let Item::Assignment { name, value } = &result.items[0] else {
            panic!("expected assignment");
        };
        assert_eq!(name, "x");
        assert!(matches!(value, Expression::Literal(Literal::Integer(10))));
    }

    #[test]
    fn lower_variable_reference() {
        let result = lower_src("x := y");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::VariableRef { name } if name == "y"));
    }

    // === Operators ===

    #[test]
    fn lower_infix_int_arithmetic() {
        for (src, expected_op) in [
            ("x := 1 + 2", InfixOp::Add),
            ("x := 1 - 2", InfixOp::Sub),
            ("x := 1 * 2", InfixOp::Mul),
            ("x := 1 / 2", InfixOp::Div),
            ("x := 1 % 2", InfixOp::Mod),
        ] {
            let result = lower_src(src);
            let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
                panic!("expected variable definition for {src}");
            };
            let Expression::Infix { op, .. } = value else {
                panic!("expected infix expression for {src}");
            };
            assert_eq!(*op, expected_op, "failed for {src}");
        }
    }

    #[test]
    fn lower_infix_float_arithmetic() {
        for (src, expected_op) in [
            ("x := 1.0 +. 2.0", InfixOp::AddFloat),
            ("x := 1.0 -. 2.0", InfixOp::SubFloat),
            ("x := 1.0 *. 2.0", InfixOp::MulFloat),
            ("x := 1.0 /. 2.0", InfixOp::DivFloat),
        ] {
            let result = lower_src(src);
            let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
                panic!("expected variable definition for {src}");
            };
            let Expression::Infix { op, .. } = value else {
                panic!("expected infix expression for {src}");
            };
            assert_eq!(*op, expected_op, "failed for {src}");
        }
    }

    #[test]
    fn lower_infix_comparison() {
        for (src, expected_op) in [
            ("x := a == b", InfixOp::Eq),
            ("x := a != b", InfixOp::NotEq),
            ("x := a > b", InfixOp::Gt),
            ("x := a < b", InfixOp::Lt),
            ("x := a >= b", InfixOp::Gte),
            ("x := a <= b", InfixOp::Lte),
        ] {
            let result = lower_src(src);
            let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
                panic!("expected variable definition for {src}");
            };
            let Expression::Infix { op, .. } = value else {
                panic!("expected infix expression for {src}");
            };
            assert_eq!(*op, expected_op, "failed for {src}");
        }
    }

    #[test]
    fn lower_infix_boolean() {
        for (src, expected_op) in [("x := a and b", InfixOp::And), ("x := a or b", InfixOp::Or)] {
            let result = lower_src(src);
            let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
                panic!("expected variable definition for {src}");
            };
            let Expression::Infix { op, .. } = value else {
                panic!("expected infix expression for {src}");
            };
            assert_eq!(*op, expected_op, "failed for {src}");
        }
    }

    #[test]
    fn lower_prefix_operators() {
        for (src, expected_op) in [("x := -5", PrefixOp::Neg), ("x := !true", PrefixOp::Not)] {
            let result = lower_src(src);
            let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
                panic!("expected variable definition for {src}");
            };
            let Expression::Prefix { op, .. } = value else {
                panic!("expected prefix expression for {src}");
            };
            assert_eq!(*op, expected_op, "failed for {src}");
        }
    }

    // === Malformed input ===

    #[test]
    fn malformed_missing_value() {
        let result = lower_src("x :=");
        assert_eq!(result.items.len(), 1);
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Missing));
    }

    #[test]
    fn malformed_missing_rhs() {
        let result = lower_src("x := 1 +");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        let Expression::Infix { rhs, .. } = value else {
            panic!("expected infix expression");
        };
        let rhs_expr = &result.expressions[*rhs];
        assert!(matches!(rhs_expr, Expression::Missing));
    }

    #[test]
    fn malformed_missing_lhs() {
        let result = lower_src("x := + 1");
        // Parser treats this as prefix +, which we don't support, so it becomes Missing
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        // Prefix + is not a valid operator, should result in Missing
        assert!(matches!(
            value,
            Expression::Missing | Expression::Prefix { .. }
        ));
    }

    #[test]
    fn malformed_empty_parens() {
        let result = lower_src("x := ()");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Missing));
    }

    #[test]
    fn malformed_unclosed_paren() {
        let result = lower_src("x := (1 + 2");
        // Parser recovers, should still lower the inner expression
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(
            value,
            Expression::Infix {
                op: InfixOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn malformed_missing_name_in_definition() {
        let result = lower_src(":= 42");
        // Parser may recover in various ways - either empty or expression
        // The key is that we don't panic and produce *something* reasonable
        for item in &result.items {
            // Should not produce a valid variable definition with a name
            if let Item::Definition(Definition::Variable { name, .. }) = item {
                assert!(name.is_empty());
            }
        }
    }

    #[test]
    fn malformed_missing_name_in_assignment() {
        let result = lower_src("= 42");
        // Parser may not recognize this as assignment at all
        assert!(result.items.is_empty() || matches!(&result.items[0], Item::Expression(_)));
    }

    #[test]
    fn malformed_double_operator() {
        let result = lower_src("x := 1 + + 2");
        // Second + is treated as prefix, which is invalid
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        // Should still produce an infix with potentially malformed rhs
        assert!(matches!(
            value,
            Expression::Infix { .. } | Expression::Missing
        ));
    }

    #[test]
    fn malformed_only_operator() {
        let result = lower_src("x := +");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(
            value,
            Expression::Missing | Expression::Prefix { .. }
        ));
    }

    // === Integer literal formats ===

    #[test]
    fn lower_binary_integer() {
        let result = lower_src("x := 0b1010");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Literal(Literal::Integer(10))));
    }

    #[test]
    fn lower_octal_integer() {
        let result = lower_src("x := 0o17");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Literal(Literal::Integer(15))));
    }

    #[test]
    fn lower_hex_integer() {
        let result = lower_src("x := 0xFF");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(value, Expression::Literal(Literal::Integer(255))));
    }

    #[test]
    fn lower_integer_with_underscores() {
        let result = lower_src("x := 1_000_000");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert!(matches!(
            value,
            Expression::Literal(Literal::Integer(1_000_000))
        ));
    }

    // === Complex expressions ===

    #[test]
    fn lower_nested_expression() {
        let result = lower_src("x := (1 + 2) * 3");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        let Expression::Infix { op, lhs, rhs } = value else {
            panic!("expected infix expression");
        };
        assert_eq!(*op, InfixOp::Mul);

        let lhs_expr = &result.expressions[*lhs];
        assert!(matches!(
            lhs_expr,
            Expression::Infix {
                op: InfixOp::Add,
                ..
            }
        ));

        let rhs_expr = &result.expressions[*rhs];
        assert!(matches!(rhs_expr, Expression::Literal(Literal::Integer(3))));
    }

    #[test]
    fn lower_chained_operators() {
        let result = lower_src("x := 1 + 2 + 3");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        // Should be left-associative: (1 + 2) + 3
        let Expression::Infix { op, lhs, rhs } = value else {
            panic!("expected infix expression");
        };
        assert_eq!(*op, InfixOp::Add);

        let rhs_expr = &result.expressions[*rhs];
        assert!(matches!(rhs_expr, Expression::Literal(Literal::Integer(3))));

        let lhs_expr = &result.expressions[*lhs];
        assert!(matches!(
            lhs_expr,
            Expression::Infix {
                op: InfixOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn lower_multiple_items() {
        let result = lower_src("x := 1\ny := 2\nz = 3");
        assert_eq!(result.items.len(), 3);
        assert!(
            matches!(&result.items[0], Item::Definition(Definition::Variable { name, .. }) if name == "x")
        );
        assert!(
            matches!(&result.items[1], Item::Definition(Definition::Variable { name, .. }) if name == "y")
        );
        assert!(matches!(&result.items[2], Item::Assignment { name, .. } if name == "z"));
    }

    #[test]
    fn lower_expression_statement() {
        let result = lower_src("42");
        assert_eq!(result.items.len(), 1);
        let Item::Expression(expr) = &result.items[0] else {
            panic!("expected expression item");
        };
        assert!(matches!(expr, Expression::Literal(Literal::Integer(42))));
    }
}
