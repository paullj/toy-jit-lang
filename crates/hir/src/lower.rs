use crate::{
    BlockItem, Definition, ExprIdx, Expression, FunctionParam, HirDiagnostic, Ident, InfixOp, Item,
    Literal, PrefixOp, SymbolKind, SymbolTable,
};
use ast::AstNode;
use la_arena::{Arena, ArenaMap};
use lasso::Rodeo;
use miette::SourceSpan;
use syntax::{SyntaxKind, TextRange};

fn extract_doc_comment(node: &syntax::SyntaxNode) -> Option<String> {
    use syntax::SyntaxElement;

    let mut comments = Vec::new();
    let mut consecutive_newlines = 0;
    let mut found_doc = false;

    // Walk through children (trivia comes first, before actual content)
    for elem in node.children_with_tokens() {
        match elem {
            SyntaxElement::Token(token) => {
                let kind = token.kind();
                match kind {
                    SyntaxKind::DocComment => {
                        // Strip ## prefix and trim whitespace
                        let text = token.text().trim_start_matches('#').trim();
                        comments.push(text.to_string());
                        consecutive_newlines = 0;
                        found_doc = true;
                    }
                    SyntaxKind::NewLine => {
                        consecutive_newlines += 1;
                        // 2+ consecutive newlines = blank line, discard comments
                        if found_doc && consecutive_newlines >= 2 {
                            comments.clear();
                            found_doc = false;
                        }
                    }
                    SyntaxKind::Whitespace | SyntaxKind::Comment => {
                        // Skip whitespace and regular comments
                    }
                    _ => {
                        // Hit non-trivia token (e.g., Identifier), stop
                        break;
                    }
                }
            }
            SyntaxElement::Node(_) => {
                // Hit a child node, stop
                break;
            }
        }
    }

    if comments.is_empty() {
        None
    } else {
        Some(comments.join("\n"))
    }
}

fn to_span(range: TextRange) -> SourceSpan {
    let start: usize = range.start().into();
    let len: usize = range.len().into();
    (start, len).into()
}

#[derive(Debug)]
pub struct LowerResult {
    pub items: Vec<Item>,
    pub expressions: Arena<Expression>,
    pub expr_spans: ArenaMap<ExprIdx, TextRange>,
    pub item_spans: Vec<TextRange>,
    pub symbols: SymbolTable,
    pub diagnostics: Vec<HirDiagnostic>,
    /// Identifier interner - owns all interned identifier strings
    pub interner: Rodeo,
}

impl LowerResult {
    /// Resolve an Ident to its string value
    pub fn resolve(&self, ident: Ident) -> &str {
        self.interner.resolve(&ident.spur())
    }
}

impl Default for LowerResult {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            expressions: Arena::new(),
            expr_spans: ArenaMap::new(),
            item_spans: Vec::new(),
            symbols: SymbolTable::default(),
            diagnostics: Vec::new(),
            interner: Rodeo::default(),
        }
    }
}

struct Ctx {
    expressions: Arena<Expression>,
    expr_spans: ArenaMap<ExprIdx, TextRange>,
    symbols: SymbolTable,
    diagnostics: Vec<HirDiagnostic>,
    interner: Rodeo,
}

impl Ctx {
    fn new() -> Self {
        Self {
            expressions: Arena::new(),
            expr_spans: ArenaMap::new(),
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            interner: Rodeo::default(),
        }
    }

    fn alloc(&mut self, expr: Expression, span: TextRange) -> ExprIdx {
        let idx = self.expressions.alloc(expr);
        self.expr_spans.insert(idx, span);
        idx
    }

    /// Intern an identifier string, returning an Ident
    fn intern(&mut self, s: &str) -> Ident {
        Ident::new(self.interner.get_or_intern(s))
    }

    fn define_symbol(
        &mut self,
        name: &str,
        def_span: TextRange,
        name_span: TextRange,
        doc_comment: Option<String>,
    ) {
        self.symbols.define(
            name.to_string(),
            SymbolKind::Variable,
            def_span,
            name_span,
            doc_comment,
        );
    }

    fn add_reference(&mut self, name: &str, span: TextRange) {
        self.symbols.add_reference(name, span);
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
        symbols: ctx.symbols,
        diagnostics: ctx.diagnostics,
        interner: ctx.interner,
    }
}

fn lower_item(ctx: &mut Ctx, ast: ast::Item) -> Option<Item> {
    match ast {
        ast::Item::FunctionDefinition(fn_def) => {
            let name_token = fn_def.name()?;
            let name_str = name_token.text();
            let def_span = fn_def.syntax().text_range();
            let name_span = name_token.text_range();
            let doc_comment = extract_doc_comment(fn_def.syntax());
            ctx.define_symbol(name_str, def_span, name_span, doc_comment);
            let name = ctx.intern(name_str);

            let params = lower_params(ctx, fn_def.params());
            let return_type = fn_def
                .return_type()
                .and_then(|t| t.type_token())
                .map(|t| ctx.intern(t.text()));

            let body_ast = fn_def.body();
            let body_span = body_ast
                .as_ref()
                .map(|b| b.syntax().text_range())
                .unwrap_or_default();
            let body = body_ast
                .map(|b| lower_block(ctx, b))
                .unwrap_or(Expression::Missing);
            let body_idx = ctx.alloc(body, body_span);

            Some(Item::Definition(Definition::Function {
                name,
                params,
                return_type,
                body: body_idx,
            }))
        }
        ast::Item::VariableDefinition(def) => {
            let name_token = def.name()?;
            let name_str = name_token.text();
            let def_span = def.syntax().text_range();
            let name_span = name_token.text_range();
            let doc_comment = extract_doc_comment(def.syntax());
            ctx.define_symbol(name_str, def_span, name_span, doc_comment);
            let name = ctx.intern(name_str);
            let value = lower_expression(ctx, def.value());
            Some(Item::Definition(Definition::Variable { name, value }))
        }
        ast::Item::VariableAssignment(asgn) => {
            let name_token = asgn.name()?;
            let name_str = name_token.text();
            let name_span = name_token.text_range();
            // Assignment is a reference to existing variable
            ctx.add_reference(name_str, name_span);
            let name = ctx.intern(name_str);
            let value = lower_expression(ctx, asgn.value());
            Some(Item::Assignment { name, value })
        }
        ast::Item::ReturnStatement(ret) => {
            let value = ret.value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            Some(Item::Expression(Expression::Return { value }))
        }
        ast::Item::EchoStatement(echo) => {
            let value = echo.value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            // If no value, use Missing expression
            let value_idx =
                value.unwrap_or_else(|| ctx.alloc(Expression::Missing, Default::default()));
            Some(Item::Expression(Expression::Echo { value: value_idx }))
        }
        ast::Item::Expression(expr) => {
            let value = lower_expression(ctx, Some(expr));
            Some(Item::Expression(value))
        }
    }
}

fn lower_params(ctx: &mut Ctx, params: Option<ast::ParameterList>) -> Vec<FunctionParam> {
    let Some(param_list) = params else {
        return vec![];
    };

    param_list
        .params()
        .filter_map(|param| {
            let name_str = param.name()?.text().to_string();
            let name = ctx.intern(&name_str);
            let ty = param
                .type_annotation()
                .and_then(|t| t.type_token())
                .map(|t| ctx.intern(t.text()));
            let default = param.default_value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            Some(FunctionParam { name, ty, default })
        })
        .collect()
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
            if let Some(name_token) = var.name() {
                let name_str = name_token.text();
                let ref_span = name_token.text_range();
                ctx.add_reference(name_str, ref_span);
                let name = ctx.intern(name_str);
                Expression::VariableRef { name }
            } else {
                Expression::Missing
            }
        }
        ast::Expression::Block(block) => lower_block(ctx, block),
        ast::Expression::If(if_expr) => lower_if(ctx, if_expr),
        ast::Expression::Function(fn_expr) => lower_function_expr(ctx, fn_expr),
        ast::Expression::Call(call) => lower_call(ctx, call),
    }
}

fn lower_function_expr(ctx: &mut Ctx, fn_expr: ast::FunctionExpression) -> Expression {
    let params = lower_params(ctx, fn_expr.params());
    let return_type = fn_expr
        .return_type()
        .and_then(|t| t.type_token())
        .map(|t| ctx.intern(t.text()));

    let body_ast = fn_expr.body();
    let body_span = body_ast
        .as_ref()
        .map(|b| b.syntax().text_range())
        .unwrap_or_default();
    let body = body_ast
        .map(|b| lower_block(ctx, b))
        .unwrap_or(Expression::Missing);
    let body_idx = ctx.alloc(body, body_span);

    Expression::Function {
        params,
        return_type,
        body: body_idx,
        captures: vec![], // Filled later during capture analysis
    }
}

fn lower_call(ctx: &mut Ctx, call: ast::CallExpression) -> Expression {
    let callee_ast = call.callee();
    let callee_span = callee_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let callee_expr = lower_expression(ctx, callee_ast);
    let callee = ctx.alloc(callee_expr, callee_span);

    let args: Vec<ExprIdx> = call
        .args()
        .map(|a| {
            let span = a.syntax().text_range();
            let expr = lower_expression(ctx, Some(a));
            ctx.alloc(expr, span)
        })
        .collect();

    Expression::Call { callee, args }
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

    // Check for division by zero with literal 0
    if matches!(op, InfixOp::Div | InfixOp::DivFloat | InfixOp::Mod) && is_zero_literal(&rhs_ast) {
        ctx.diagnostics.push(HirDiagnostic::DivisionByZero {
            span: to_span(rhs_span),
        });
    }

    let lhs = lower_expression(ctx, lhs_ast);
    let rhs = lower_expression(ctx, rhs_ast);

    Expression::Infix {
        op,
        lhs: ctx.alloc(lhs, lhs_span),
        rhs: ctx.alloc(rhs, rhs_span),
    }
}

fn is_zero_literal(ast: &Option<ast::Expression>) -> bool {
    let Some(ast::Expression::Literal(lit)) = ast else {
        return false;
    };
    matches!(
        lit.value(),
        Some(ast::LiteralValue::Integer(0)) | Some(ast::LiteralValue::Float(0.0))
    )
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

fn lower_block(ctx: &mut Ctx, ast: ast::BlockExpression) -> Expression {
    ctx.symbols.push_scope();

    let mut items = Vec::new();

    for ast_item in ast.items() {
        if let Some(block_item) = lower_block_item(ctx, ast_item) {
            items.push(block_item);
        }
    }

    // Tail is the last item if it's an expression (not definition/assignment)
    // Remove it from items to avoid processing it twice in MIR lowering
    let tail = match items.last() {
        Some(BlockItem::Expression(idx)) => {
            let idx = *idx;
            items.pop();
            Some(idx)
        }
        _ => None,
    };

    // Warn on empty blocks
    if items.is_empty() && tail.is_none() {
        ctx.diagnostics.push(HirDiagnostic::EmptyBlock {
            span: to_span(ast.syntax().text_range()),
        });
    }

    ctx.symbols.pop_scope();

    Expression::Block { items, tail }
}

fn lower_if(ctx: &mut Ctx, ast: ast::IfExpression) -> Expression {
    let cond_ast = ast.condition();
    let cond_span = cond_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let cond_expr = lower_expression(ctx, cond_ast);
    let condition = ctx.alloc(cond_expr, cond_span);

    let then_ast = ast.then_branch();
    let then_span = then_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let then_expr = then_ast
        .map(|b| lower_block(ctx, b))
        .unwrap_or(Expression::Missing);
    let then_branch = ctx.alloc(then_expr, then_span);

    let else_branch = ast.else_branch().map(|eb| {
        let span = eb.syntax().text_range();
        let expr = match eb {
            ast::ElseBranch::Block(b) => lower_block(ctx, b),
            ast::ElseBranch::ElseIf(if_e) => lower_if(ctx, if_e),
        };
        ctx.alloc(expr, span)
    });

    Expression::If {
        condition,
        then_branch,
        else_branch,
    }
}

fn lower_block_item(ctx: &mut Ctx, ast: ast::Item) -> Option<BlockItem> {
    match ast {
        ast::Item::FunctionDefinition(fn_def) => {
            // Functions inside blocks are treated as local definitions
            let name_token = fn_def.name()?;
            let name_str = name_token.text();
            let def_span = fn_def.syntax().text_range();
            let name_span = name_token.text_range();
            let doc_comment = extract_doc_comment(fn_def.syntax());
            ctx.define_symbol(name_str, def_span, name_span, doc_comment);
            let name = ctx.intern(name_str);

            let params = lower_params(ctx, fn_def.params());
            let return_type = fn_def
                .return_type()
                .and_then(|t| t.type_token())
                .map(|t| ctx.intern(t.text()));

            let body_ast = fn_def.body();
            let body_span = body_ast
                .as_ref()
                .map(|b| b.syntax().text_range())
                .unwrap_or_default();
            let body = body_ast
                .map(|b| lower_block(ctx, b))
                .unwrap_or(Expression::Missing);
            let body_idx = ctx.alloc(body, body_span);

            // Lower as function expression assigned to the name
            let func_expr = Expression::Function {
                params,
                return_type,
                body: body_idx,
                captures: vec![],
            };
            let func_idx = ctx.alloc(func_expr, def_span);
            Some(BlockItem::Definition {
                name,
                value: func_idx,
            })
        }
        ast::Item::VariableDefinition(def) => {
            let name_token = def.name()?;
            let name_str = name_token.text();
            let def_span = def.syntax().text_range();
            let name_span = name_token.text_range();
            let doc_comment = extract_doc_comment(def.syntax());
            ctx.define_symbol(name_str, def_span, name_span, doc_comment);
            let name = ctx.intern(name_str);
            let value = lower_expression(ctx, def.value());
            let value_span = def
                .value()
                .map(|e| e.syntax().text_range())
                .unwrap_or_default();
            let value_idx = ctx.alloc(value, value_span);
            Some(BlockItem::Definition {
                name,
                value: value_idx,
            })
        }
        ast::Item::VariableAssignment(asgn) => {
            let name_token = asgn.name()?;
            let name_str = name_token.text();
            let name_span = name_token.text_range();
            ctx.add_reference(name_str, name_span);
            let name = ctx.intern(name_str);
            let value = lower_expression(ctx, asgn.value());
            let value_span = asgn
                .value()
                .map(|e| e.syntax().text_range())
                .unwrap_or_default();
            let value_idx = ctx.alloc(value, value_span);
            Some(BlockItem::Assignment {
                name,
                value: value_idx,
            })
        }
        ast::Item::ReturnStatement(ret) => {
            let value = ret.value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            Some(BlockItem::Return { value })
        }
        ast::Item::EchoStatement(echo) => {
            let value = echo.value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            // If no value, use Missing expression
            let value_idx =
                value.unwrap_or_else(|| ctx.alloc(Expression::Missing, Default::default()));
            Some(BlockItem::Echo { value: value_idx })
        }
        ast::Item::Expression(expr) => {
            let span = expr.syntax().text_range();
            let value = lower_expression(ctx, Some(expr));
            let idx = ctx.alloc(value, span);
            Some(BlockItem::Expression(idx))
        }
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
        assert_eq!(result.resolve(*name), "x");
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
        assert_eq!(result.resolve(*name), "x");
        assert!(matches!(value, Expression::Literal(Literal::Integer(10))));
    }

    #[test]
    fn lower_variable_reference() {
        let result = lower_src("x := y");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        let Expression::VariableRef { name } = value else {
            panic!("expected variable ref");
        };
        assert_eq!(result.resolve(*name), "y");
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
                assert!(result.resolve(*name).is_empty());
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

        let Item::Definition(Definition::Variable { name, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert_eq!(result.resolve(*name), "x");

        let Item::Definition(Definition::Variable { name, .. }) = &result.items[1] else {
            panic!("expected variable definition");
        };
        assert_eq!(result.resolve(*name), "y");

        let Item::Assignment { name, .. } = &result.items[2] else {
            panic!("expected assignment");
        };
        assert_eq!(result.resolve(*name), "z");
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

    // === Doc comments ===

    #[test]
    fn lower_doc_comment() {
        let result = lower_src("## This is a doc comment\nx := 42");
        let symbol = result.symbols.get("x").unwrap();
        assert_eq!(symbol.doc_comment.as_deref(), Some("This is a doc comment"));
    }

    #[test]
    fn lower_multiline_doc_comment() {
        let result = lower_src("## Line 1\n## Line 2\n## Line 3\nx := 42");
        let symbol = result.symbols.get("x").unwrap();
        assert_eq!(
            symbol.doc_comment.as_deref(),
            Some("Line 1\nLine 2\nLine 3")
        );
    }

    #[test]
    fn lower_no_doc_comment() {
        let result = lower_src("x := 42");
        let symbol = result.symbols.get("x").unwrap();
        assert_eq!(symbol.doc_comment, None);
    }

    #[test]
    fn lower_doc_comment_with_blank_line_breaks() {
        let result = lower_src("## Comment\n\n\nx := 42");
        let symbol = result.symbols.get("x").unwrap();
        // More than one blank line (2+) should break the doc comment
        assert_eq!(symbol.doc_comment, None);
    }

    #[test]
    fn lower_regular_comment_not_doc() {
        // Regular # comments should not become doc comments
        let result = lower_src("# This is a regular comment\nx := 42");
        let symbol = result.symbols.get("x").unwrap();
        assert_eq!(symbol.doc_comment, None);
    }

    // === Functions ===

    #[test]
    fn lower_function_definition() {
        let result = lower_src("fn add(a, b) { a + b }");
        assert_eq!(result.items.len(), 1);
        let Item::Definition(Definition::Function {
            name, params, body, ..
        }) = &result.items[0]
        else {
            panic!("expected function definition");
        };
        assert_eq!(result.resolve(*name), "add");
        assert_eq!(params.len(), 2);
        assert_eq!(result.resolve(params[0].name), "a");
        assert_eq!(result.resolve(params[1].name), "b");

        let body_expr = &result.expressions[*body];
        assert!(matches!(body_expr, Expression::Block { .. }));
    }

    #[test]
    fn lower_function_with_return_type() {
        let result = lower_src("fn double(x): int { x * 2 }");
        let Item::Definition(Definition::Function {
            name, return_type, ..
        }) = &result.items[0]
        else {
            panic!("expected function definition");
        };
        assert_eq!(result.resolve(*name), "double");
        assert_eq!(result.resolve(return_type.unwrap()), "int");
    }

    #[test]
    fn lower_lambda_expression() {
        let result = lower_src("inc := fn(x) { x + 1 }");
        let Item::Definition(Definition::Variable { name, value }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        assert_eq!(result.resolve(*name), "inc");
        assert!(matches!(value, Expression::Function { .. }));
    }

    #[test]
    fn lower_call_expression() {
        let result = lower_src("x := add(1, 2)");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        let Expression::Call { callee, args } = value else {
            panic!("expected call expression");
        };
        let callee_expr = &result.expressions[*callee];
        let Expression::VariableRef { name } = callee_expr else {
            panic!("expected variable ref");
        };
        assert_eq!(result.resolve(*name), "add");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn lower_return_statement() {
        let result = lower_src("fn foo() { return 42 }");
        let Item::Definition(Definition::Function { body, .. }) = &result.items[0] else {
            panic!("expected function definition");
        };
        let Expression::Block { items, .. } = &result.expressions[*body] else {
            panic!("expected block");
        };
        assert_eq!(items.len(), 1);
        assert!(matches!(&items[0], BlockItem::Return { value: Some(_) }));
    }

    #[test]
    fn lower_return_without_value() {
        let result = lower_src("fn foo() { return }");
        let Item::Definition(Definition::Function { body, .. }) = &result.items[0] else {
            panic!("expected function definition");
        };
        let Expression::Block { items, .. } = &result.expressions[*body] else {
            panic!("expected block");
        };
        assert!(matches!(&items[0], BlockItem::Return { value: None }));
    }

    #[test]
    fn lower_function_with_default_param() {
        let result = lower_src("fn greet(name = \"World\") { name }");
        let Item::Definition(Definition::Function { params, .. }) = &result.items[0] else {
            panic!("expected function definition");
        };
        assert_eq!(params.len(), 1);
        assert_eq!(result.resolve(params[0].name), "name");
        assert!(params[0].default.is_some());
    }
}
