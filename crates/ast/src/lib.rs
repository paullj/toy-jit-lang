use syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Debug)]
pub struct Root(SyntaxNode);

impl Root {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::Root {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn items(&self) -> impl Iterator<Item = Item> {
        self.0.children().filter_map(Item::cast)
    }
}

#[derive(Debug)]
pub enum Item {
    FunctionDefinition(FunctionDefinition),
    VariableDefinition(VariableDefinition),
    VariableAssignment(VariableAssignment),
    Expression(Expression),
}

impl Item {
    pub fn cast(node: SyntaxNode) -> Option<Item> {
        let result = match node.kind() {
            SyntaxKind::FunctionDeclaration => Self::FunctionDefinition(FunctionDefinition(node)),
            SyntaxKind::VariableDefinition => Self::VariableDefinition(VariableDefinition(node)),
            SyntaxKind::VariableAssignment => Self::VariableAssignment(VariableAssignment(node)),
            _ => Self::Expression(Expression::cast(node)?),
        };

        Some(result)
    }
}

#[derive(Debug)]
pub struct FunctionDefinition(SyntaxNode);

impl FunctionDefinition {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::FunctionDeclaration {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }

    pub fn parameters(&self) -> Option<ParameterList> {
        self.0.children().find_map(ParameterList::cast)
    }

    pub fn return_type_annotation(&self) -> Option<ReturnTypeAnnotation> {
        self.0.children().find_map(ReturnTypeAnnotation::cast)
    }

    pub fn body(&self) -> Option<Block> {
        self.0
            .children()
            .find(|node| node.kind() == SyntaxKind::Block)
            .and_then(Block::cast)
    }
}

#[derive(Debug)]
pub struct ParameterList(SyntaxNode);

impl ParameterList {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::ParameterList {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn parameters(&self) -> impl Iterator<Item = Parameter> {
        self.0.children().filter_map(Parameter::cast)
    }
}

#[derive(Debug)]
pub struct Parameter(SyntaxNode);

impl Parameter {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::Parameter {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }

    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        self.0.children().find_map(TypeAnnotation::cast)
    }
}

#[derive(Debug)]
pub struct TypeAnnotation(SyntaxNode);

impl TypeAnnotation {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::TypeAnnotation {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn type_token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }
}

#[derive(Debug)]
pub struct ReturnTypeAnnotation(SyntaxNode);

impl ReturnTypeAnnotation {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::ReturnTypeAnnotation {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn type_annotation(&self) -> Option<TypeAnnotation> {
        self.0.children().find_map(TypeAnnotation::cast)
    }
}

#[derive(Debug)]
pub struct Block(SyntaxNode);

impl Block {
    pub fn statements(&self) -> impl Iterator<Item = Statement> {
        self.0.children().filter_map(Statement::cast)
    }

    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::Block {
            Some(Self(node))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct VariableAssignment(SyntaxNode);

impl VariableAssignment {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::VariableAssignment {
            Some(Self(node))
        } else {
            None
        }
    }

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
pub struct VariableDefinition(SyntaxNode);

impl VariableDefinition {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::VariableDefinition {
            Some(Self(node))
        } else {
            None
        }
    }

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
    Binary(BinaryExpression),
    Literal(Literal),
    Parenthesis(ParenthesisExpression),
    Unary(UnaryExpression),
    VariableReference(VariableReference),
}

impl Expression {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        let result = match node.kind() {
            SyntaxKind::InfixExpression => Self::Binary(BinaryExpression(node)),
            SyntaxKind::Literal => Self::Literal(Literal(node)),
            SyntaxKind::ParenthesisExpression => Self::Parenthesis(ParenthesisExpression(node)),
            SyntaxKind::PrefixExpression => Self::Unary(UnaryExpression(node)),
            SyntaxKind::VariableReference => Self::VariableReference(VariableReference(node)),
            _ => return None,
        };

        Some(result)
    }
}

#[derive(Debug)]
pub struct BinaryExpression(SyntaxNode);

impl BinaryExpression {
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
                    SyntaxKind::Plus | SyntaxKind::Minus | SyntaxKind::Asterisk | SyntaxKind::Slash,
                )
            })
    }
}

#[derive(Debug)]
pub struct Literal(SyntaxNode);

impl Literal {
    pub fn parse(&self) -> u64 {
        // Find the actual integer token, skipping whitespace
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

#[derive(Debug)]
pub struct ParenthesisExpression(SyntaxNode);

impl ParenthesisExpression {
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

#[derive(Debug)]
pub struct UnaryExpression(SyntaxNode);

impl UnaryExpression {
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    pub fn operation(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Minus)
    }
}

#[derive(Debug)]
pub struct VariableReference(SyntaxNode);

impl VariableReference {
    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == SyntaxKind::Identifier)
    }
}

#[derive(Debug)]
pub enum Statement {
    VariableDefinition(VariableDefinition),
    VariableAssignment(VariableAssignment),
    Expression(Expression),
}

impl Statement {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        let result = match node.kind() {
            SyntaxKind::VariableDefinition => Self::VariableDefinition(VariableDefinition(node)),
            SyntaxKind::VariableAssignment => Self::VariableAssignment(VariableAssignment(node)),
            _ => Self::Expression(Expression::cast(node)?),
        };

        Some(result)
    }
}
