use crate::{
    BlockItem, Definition, ExprIdx, Expression, FunctionParam, HirError, Ident, InfixOp, Item,
    Literal, ModuleId, PrefixOp, ResolvedIdent, SymbolKind, SymbolTable,
};
use la_arena::{Arena, ArenaMap};
use lasso::ThreadedRodeo;
use miette::SourceSpan;
use std::collections::HashMap;
use toy_ast::AstNode;
use toy_cst::{SyntaxKind, TextRange};

fn extract_doc_comment(node: &toy_cst::ResolvedNode) -> Option<String> {
    let mut comments = Vec::new();
    let mut consecutive_newlines = 0;
    let mut found_doc = false;

    // Walk through children (trivia comes first, before actual content)
    for elem in node.children_with_tokens() {
        if let Some(token) = elem.as_token() {
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
        } else {
            // Hit a child node, stop
            break;
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
    pub diagnostics: Vec<HirError>,
    /// Identifier interner - owns all interned identifier strings
    pub interner: ThreadedRodeo,
}

impl LowerResult {
    /// Resolve an Ident to its string value
    pub fn resolve(&self, ident: Ident) -> &str {
        self.interner.resolve(&ident.spur())
    }

    /// Resolve a ResolvedIdent to its string value
    pub fn resolve_ident(&self, ident: &ResolvedIdent) -> &str {
        match ident {
            ResolvedIdent::Local(id) | ResolvedIdent::External { name: id, .. } => {
                self.interner.resolve(&id.spur())
            }
        }
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
            interner: ThreadedRodeo::default(),
        }
    }
}

/// Result of lowering a single AST item (for incremental compilation).
///
/// Unlike `LowerResult`, this doesn't own the interner - it borrows from the
/// shared compiler database interner.
#[derive(Debug, PartialEq)]
pub struct LowerItemResult {
    pub item: Item,
    pub expressions: Arena<Expression>,
    pub expr_spans: ArenaMap<ExprIdx, TextRange>,
    pub item_span: TextRange,
    pub symbols: SymbolTable,
    pub diagnostics: Vec<HirError>,
}

/// Tracks loop context for break/continue validation
#[derive(Clone)]
struct LoopContext {
    label: Option<Ident>,
}

/// Resolution data for HIR lowering
#[derive(Debug)]
pub struct ResolutionData {
    /// Current module ID
    pub module_id: ModuleId,
    /// Symbol tables from all modules (module_id -> symbols)
    pub symbol_tables: HashMap<ModuleId, toy_resolve::SymbolTable>,
    /// Module paths for resolving imports
    pub module_paths: HashMap<ModuleId, toy_resolve::ModulePath>,
}

/// Context for HIR lowering.
///
/// Borrows the interner to support both:
/// - Non-incremental path: `lower()` creates and owns a `ThreadedRodeo`
/// - Incremental path: `lower_single_item()` borrows from compiler database
struct Ctx<'i> {
    expressions: Arena<Expression>,
    expr_spans: ArenaMap<ExprIdx, TextRange>,
    symbols: SymbolTable,
    diagnostics: Vec<HirError>,
    interner: &'i ThreadedRodeo,
    loop_stack: Vec<LoopContext>,
    /// Optional resolution data for cross-module references
    resolution: Option<ResolutionData>,
}

impl<'i> Ctx<'i> {
    fn new(interner: &'i ThreadedRodeo) -> Self {
        Self {
            expressions: Arena::new(),
            expr_spans: ArenaMap::new(),
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            interner,
            loop_stack: Vec::new(),
            resolution: None,
        }
    }

    fn with_resolution(interner: &'i ThreadedRodeo, resolution: ResolutionData) -> Self {
        Self {
            expressions: Arena::new(),
            expr_spans: ArenaMap::new(),
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            interner,
            loop_stack: Vec::new(),
            resolution: Some(resolution),
        }
    }

    fn push_loop(&mut self, label: Option<Ident>) {
        self.loop_stack.push(LoopContext { label });
    }

    fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    fn in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }

    fn find_loop(&self, label: Option<Ident>) -> bool {
        match label {
            None => self.in_loop(),
            Some(target) => self.loop_stack.iter().any(|ctx| ctx.label == Some(target)),
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

/// Lower an AST root to HIR (non-incremental path).
///
/// Creates and owns its own interner. For incremental compilation,
/// use `lower_single_item` instead.
pub fn lower(root: toy_ast::Root) -> LowerResult {
    let interner = ThreadedRodeo::default();
    let mut ctx = Ctx::new(&interner);
    let mut items = Vec::new();
    let mut item_spans = Vec::new();

    for item in root.items() {
        let span = item.syntax().text_range();
        if let Some(lowered) = lower_ast_item(&mut ctx, item) {
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
        interner,
    }
}

/// Lower an AST root to HIR with module resolution data.
///
/// This version uses pre-resolved symbol tables to properly handle cross-module references.
/// Creates and owns its own interner.
pub fn lower_with_resolution(
    root: toy_ast::Root,
    module_id: ModuleId,
    symbol_tables: HashMap<ModuleId, toy_resolve::SymbolTable>,
    module_paths: HashMap<ModuleId, toy_resolve::ModulePath>,
) -> LowerResult {
    let interner = ThreadedRodeo::default();
    let resolution = ResolutionData {
        module_id,
        symbol_tables,
        module_paths,
    };
    let mut ctx = Ctx::with_resolution(&interner, resolution);
    let mut items = Vec::new();
    let mut item_spans = Vec::new();

    for item in root.items() {
        let span = item.syntax().text_range();
        if let Some(lowered) = lower_ast_item(&mut ctx, item) {
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
        interner,
    }
}

/// Lower a single AST item to HIR (incremental compilation path).
///
/// Takes a borrowed interner from the compiler database.
pub fn lower_single_item(
    interner: &ThreadedRodeo,
    ast_item: toy_ast::Item,
) -> Option<LowerItemResult> {
    let mut ctx = Ctx::new(interner);
    let span = ast_item.syntax().text_range();

    let item = lower_ast_item(&mut ctx, ast_item)?;

    Some(LowerItemResult {
        item,
        expressions: ctx.expressions,
        expr_spans: ctx.expr_spans,
        item_span: span,
        symbols: ctx.symbols,
        diagnostics: ctx.diagnostics,
    })
}

/// Lower a single AST item to HIR with resolution (incremental compilation path).
///
/// Takes a borrowed interner from the compiler database and resolution data.
pub fn lower_single_item_with_resolution(
    interner: &ThreadedRodeo,
    ast_item: toy_ast::Item,
    module_id: ModuleId,
    symbol_tables: HashMap<ModuleId, toy_resolve::SymbolTable>,
    module_paths: HashMap<ModuleId, toy_resolve::ModulePath>,
) -> Option<LowerItemResult> {
    let resolution = ResolutionData {
        module_id,
        symbol_tables,
        module_paths,
    };
    let mut ctx = Ctx::with_resolution(interner, resolution);
    let span = ast_item.syntax().text_range();

    let item = lower_ast_item(&mut ctx, ast_item)?;

    Some(LowerItemResult {
        item,
        expressions: ctx.expressions,
        expr_spans: ctx.expr_spans,
        item_span: span,
        symbols: ctx.symbols,
        diagnostics: ctx.diagnostics,
    })
}

fn lower_ast_item(ctx: &mut Ctx, ast: toy_ast::Item) -> Option<Item> {
    match ast {
        toy_ast::Item::FunctionDefinition(fn_def) => {
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
        toy_ast::Item::VariableDefinition(def) => {
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
        toy_ast::Item::VariableAssignment(asgn) => {
            let name_token = asgn.name()?;
            let name_str = name_token.text();
            let name_span = name_token.text_range();
            // Assignment is a reference to existing variable
            ctx.add_reference(name_str, name_span);
            let name = ctx.intern(name_str);
            let value = lower_expression(ctx, asgn.value());
            Some(Item::Assignment { name, value })
        }
        toy_ast::Item::IndexAssignment(idx_asgn) => {
            let target = idx_asgn.target()?;
            let value_expr = lower_expression(ctx, idx_asgn.value());

            match target {
                toy_ast::Expression::Index(idx_expr) => {
                    let collection = idx_expr.collection().map(|e| {
                        let span = e.syntax().text_range();
                        let expr = lower_expression(ctx, Some(e));
                        ctx.alloc(expr, span)
                    })?;
                    let index = idx_expr.index().map(|e| {
                        let span = e.syntax().text_range();
                        let expr = lower_expression(ctx, Some(e));
                        ctx.alloc(expr, span)
                    })?;
                    Some(Item::IndexAssignment {
                        collection,
                        index,
                        value: value_expr,
                    })
                }
                _ => {
                    // Slice assignment not yet supported
                    Some(Item::Expression(Expression::Missing))
                }
            }
        }
        toy_ast::Item::ReturnStatement(ret) => {
            let value = ret.value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            Some(Item::Expression(Expression::Return { value }))
        }
        toy_ast::Item::EchoStatement(echo) => {
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
        toy_ast::Item::BreakStatement(brk) => {
            let span = brk.syntax().text_range();
            let label = brk.label().map(|t| ctx.intern(t.text()));

            if !ctx.in_loop() {
                ctx.diagnostics.push(HirError::BreakOutsideLoop {
                    span: to_span(span),
                });
            } else if let Some(lbl) = label
                && !ctx.find_loop(Some(lbl))
            {
                ctx.diagnostics.push(HirError::UnknownLoopLabel {
                    label: ctx.interner.resolve(&lbl.spur()).to_string(),
                    span: to_span(span),
                });
            }

            Some(Item::Expression(Expression::Break { label }))
        }
        toy_ast::Item::ContinueStatement(cont) => {
            let span = cont.syntax().text_range();
            let label = cont.label().map(|t| ctx.intern(t.text()));

            if !ctx.in_loop() {
                ctx.diagnostics.push(HirError::ContinueOutsideLoop {
                    span: to_span(span),
                });
            } else if let Some(lbl) = label
                && !ctx.find_loop(Some(lbl))
            {
                ctx.diagnostics.push(HirError::UnknownLoopLabel {
                    label: ctx.interner.resolve(&lbl.spur()).to_string(),
                    span: to_span(span),
                });
            }

            Some(Item::Expression(Expression::Continue { label }))
        }
        toy_ast::Item::Expression(expr) => {
            let value = lower_expression(ctx, Some(expr));
            Some(Item::Expression(value))
        }
        toy_ast::Item::UseStatement(_use_stmt) => {
            // TODO: Handle use statements during module resolution phase
            // For now, skip them in HIR lowering
            None
        }
    }
}

fn lower_params(ctx: &mut Ctx, params: Option<toy_ast::ParameterList>) -> Vec<FunctionParam> {
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

fn lower_expression(ctx: &mut Ctx, ast: Option<toy_ast::Expression>) -> Expression {
    let Some(ast) = ast else {
        return Expression::Missing;
    };

    match ast {
        toy_ast::Expression::Infix(infix) => lower_infix(ctx, infix),
        toy_ast::Expression::Literal(lit) => Expression::Literal(lower_literal(lit)),
        toy_ast::Expression::Parenthesis(paren) => lower_expression(ctx, paren.expression()),
        toy_ast::Expression::Prefix(prefix) => lower_prefix(ctx, prefix),
        toy_ast::Expression::VariableReference(var) => {
            if let Some(name_token) = var.name() {
                let name_str = name_token.text();
                let ref_span = name_token.text_range();
                ctx.add_reference(name_str, ref_span);
                let name = ctx.intern(name_str);

                // Resolve name based on context
                let resolved_name = if let Some(resolution) = &ctx.resolution {
                    // Check if it's an imported symbol
                    if let Some(symbol_table) = resolution.symbol_tables.get(&resolution.module_id)
                    {
                        if let Some(resolved_symbol) = symbol_table.all_symbols.get(name_str) {
                            match &resolved_symbol.source {
                                toy_resolve::SymbolSource::ReExport { from_module, .. } => {
                                    ResolvedIdent::External {
                                        module_id: ModuleId(from_module.0),
                                        name,
                                    }
                                }
                                _ => ResolvedIdent::Local(name),
                            }
                        } else {
                            // Symbol not found in resolution, treat as local
                            ResolvedIdent::Local(name)
                        }
                    } else {
                        ResolvedIdent::Local(name)
                    }
                } else {
                    // No resolution context, treat as local
                    ResolvedIdent::Local(name)
                };

                Expression::VariableRef {
                    name: resolved_name,
                }
            } else {
                Expression::Missing
            }
        }
        toy_ast::Expression::Block(block) => lower_block(ctx, block),
        toy_ast::Expression::If(if_expr) => lower_if(ctx, if_expr),
        toy_ast::Expression::Loop(loop_expr) => lower_loop(ctx, loop_expr),
        toy_ast::Expression::While(while_expr) => lower_while(ctx, while_expr),
        toy_ast::Expression::For(for_expr) => lower_for(ctx, for_expr),
        toy_ast::Expression::Range(range_expr) => lower_range(ctx, range_expr),
        toy_ast::Expression::Function(fn_expr) => lower_function_expr(ctx, fn_expr),
        toy_ast::Expression::Call(call) => lower_call(ctx, call),
        toy_ast::Expression::List(list) => lower_list(ctx, list),
        toy_ast::Expression::Index(index) => lower_index(ctx, index),
        toy_ast::Expression::Slice(slice) => lower_slice(ctx, slice),
        toy_ast::Expression::Tuple(tuple) => lower_tuple(ctx, tuple),
        toy_ast::Expression::TupleAccess(access) => lower_tuple_access(ctx, access),
    }
}

fn lower_function_expr(ctx: &mut Ctx, fn_expr: toy_ast::FunctionExpression) -> Expression {
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

fn lower_call(ctx: &mut Ctx, call: toy_ast::CallExpression) -> Expression {
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

fn lower_list(ctx: &mut Ctx, list: toy_ast::ListExpression) -> Expression {
    let elements: Vec<ExprIdx> = list
        .elements()
        .map(|e| {
            let span = e.syntax().text_range();
            let expr = lower_expression(ctx, Some(e));
            ctx.alloc(expr, span)
        })
        .collect();

    Expression::List { elements }
}

fn lower_index(ctx: &mut Ctx, index: toy_ast::IndexExpression) -> Expression {
    let coll_ast = index.collection();
    let coll_span = coll_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let coll_expr = lower_expression(ctx, coll_ast);
    let collection = ctx.alloc(coll_expr, coll_span);

    let idx_ast = index.index();
    let idx_span = idx_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let idx_expr = lower_expression(ctx, idx_ast);
    let index_idx = ctx.alloc(idx_expr, idx_span);

    Expression::Index {
        collection,
        index: index_idx,
    }
}

fn lower_slice(ctx: &mut Ctx, slice: toy_ast::SliceExpression) -> Expression {
    let coll_ast = slice.collection();
    let coll_span = coll_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let coll_expr = lower_expression(ctx, coll_ast);
    let collection = ctx.alloc(coll_expr, coll_span);

    let start = slice.start().map(|e| {
        let span = e.syntax().text_range();
        let expr = lower_expression(ctx, Some(e));
        ctx.alloc(expr, span)
    });

    let end = slice.end().map(|e| {
        let span = e.syntax().text_range();
        let expr = lower_expression(ctx, Some(e));
        ctx.alloc(expr, span)
    });

    Expression::Slice {
        collection,
        start,
        end,
    }
}

fn lower_tuple(ctx: &mut Ctx, tuple: toy_ast::TupleExpression) -> Expression {
    let elements: Vec<ExprIdx> = tuple
        .elements()
        .map(|e| {
            let span = e.syntax().text_range();
            let expr = lower_expression(ctx, Some(e));
            ctx.alloc(expr, span)
        })
        .collect();

    Expression::Tuple { elements }
}

fn lower_tuple_access(ctx: &mut Ctx, access: toy_ast::TupleAccessExpression) -> Expression {
    let tuple_ast = access.tuple();
    let tuple_span = tuple_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let tuple_expr = lower_expression(ctx, tuple_ast);
    let tuple = ctx.alloc(tuple_expr, tuple_span);

    let index = access.index().unwrap_or(0);

    Expression::TupleAccess { tuple, index }
}

fn lower_infix(ctx: &mut Ctx, ast: toy_ast::InfixExpression) -> Expression {
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
        ctx.diagnostics.push(HirError::DivisionByZero {
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

fn is_zero_literal(ast: &Option<toy_ast::Expression>) -> bool {
    let Some(toy_ast::Expression::Literal(lit)) = ast else {
        return false;
    };
    matches!(
        lit.value(),
        Some(toy_ast::LiteralValue::Integer(0)) | Some(toy_ast::LiteralValue::Float(0.0))
    )
}

fn lower_prefix(ctx: &mut Ctx, ast: toy_ast::PrefixExpression) -> Expression {
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

fn lower_literal(ast: toy_ast::Literal) -> Literal {
    match ast.value() {
        Some(toy_ast::LiteralValue::Integer(n)) => Literal::Integer(n),
        Some(toy_ast::LiteralValue::Float(n)) => Literal::Float(n),
        Some(toy_ast::LiteralValue::Boolean(b)) => Literal::Boolean(b),
        Some(toy_ast::LiteralValue::String(s)) => Literal::String(s),
        None => Literal::Integer(0), // fallback for malformed literals
    }
}

fn lower_block(ctx: &mut Ctx, ast: toy_ast::BlockExpression) -> Expression {
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
        ctx.diagnostics.push(HirError::EmptyBlock {
            span: to_span(ast.syntax().text_range()),
        });
    }

    ctx.symbols.pop_scope();

    Expression::Block { items, tail }
}

fn lower_if(ctx: &mut Ctx, ast: toy_ast::IfExpression) -> Expression {
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
            toy_ast::ElseBranch::Block(b) => lower_block(ctx, b),
            toy_ast::ElseBranch::ElseIf(if_e) => lower_if(ctx, if_e),
        };
        ctx.alloc(expr, span)
    });

    Expression::If {
        condition,
        then_branch,
        else_branch,
    }
}

fn lower_loop(ctx: &mut Ctx, ast: toy_ast::LoopExpression) -> Expression {
    let label = ast.label().map(|t| ctx.intern(t.text()));

    ctx.push_loop(label);

    let body_ast = ast.body();
    let body_span = body_ast
        .as_ref()
        .map(|b| b.syntax().text_range())
        .unwrap_or_default();
    let body = body_ast
        .map(|b| lower_block(ctx, b))
        .unwrap_or(Expression::Missing);
    let body_idx = ctx.alloc(body, body_span);

    ctx.pop_loop();

    Expression::Loop {
        label,
        body: body_idx,
    }
}

fn lower_while(ctx: &mut Ctx, ast: toy_ast::WhileExpression) -> Expression {
    let cond_ast = ast.condition();
    let cond_span = cond_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let cond_expr = lower_expression(ctx, cond_ast);
    let condition = ctx.alloc(cond_expr, cond_span);

    let label = ast.label().map(|t| ctx.intern(t.text()));

    ctx.push_loop(label);

    let body_ast = ast.body();
    let body_span = body_ast
        .as_ref()
        .map(|b| b.syntax().text_range())
        .unwrap_or_default();
    let body = body_ast
        .map(|b| lower_block(ctx, b))
        .unwrap_or(Expression::Missing);
    let body_idx = ctx.alloc(body, body_span);

    ctx.pop_loop();

    Expression::While {
        condition,
        label,
        body: body_idx,
    }
}

fn lower_for(ctx: &mut Ctx, ast: toy_ast::ForExpression) -> Expression {
    let binding = ast
        .binding()
        .map(|t| ctx.intern(t.text()))
        .unwrap_or_else(|| ctx.intern("_"));

    let iterable_ast = ast.iterable();
    let iterable_span = iterable_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let iterable_expr = lower_expression(ctx, iterable_ast);
    let iterable = ctx.alloc(iterable_expr, iterable_span);

    let label = ast.label().map(|t| ctx.intern(t.text()));

    ctx.push_loop(label);

    let body_ast = ast.body();
    let body_span = body_ast
        .as_ref()
        .map(|b| b.syntax().text_range())
        .unwrap_or_default();
    let body = body_ast
        .map(|b| lower_block(ctx, b))
        .unwrap_or(Expression::Missing);
    let body_idx = ctx.alloc(body, body_span);

    ctx.pop_loop();

    Expression::For {
        binding,
        iterable,
        label,
        body: body_idx,
    }
}

fn lower_range(ctx: &mut Ctx, ast: toy_ast::RangeExpression) -> Expression {
    let start_ast = ast.start();
    let start_span = start_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let start_expr = lower_expression(ctx, start_ast);
    let start = ctx.alloc(start_expr, start_span);

    let end_ast = ast.end();
    let end_span = end_ast
        .as_ref()
        .map(|e| e.syntax().text_range())
        .unwrap_or_default();
    let end_expr = lower_expression(ctx, end_ast);
    let end = ctx.alloc(end_expr, end_span);

    Expression::Range { start, end }
}

fn lower_block_item(ctx: &mut Ctx, ast: toy_ast::Item) -> Option<BlockItem> {
    match ast {
        toy_ast::Item::FunctionDefinition(fn_def) => {
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
        toy_ast::Item::VariableDefinition(def) => {
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
        toy_ast::Item::VariableAssignment(asgn) => {
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
        toy_ast::Item::IndexAssignment(idx_asgn) => {
            let target = idx_asgn.target()?;
            let value = lower_expression(ctx, idx_asgn.value());
            let value_span = idx_asgn
                .value()
                .map(|e| e.syntax().text_range())
                .unwrap_or_default();
            let value_idx = ctx.alloc(value, value_span);

            match target {
                toy_ast::Expression::Index(idx_expr) => {
                    let collection = idx_expr.collection().map(|e| {
                        let span = e.syntax().text_range();
                        let expr = lower_expression(ctx, Some(e));
                        ctx.alloc(expr, span)
                    })?;
                    let index = idx_expr.index().map(|e| {
                        let span = e.syntax().text_range();
                        let expr = lower_expression(ctx, Some(e));
                        ctx.alloc(expr, span)
                    })?;
                    Some(BlockItem::IndexAssignment {
                        collection,
                        index,
                        value: value_idx,
                    })
                }
                _ => None,
            }
        }
        toy_ast::Item::ReturnStatement(ret) => {
            let value = ret.value().map(|e| {
                let span = e.syntax().text_range();
                let expr = lower_expression(ctx, Some(e));
                ctx.alloc(expr, span)
            });
            Some(BlockItem::Return { value })
        }
        toy_ast::Item::EchoStatement(echo) => {
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
        toy_ast::Item::BreakStatement(brk) => {
            let span = brk.syntax().text_range();
            let label = brk.label().map(|t| ctx.intern(t.text()));

            if !ctx.in_loop() {
                ctx.diagnostics.push(HirError::BreakOutsideLoop {
                    span: to_span(span),
                });
            } else if let Some(lbl) = label
                && !ctx.find_loop(Some(lbl))
            {
                ctx.diagnostics.push(HirError::UnknownLoopLabel {
                    label: ctx.interner.resolve(&lbl.spur()).to_string(),
                    span: to_span(span),
                });
            }

            Some(BlockItem::Break { label })
        }
        toy_ast::Item::ContinueStatement(cont) => {
            let span = cont.syntax().text_range();
            let label = cont.label().map(|t| ctx.intern(t.text()));

            if !ctx.in_loop() {
                ctx.diagnostics.push(HirError::ContinueOutsideLoop {
                    span: to_span(span),
                });
            } else if let Some(lbl) = label
                && !ctx.find_loop(Some(lbl))
            {
                ctx.diagnostics.push(HirError::UnknownLoopLabel {
                    label: ctx.interner.resolve(&lbl.spur()).to_string(),
                    span: to_span(span),
                });
            }

            Some(BlockItem::Continue { label })
        }
        toy_ast::Item::Expression(expr) => {
            let span = expr.syntax().text_range();
            let value = lower_expression(ctx, Some(expr));
            let idx = ctx.alloc(value, span);
            Some(BlockItem::Expression(idx))
        }
        toy_ast::Item::UseStatement(_use_stmt) => {
            // TODO: Handle use statements during module resolution phase
            // For now, skip them in HIR lowering
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Definition, Expression, InfixOp, Item, Literal, PrefixOp};
    use toy_ast::AstNode;

    fn lower_src(src: &str) -> LowerResult {
        let (node, _errors) = toy_parser::parse(src);
        let root = toy_ast::Root::cast(&node).unwrap();
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
        assert_eq!(result.resolve_ident(name), "y");
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
    fn empty_parens_is_empty_tuple() {
        // "()" is the empty tuple (unit), not a missing expression
        let result = lower_src("x := ()");
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[0] else {
            panic!("expected variable definition");
        };
        let Expression::Tuple { elements } = value else {
            panic!("expected Tuple");
        };
        assert!(elements.is_empty());
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

    // === Resolution tests ===

    #[test]
    fn lower_with_external_resolution() {
        use crate::ModuleId;
        use std::collections::HashMap;

        // Module 1 source code (defines a function)
        let module1_src = "fn greet(name) { echo name }";
        let (node1, _) = toy_parser::parse(module1_src);
        let _root1 = toy_ast::Root::cast(&node1).unwrap();

        // Module 2 source code (uses function from module 1)
        let module2_src = "greet(\"hello\")";
        let (node2, _) = toy_parser::parse(module2_src);
        let root2 = toy_ast::Root::cast(&node2).unwrap();

        // Create mock symbol tables
        let mut symbol_tables = HashMap::new();

        // Module 1 exports 'greet' function
        let mut module1_symbols = toy_resolve::SymbolTable::new(toy_resolve::ModuleId(0));
        module1_symbols.add_local_symbol(&toy_resolve::Export {
            name: "greet".to_string(),
            kind: toy_resolve::SymbolKind::Function,
            is_pub: true,
            span: toy_cst::TextRange::default(),
        });
        symbol_tables.insert(ModuleId(0), module1_symbols.clone());

        // Module 2 imports 'greet' from module 1
        let mut module2_symbols = toy_resolve::SymbolTable::new(toy_resolve::ModuleId(1));
        // Add greet as a re-export to simulate import
        let greet_symbol = module1_symbols.exports.get("greet").unwrap();
        module2_symbols.add_reexport("greet".to_string(), greet_symbol, toy_resolve::ModuleId(0));
        symbol_tables.insert(ModuleId(1), module2_symbols);

        // Lower module 2 with resolution
        let result = lower_with_resolution(root2, ModuleId(1), symbol_tables, HashMap::new());

        // Check that the function call in module 2 correctly references module 1's function
        assert_eq!(result.items.len(), 1);
        let Item::Expression(Expression::Call { callee, .. }) = &result.items[0] else {
            panic!("expected call expression");
        };

        let callee_expr = &result.expressions[*callee];
        let Expression::VariableRef { name } = callee_expr else {
            panic!("expected variable ref");
        };

        // Should be an external reference to module 0
        assert!(matches!(
            name,
            ResolvedIdent::External {
                module_id: ModuleId(0),
                ..
            }
        ));
        assert_eq!(result.resolve_ident(name), "greet");
    }

    #[test]
    fn lower_with_local_resolution() {
        use crate::ModuleId;
        use std::collections::HashMap;

        // Module with local function definition and call
        let src = "fn add(a, b) { a + b }\nx := add(1, 2)";
        let (node, _) = toy_parser::parse(src);
        let root = toy_ast::Root::cast(&node).unwrap();

        // Create symbol table with local function
        let mut symbol_tables = HashMap::new();
        let mut module_symbols = toy_resolve::SymbolTable::new(toy_resolve::ModuleId(0));
        module_symbols.add_local_symbol(&toy_resolve::Export {
            name: "add".to_string(),
            kind: toy_resolve::SymbolKind::Function,
            is_pub: false,
            span: toy_cst::TextRange::default(),
        });
        symbol_tables.insert(ModuleId(0), module_symbols);

        // Lower with resolution
        let result = lower_with_resolution(root, ModuleId(0), symbol_tables, HashMap::new());

        // Check the function call
        let Item::Definition(Definition::Variable { value, .. }) = &result.items[1] else {
            panic!("expected variable definition");
        };
        let Expression::Call { callee, .. } = value else {
            panic!("expected call expression");
        };

        let callee_expr = &result.expressions[*callee];
        let Expression::VariableRef { name } = callee_expr else {
            panic!("expected variable ref");
        };

        // Should be a local reference
        assert!(matches!(name, ResolvedIdent::Local(_)));
        assert_eq!(result.resolve_ident(name), "add");
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
        assert_eq!(result.resolve_ident(name), "add");
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
