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

impl Item {
    pub fn cast(node: SyntaxNode) -> Option<Item> {
        let result = match node.kind() {
            SyntaxKind::FunctionDefinition => Self::FunctionDefinition(FunctionDefinition(node)),
            SyntaxKind::VariableDefinition => Self::VariableDefinition(VariableDefinition(node)),
            SyntaxKind::VariableAssignment => Self::VariableAssignment(VariableAssignment(node)),
            SyntaxKind::IndexAssignment => Self::IndexAssignment(IndexAssignment(node)),
            SyntaxKind::ReturnStatement => Self::ReturnStatement(ReturnStatement(node)),
            SyntaxKind::EchoStatement => Self::EchoStatement(EchoStatement(node)),
            SyntaxKind::BreakStatement => Self::BreakStatement(BreakStatement(node)),
            SyntaxKind::ContinueStatement => Self::ContinueStatement(ContinueStatement(node)),
            _ => Self::Expression(Expression::cast(node)?),
        };
        Some(result)
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
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
    Block(BlockExpression),
    If(IfExpression),
    Loop(LoopExpression),
    While(WhileExpression),
    Function(FunctionExpression),
    Call(CallExpression),
    List(ListExpression),
    Index(IndexExpression),
    Slice(SliceExpression),
    Tuple(TupleExpression),
    TupleAccess(TupleAccessExpression),
}

impl Expression {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        let result = match node.kind() {
            SyntaxKind::InfixExpression => Self::Infix(InfixExpression(node)),
            SyntaxKind::Literal => Self::Literal(Literal(node)),
            SyntaxKind::ParenthesisExpression => Self::Parenthesis(ParenthesisExpression(node)),
            SyntaxKind::PrefixExpression => Self::Prefix(PrefixExpression(node)),
            SyntaxKind::VariableReference => Self::VariableReference(VariableReference(node)),
            SyntaxKind::BlockExpression => Self::Block(BlockExpression(node)),
            SyntaxKind::IfExpression => Self::If(IfExpression(node)),
            SyntaxKind::LoopExpression => Self::Loop(LoopExpression(node)),
            SyntaxKind::WhileExpression => Self::While(WhileExpression(node)),
            SyntaxKind::FunctionExpression => Self::Function(FunctionExpression(node)),
            SyntaxKind::CallExpression => Self::Call(CallExpression(node)),
            SyntaxKind::ListExpression => Self::List(ListExpression(node)),
            SyntaxKind::IndexExpression => Self::Index(IndexExpression(node)),
            SyntaxKind::SliceExpression => Self::Slice(SliceExpression(node)),
            SyntaxKind::TupleExpression => Self::Tuple(TupleExpression(node)),
            SyntaxKind::TupleAccessExpression => Self::TupleAccess(TupleAccessExpression(node)),
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
            .filter_map(SyntaxElement::into_token)
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
                Some(BlockExpression(n))
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
            SyntaxKind::BlockExpression => Some(ElseBranch::Block(BlockExpression(else_node))),
            SyntaxKind::IfExpression => Some(ElseBranch::ElseIf(IfExpression(else_node))),
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
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
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
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
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
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
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
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }

    pub fn body(&self) -> Option<BlockExpression> {
        self.0.children().find_map(BlockExpression::cast)
    }
}

ast_node!(BreakStatement, SyntaxKind::BreakStatement);

impl BreakStatement {
    pub fn label(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }
}

ast_node!(ContinueStatement, SyntaxKind::ContinueStatement);

impl ContinueStatement {
    pub fn label(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
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
            match child {
                SyntaxElement::Token(t) if t.kind() == SyntaxKind::DotDot => {
                    saw_dotdot = true;
                }
                SyntaxElement::Node(n) if !saw_dotdot => {
                    if is_first {
                        // Skip the collection (first expression)
                        is_first = false;
                    } else {
                        // This is the start expression
                        return Expression::cast(n);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The end index (None for `[start..]`)
    pub fn end(&self) -> Option<Expression> {
        // End is the expression after DotDot (if any)
        let mut saw_dotdot = false;
        for child in self.0.children_with_tokens() {
            match child {
                SyntaxElement::Token(t) if t.kind() == SyntaxKind::DotDot => {
                    saw_dotdot = true;
                }
                SyntaxElement::Node(n) if saw_dotdot => {
                    return Expression::cast(n);
                }
                _ => {}
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
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Integer)
            .and_then(|token| token.text().parse().ok())
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
        let syntax = root.syntax().clone();
        // Root node cannot cast to VariableDefinition
        assert!(VariableDefinition::cast(syntax).is_none());
    }
}
