mod node;

pub use node::AstNode;

use node::ast_node;
use toy_cst::{ResolvedElement, ResolvedNode, ResolvedToken, SyntaxKind};

pub type SyntaxNode = ResolvedNode;
pub type SyntaxToken = ResolvedToken;
pub type SyntaxElement = ResolvedElement;

ast_node!(Root, SyntaxKind::Root);

impl Root {
    pub fn items(&self) -> impl Iterator<Item = Item> {
        self.0.children().filter_map(Item::cast)
    }
}

#[derive(Debug)]
pub enum Item {
    UseStatement(UseStatement),
    FunctionDefinition(FunctionDefinition),
    VariableDefinition(VariableDefinition),
    VariableAssignment(VariableAssignment),
    IndexAssignment(IndexAssignment),
    ReturnStatement(ReturnStatement),
    EchoStatement(EchoStatement),
    BreakStatement(BreakStatement),
    ContinueStatement(ContinueStatement),
    Expression(Expression),
}

impl AstNode for Item {
    /// Returns true if this SyntaxKind can be cast to an Item.
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::UseStatement
                | SyntaxKind::FunctionDefinition
                | SyntaxKind::VariableDefinition
                | SyntaxKind::VariableAssignment
                | SyntaxKind::IndexAssignment
                | SyntaxKind::ReturnStatement
                | SyntaxKind::EchoStatement
                | SyntaxKind::BreakStatement
                | SyntaxKind::ContinueStatement
        ) || Expression::can_cast(kind)
    }

    fn cast(node: &SyntaxNode) -> Option<Item> {
        let result = match node.kind() {
            SyntaxKind::UseStatement => Self::UseStatement(UseStatement(node.clone())),
            SyntaxKind::FunctionDefinition => {
                Self::FunctionDefinition(FunctionDefinition(node.clone()))
            }
            SyntaxKind::VariableDefinition => {
                Self::VariableDefinition(VariableDefinition(node.clone()))
            }
            SyntaxKind::VariableAssignment => {
                Self::VariableAssignment(VariableAssignment(node.clone()))
            }
            SyntaxKind::IndexAssignment => Self::IndexAssignment(IndexAssignment(node.clone())),
            SyntaxKind::ReturnStatement => Self::ReturnStatement(ReturnStatement(node.clone())),
            SyntaxKind::EchoStatement => Self::EchoStatement(EchoStatement(node.clone())),
            SyntaxKind::BreakStatement => Self::BreakStatement(BreakStatement(node.clone())),
            SyntaxKind::ContinueStatement => {
                Self::ContinueStatement(ContinueStatement(node.clone()))
            }
            _ => Self::Expression(Expression::cast(node)?),
        };
        Some(result)
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Item::UseStatement(n) => n.syntax(),
            Item::FunctionDefinition(n) => n.syntax(),
            Item::VariableDefinition(n) => n.syntax(),
            Item::VariableAssignment(n) => n.syntax(),
            Item::IndexAssignment(n) => n.syntax(),
            Item::ReturnStatement(n) => n.syntax(),
            Item::EchoStatement(n) => n.syntax(),
            Item::BreakStatement(n) => n.syntax(),
            Item::ContinueStatement(n) => n.syntax(),
            Item::Expression(e) => e.syntax(),
        }
    }
}

// ========================================
// Module system nodes
// ========================================

ast_node!(UseStatement, SyntaxKind::UseStatement);

impl UseStatement {
    /// Check if this is a public re-export (has `pub` modifier)
    pub fn is_pub(&self) -> bool {
        // Check if there's a PubKeyword token in parent that comes before this UseStatement
        if let Some(parent) = self.0.parent() {
            let mut prev_was_pub = false;
            for child in parent.children_with_tokens() {
                if let Some(token) = child.as_token() {
                    if token.kind() == SyntaxKind::PubKeyword {
                        prev_was_pub = true;
                    } else if !token.kind().is_trivia() {
                        // Found non-trivia token that's not pub, reset
                        prev_was_pub = false;
                    }
                } else if let Some(node) = child.as_node() {
                    // Check if this is our UseStatement
                    if node.kind() == SyntaxKind::UseStatement {
                        // If prev_was_pub is true, this UseStatement follows a pub token
                        return prev_was_pub;
                    }
                    // Found a different node, reset
                    prev_was_pub = false;
                }
            }
        }
        false
    }

    /// Get the module path (e.g., `std.io` or `.helpers`)
    pub fn module_path(&self) -> Option<ModulePath> {
        self.0.children().find_map(ModulePath::cast)
    }

    /// Get the import list for selective imports (e.g., `{sin, cos, PI}`)
    pub fn import_list(&self) -> Option<ImportList> {
        self.0.children().find_map(ImportList::cast)
    }

    /// Get the import alias (e.g., `as m` in `use std.math as m`)
    pub fn import_alias(&self) -> Option<ImportAlias> {
        self.0.children().find_map(ImportAlias::cast)
    }
}

ast_node!(ModulePath, SyntaxKind::ModulePath);

impl ModulePath {
    /// Check if this is a relative path (starts with '.')
    pub fn is_relative(&self) -> bool {
        // Check if the first non-trivia token is a Dot
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| !token.kind().is_trivia())
            .is_some_and(|token| token.kind() == SyntaxKind::Dot)
    }

    /// Get all segments of the path (e.g., ["std", "io"] for `std.io`)
    pub fn segments(&self) -> impl Iterator<Item = SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }
}

ast_node!(ImportList, SyntaxKind::ImportList);

impl ImportList {
    /// Get all import items in the list
    pub fn items(&self) -> impl Iterator<Item = ImportItem> {
        self.0.children().filter_map(ImportItem::cast)
    }
}

ast_node!(ImportItem, SyntaxKind::ImportItem);

impl ImportItem {
    /// Get the name being imported
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }

    /// Get the alias for this item if present (e.g., `sine` in `sin as sine`)
    pub fn alias(&self) -> Option<SyntaxToken> {
        let mut found_as = false;
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| {
                if token.kind() == SyntaxKind::AsKeyword {
                    found_as = true;
                    false
                } else {
                    found_as && token.kind() == SyntaxKind::Identifier
                }
            })
            .cloned()
    }
}

ast_node!(ImportAlias, SyntaxKind::ImportAlias);

impl ImportAlias {
    /// Get the alias name (e.g., `m` in `as m`)
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }
}

ast_node!(VariableAssignment, SyntaxKind::VariableAssignment);

impl VariableAssignment {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
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
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
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
    Block(BlockExpression),
    If(IfExpression),
    Loop(LoopExpression),
    While(WhileExpression),
    For(ForExpression),
    Range(RangeExpression),
    Function(FunctionExpression),
    Call(CallExpression),
    List(ListExpression),
    Index(IndexExpression),
    Slice(SliceExpression),
    Tuple(TupleExpression),
    TupleAccess(TupleAccessExpression),
}

impl Expression {
    /// Returns true if this SyntaxKind can be cast to an Expression.
    pub fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::InfixExpression
                | SyntaxKind::Literal
                | SyntaxKind::ParenthesisExpression
                | SyntaxKind::PrefixExpression
                | SyntaxKind::VariableReference
                | SyntaxKind::BlockExpression
                | SyntaxKind::IfExpression
                | SyntaxKind::LoopExpression
                | SyntaxKind::WhileExpression
                | SyntaxKind::ForExpression
                | SyntaxKind::RangeExpression
                | SyntaxKind::FunctionExpression
                | SyntaxKind::CallExpression
                | SyntaxKind::ListExpression
                | SyntaxKind::IndexExpression
                | SyntaxKind::SliceExpression
                | SyntaxKind::TupleExpression
                | SyntaxKind::TupleAccessExpression
        )
    }

    pub fn cast(node: &SyntaxNode) -> Option<Self> {
        let result = match node.kind() {
            SyntaxKind::InfixExpression => Self::Infix(InfixExpression(node.clone())),
            SyntaxKind::Literal => Self::Literal(Literal(node.clone())),
            SyntaxKind::ParenthesisExpression => {
                Self::Parenthesis(ParenthesisExpression(node.clone()))
            }
            SyntaxKind::PrefixExpression => Self::Prefix(PrefixExpression(node.clone())),
            SyntaxKind::VariableReference => {
                Self::VariableReference(VariableReference(node.clone()))
            }
            SyntaxKind::BlockExpression => Self::Block(BlockExpression(node.clone())),
            SyntaxKind::IfExpression => Self::If(IfExpression(node.clone())),
            SyntaxKind::LoopExpression => Self::Loop(LoopExpression(node.clone())),
            SyntaxKind::WhileExpression => Self::While(WhileExpression(node.clone())),
            SyntaxKind::ForExpression => Self::For(ForExpression(node.clone())),
            SyntaxKind::RangeExpression => Self::Range(RangeExpression(node.clone())),
            SyntaxKind::FunctionExpression => Self::Function(FunctionExpression(node.clone())),
            SyntaxKind::CallExpression => Self::Call(CallExpression(node.clone())),
            SyntaxKind::ListExpression => Self::List(ListExpression(node.clone())),
            SyntaxKind::IndexExpression => Self::Index(IndexExpression(node.clone())),
            SyntaxKind::SliceExpression => Self::Slice(SliceExpression(node.clone())),
            SyntaxKind::TupleExpression => Self::Tuple(TupleExpression(node.clone())),
            SyntaxKind::TupleAccessExpression => {
                Self::TupleAccess(TupleAccessExpression(node.clone()))
            }
            _ => return None,
        };
        Some(result)
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Expression::Infix(n) => n.syntax(),
            Expression::Literal(n) => n.syntax(),
            Expression::Parenthesis(n) => n.syntax(),
            Expression::Prefix(n) => n.syntax(),
            Expression::VariableReference(n) => n.syntax(),
            Expression::Block(n) => n.syntax(),
            Expression::If(n) => n.syntax(),
            Expression::Loop(n) => n.syntax(),
            Expression::While(n) => n.syntax(),
            Expression::For(n) => n.syntax(),
            Expression::Range(n) => n.syntax(),
            Expression::Function(n) => n.syntax(),
            Expression::Call(n) => n.syntax(),
            Expression::List(n) => n.syntax(),
            Expression::Index(n) => n.syntax(),
            Expression::Slice(n) => n.syntax(),
            Expression::Tuple(n) => n.syntax(),
            Expression::TupleAccess(n) => n.syntax(),
        }
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
            .filter_map(|el| el.into_token())
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
            .cloned()
    }
}

ast_node!(Literal, SyntaxKind::Literal);

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Integer(u64),
    Float(f64),
    Boolean(bool),
    String(String),
}

impl Literal {
    pub fn value(&self) -> Option<LiteralValue> {
        let token = self
            .0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::Integer
                        | SyntaxKind::BinaryInteger
                        | SyntaxKind::OctalInteger
                        | SyntaxKind::HexInteger
                        | SyntaxKind::Float
                        | SyntaxKind::FloatExponent
                        | SyntaxKind::String
                        | SyntaxKind::MultiLineString
                        | SyntaxKind::TrueKeyword
                        | SyntaxKind::FalseKeyword
                )
            })?;

        match token.kind() {
            SyntaxKind::Integer => {
                let text = token.text().replace('_', "");
                text.parse::<u64>().ok().map(LiteralValue::Integer)
            }
            SyntaxKind::BinaryInteger => {
                let text = token.text().replace('_', "");
                u64::from_str_radix(&text[2..], 2)
                    .ok()
                    .map(LiteralValue::Integer)
            }
            SyntaxKind::OctalInteger => {
                let text = token.text().replace('_', "");
                u64::from_str_radix(&text[2..], 8)
                    .ok()
                    .map(LiteralValue::Integer)
            }
            SyntaxKind::HexInteger => {
                let text = token.text().replace('_', "");
                u64::from_str_radix(&text[2..], 16)
                    .ok()
                    .map(LiteralValue::Integer)
            }
            SyntaxKind::Float | SyntaxKind::FloatExponent => {
                let text = token.text().replace('_', "");
                text.parse::<f64>().ok().map(LiteralValue::Float)
            }
            SyntaxKind::String => {
                let text = token.text();
                // Remove surrounding quotes and process escapes
                let inner = &text[1..text.len() - 1];
                Some(LiteralValue::String(process_escapes(inner)))
            }
            SyntaxKind::MultiLineString => {
                let text = token.text();
                // Remove surrounding triple quotes
                let inner = &text[3..text.len() - 3];
                Some(LiteralValue::String(inner.to_string()))
            }
            SyntaxKind::TrueKeyword => Some(LiteralValue::Boolean(true)),
            SyntaxKind::FalseKeyword => Some(LiteralValue::Boolean(false)),
            _ => None,
        }
    }
}

fn process_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('f') => result.push('\x0C'),
                Some('u') => {
                    // Unicode escape: \u{XXXXXX}
                    if chars.next() == Some('{') {
                        let hex: String = chars.by_ref().take_while(|&c| c != '}').collect();
                        if let Ok(code) = u32::from_str_radix(&hex, 16)
                            && let Some(ch) = char::from_u32(code)
                        {
                            result.push(ch);
                        }
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
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
            .filter_map(|el| el.into_token())
            .find(|token| matches!(token.kind(), SyntaxKind::Minus | SyntaxKind::Bang))
            .cloned()
    }
}

ast_node!(VariableReference, SyntaxKind::VariableReference);

impl VariableReference {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }
}

ast_node!(TypeAnnotation, SyntaxKind::TypeAnnotation);

impl TypeAnnotation {
    pub fn type_token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }
}

ast_node!(BlockExpression, SyntaxKind::BlockExpression);

impl BlockExpression {
    /// All items (statements/expressions) inside the block
    pub fn items(&self) -> impl Iterator<Item = Item> {
        self.0.children().filter_map(Item::cast)
    }

    /// The final expression that determines the block's value.
    /// Returns None if block is empty or ends with a definition/assignment.
    pub fn tail(&self) -> Option<Expression> {
        self.items().last().and_then(|item| match item {
            Item::Expression(e) => Some(e),
            _ => None,
        })
    }
}

ast_node!(IfExpression, SyntaxKind::IfExpression);

impl IfExpression {
    /// The condition expression
    pub fn condition(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// The then branch (block expression)
    pub fn then_branch(&self) -> Option<BlockExpression> {
        self.0.children().find_map(|n| {
            if n.kind() == SyntaxKind::BlockExpression {
                Some(BlockExpression(n.clone()))
            } else {
                None
            }
        })
    }

    /// The else branch (either a block or another if expression)
    pub fn else_branch(&self) -> Option<ElseBranch> {
        let mut children = self.0.children();
        // Skip condition expression
        children.next();
        // Skip then block
        children.find(|n| n.kind() == SyntaxKind::BlockExpression)?;
        // Next child is the else branch (if present)
        let else_node = children.next()?;
        match else_node.kind() {
            SyntaxKind::BlockExpression => {
                Some(ElseBranch::Block(BlockExpression(else_node.clone())))
            }
            SyntaxKind::IfExpression => Some(ElseBranch::ElseIf(IfExpression(else_node.clone()))),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ElseBranch {
    Block(BlockExpression),
    ElseIf(IfExpression),
}

impl ElseBranch {
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            ElseBranch::Block(b) => b.syntax(),
            ElseBranch::ElseIf(i) => i.syntax(),
        }
    }
}

ast_node!(FunctionDefinition, SyntaxKind::FunctionDefinition);

impl FunctionDefinition {
    /// Check if this function has the `pub` modifier
    pub fn is_pub(&self) -> bool {
        // Check if there's a PubKeyword token in parent that comes before this FunctionDefinition
        if let Some(parent) = self.0.parent() {
            let mut prev_was_pub = false;
            for child in parent.children_with_tokens() {
                if let Some(token) = child.as_token() {
                    if token.kind() == SyntaxKind::PubKeyword {
                        prev_was_pub = true;
                    } else if !token.kind().is_trivia() {
                        // Found non-trivia token that's not pub, reset
                        prev_was_pub = false;
                    }
                } else if let Some(node) = child.as_node() {
                    // Check if this is our FunctionDefinition
                    if node.kind() == SyntaxKind::FunctionDefinition {
                        // If prev_was_pub is true, this FunctionDefinition follows a pub token
                        return prev_was_pub;
                    }
                    // Found a different node, reset
                    prev_was_pub = false;
                }
            }
        }
        false
    }

    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }

    pub fn params(&self) -> Option<ParameterList> {
        self.0.children().find_map(ParameterList::cast)
    }

    pub fn return_type(&self) -> Option<TypeAnnotation> {
        self.0.children().find_map(TypeAnnotation::cast)
    }

    pub fn body(&self) -> Option<BlockExpression> {
        self.0.children().find_map(BlockExpression::cast)
    }
}

ast_node!(FunctionExpression, SyntaxKind::FunctionExpression);

impl FunctionExpression {
    pub fn params(&self) -> Option<ParameterList> {
        self.0.children().find_map(ParameterList::cast)
    }

    pub fn return_type(&self) -> Option<TypeAnnotation> {
        self.0.children().find_map(TypeAnnotation::cast)
    }

    pub fn body(&self) -> Option<BlockExpression> {
        self.0.children().find_map(BlockExpression::cast)
    }
}

ast_node!(ParameterList, SyntaxKind::ParameterList);

impl ParameterList {
    pub fn params(&self) -> impl Iterator<Item = Parameter> {
        self.0.children().filter_map(Parameter::cast)
    }
}

ast_node!(Parameter, SyntaxKind::Parameter);

impl Parameter {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }

    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        self.0.children().find_map(TypeAnnotation::cast)
    }

    pub fn default_value(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

ast_node!(CallExpression, SyntaxKind::CallExpression);

impl CallExpression {
    pub fn callee(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    pub fn args(&self) -> impl Iterator<Item = Expression> {
        self.0.children().filter_map(Expression::cast).skip(1)
    }
}

ast_node!(ReturnStatement, SyntaxKind::ReturnStatement);

impl ReturnStatement {
    pub fn value(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

ast_node!(EchoStatement, SyntaxKind::EchoStatement);

impl EchoStatement {
    pub fn value(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

ast_node!(LoopExpression, SyntaxKind::LoopExpression);

impl LoopExpression {
    pub fn label(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }

    pub fn body(&self) -> Option<BlockExpression> {
        self.0.children().find_map(BlockExpression::cast)
    }
}

ast_node!(WhileExpression, SyntaxKind::WhileExpression);

impl WhileExpression {
    pub fn condition(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    pub fn label(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }

    pub fn body(&self) -> Option<BlockExpression> {
        self.0.children().find_map(BlockExpression::cast)
    }
}

ast_node!(ForExpression, SyntaxKind::ForExpression);

impl ForExpression {
    /// The loop variable binding (the identifier after 'for')
    pub fn binding(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .cloned()
    }

    /// The iterable expression (could be a regular expression or RangeExpression)
    pub fn iterable(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Optional label after colon
    pub fn label(&self) -> Option<SyntaxToken> {
        let mut found_colon = false;
        self.0.children_with_tokens().find_map(|child| {
            if child.kind() == SyntaxKind::Colon {
                found_colon = true;
                return None;
            }
            if found_colon && child.kind() == SyntaxKind::Identifier {
                child.into_token().cloned()
            } else {
                None
            }
        })
    }

    /// The loop body block
    pub fn body(&self) -> Option<BlockExpression> {
        self.0.children().find_map(BlockExpression::cast)
    }
}

ast_node!(RangeExpression, SyntaxKind::RangeExpression);

impl RangeExpression {
    /// The start expression (before ..)
    pub fn start(&self) -> Option<Expression> {
        self.0.children().filter_map(Expression::cast).next()
    }

    /// The end expression (after ..)
    pub fn end(&self) -> Option<Expression> {
        self.0.children().filter_map(Expression::cast).nth(1)
    }
}

ast_node!(BreakStatement, SyntaxKind::BreakStatement);

impl BreakStatement {
    pub fn label(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }
}

ast_node!(ContinueStatement, SyntaxKind::ContinueStatement);

impl ContinueStatement {
    pub fn label(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .cloned()
    }
}

// ========================================
// List expressions
// ========================================

ast_node!(ListExpression, SyntaxKind::ListExpression);

impl ListExpression {
    /// Iterator over all elements in the list literal
    pub fn elements(&self) -> impl Iterator<Item = Expression> {
        self.0.children().filter_map(Expression::cast)
    }
}

ast_node!(IndexExpression, SyntaxKind::IndexExpression);

impl IndexExpression {
    /// The collection being indexed (e.g., `arr` in `arr[0]`)
    pub fn collection(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// The index expression (e.g., `0` in `arr[0]`)
    pub fn index(&self) -> Option<Expression> {
        self.0.children().filter_map(Expression::cast).nth(1)
    }
}

ast_node!(SliceExpression, SyntaxKind::SliceExpression);

impl SliceExpression {
    /// The collection being sliced
    pub fn collection(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// The start index (None for `[..end]`)
    pub fn start(&self) -> Option<Expression> {
        // Start is the second expression (after collection) that appears before DotDot
        // Tree: [collection, start?, DotDot, end?]
        let mut saw_dotdot = false;
        let mut is_first = true;
        for child in self.0.children_with_tokens() {
            if child
                .as_token()
                .is_some_and(|t| t.kind() == SyntaxKind::DotDot)
            {
                saw_dotdot = true;
            } else if let Some(n) = child.as_node()
                && !saw_dotdot
            {
                if is_first {
                    // Skip the collection (first expression)
                    is_first = false;
                } else {
                    // This is the start expression
                    return Expression::cast(n);
                }
            }
        }
        None
    }

    /// The end index (None for `[start..]`)
    pub fn end(&self) -> Option<Expression> {
        // End is the expression after DotDot (if any)
        let mut saw_dotdot = false;
        for child in self.0.children_with_tokens() {
            if child
                .as_token()
                .is_some_and(|t| t.kind() == SyntaxKind::DotDot)
            {
                saw_dotdot = true;
            } else if let Some(n) = child.as_node()
                && saw_dotdot
            {
                return Expression::cast(n);
            }
        }
        None
    }
}

ast_node!(IndexAssignment, SyntaxKind::IndexAssignment);

impl IndexAssignment {
    /// Get the target - either an IndexExpression or SliceExpression
    pub fn target(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Get the value being assigned
    pub fn value(&self) -> Option<Expression> {
        self.0.children().filter_map(Expression::cast).nth(1)
    }
}

ast_node!(ListType, SyntaxKind::ListType);

impl ListType {
    /// The element type (e.g., `int` in `list[int]`)
    pub fn element_type(&self) -> Option<TypeAnnotation> {
        self.0.children().find_map(TypeAnnotation::cast)
    }

    /// For nested lists, get the inner ListType
    pub fn inner_list_type(&self) -> Option<ListType> {
        self.0.children().find_map(ListType::cast)
    }
}

// ========================================
// Tuple expressions
// ========================================

ast_node!(TupleExpression, SyntaxKind::TupleExpression);

impl TupleExpression {
    /// Iterator over all elements in the tuple literal
    pub fn elements(&self) -> impl Iterator<Item = Expression> {
        self.0.children().filter_map(Expression::cast)
    }
}

ast_node!(TupleAccessExpression, SyntaxKind::TupleAccessExpression);

impl TupleAccessExpression {
    /// The tuple being accessed (e.g., `point` in `point.0`)
    pub fn tuple(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// The index being accessed (e.g., `0` in `point.0`)
    pub fn index(&self) -> Option<u32> {
        self.0
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|token| token.kind() == SyntaxKind::Integer)
            .and_then(|token| token.text().parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_root(input: &str) -> Root {
        let (node, _) = toy_parser::parse(input);
        Root::cast(&node).unwrap()
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
    fn empty_parens_is_empty_tuple() {
        // "()" is the empty tuple (unit)
        let root = parse_root("x := ()");
        let item = root.items().next().unwrap();
        let Item::VariableDefinition(def) = item else {
            panic!("expected VariableDefinition")
        };
        let Expression::Tuple(tuple) = def.value().unwrap() else {
            panic!("expected TupleExpression")
        };
        assert_eq!(
            tuple.elements().count(),
            0,
            "empty parens should be empty tuple"
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
        let syntax = root.syntax();
        // Root node cannot cast to VariableDefinition
        assert!(VariableDefinition::cast(syntax).is_none());
    }

    #[test]
    fn use_statement_basic() {
        let root = parse_root("use std.io");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        assert!(!use_stmt.is_pub());
        assert!(use_stmt.module_path().is_some());
        assert!(use_stmt.import_list().is_none());
        assert!(use_stmt.import_alias().is_none());
    }

    #[test]
    fn use_statement_pub_reexport() {
        let root = parse_root("pub use std.math.PI");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        assert!(use_stmt.is_pub());
        assert!(use_stmt.module_path().is_some());
    }

    #[test]
    fn use_statement_with_alias() {
        let root = parse_root("use std.math as m");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        let alias = use_stmt.import_alias().expect("expected alias");
        assert_eq!(alias.name().unwrap().text(), "m");
    }

    #[test]
    fn use_statement_selective_imports() {
        let root = parse_root("use std.math.{sin, cos, PI}");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        let import_list = use_stmt.import_list().expect("expected import list");
        let items: Vec<_> = import_list.items().collect();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name().unwrap().text(), "sin");
        assert_eq!(items[1].name().unwrap().text(), "cos");
        assert_eq!(items[2].name().unwrap().text(), "PI");
    }

    #[test]
    fn module_path_segments() {
        let root = parse_root("use std.collections.hashmap");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        let path = use_stmt.module_path().expect("expected module path");
        let segments: Vec<_> = path.segments().map(|s| s.text().to_string()).collect();
        assert_eq!(segments, vec!["std", "collections", "hashmap"]);
        assert!(!path.is_relative());
    }

    #[test]
    fn module_path_relative() {
        let root = parse_root("use .helpers");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        let path = use_stmt.module_path().expect("expected module path");
        assert!(path.is_relative());
        let segments: Vec<_> = path.segments().map(|s| s.text().to_string()).collect();
        assert_eq!(segments, vec!["helpers"]);
    }

    #[test]
    fn function_definition_pub() {
        let root = parse_root("pub fn add(a: int, b: int): int { a + b }");
        let item = root.items().next().unwrap();
        let Item::FunctionDefinition(func) = item else {
            panic!("expected FunctionDefinition")
        };
        assert!(func.is_pub());
        assert_eq!(func.name().unwrap().text(), "add");
    }

    #[test]
    fn function_definition_private() {
        let root = parse_root("fn helper(x: int): int { x * 2 }");
        let item = root.items().next().unwrap();
        let Item::FunctionDefinition(func) = item else {
            panic!("expected FunctionDefinition")
        };
        assert!(!func.is_pub());
        assert_eq!(func.name().unwrap().text(), "helper");
    }

    #[test]
    fn import_item_with_alias() {
        let root = parse_root("use std.math.{sin as sine, cos}");
        let item = root.items().next().unwrap();
        let Item::UseStatement(use_stmt) = item else {
            panic!("expected UseStatement")
        };
        let import_list = use_stmt.import_list().expect("expected import list");
        let items: Vec<_> = import_list.items().collect();

        // First item has an alias
        assert_eq!(items[0].name().unwrap().text(), "sin");
        assert_eq!(items[0].alias().unwrap().text(), "sine");

        // Second item has no alias
        assert_eq!(items[1].name().unwrap().text(), "cos");
        assert!(items[1].alias().is_none());
    }
}
