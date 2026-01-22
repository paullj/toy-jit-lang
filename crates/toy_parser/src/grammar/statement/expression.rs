use toy_cst::SyntaxKind;
use toy_lexer::TokenKind;

use crate::{Parser, TokenSet, marker::CompletedMarker};

pub(crate) fn expression(parser: &mut Parser) -> Option<CompletedMarker> {
    expression_with_binding_power(parser, 0)
}

/// All infix operators (used for documentation)
#[allow(dead_code)]
const OPERATOR_SET: TokenSet = TokenSet::new(&[
    // Int arithmetic
    TokenKind::Plus,
    TokenKind::Minus,
    TokenKind::Asterisk,
    TokenKind::Slash,
    TokenKind::Percent,
    // Float arithmetic
    TokenKind::PlusDot,
    TokenKind::MinusDot,
    TokenKind::AsteriskDot,
    TokenKind::SlashDot,
    // Comparison
    TokenKind::EqualsEquals,
    TokenKind::NotEquals,
    TokenKind::GreaterThan,
    TokenKind::LessThan,
    TokenKind::GreaterThanOrEqual,
    TokenKind::LessThanOrEqual,
    // Float comparison
    TokenKind::GreaterThanDot,
    TokenKind::LessThanDot,
    TokenKind::GreaterThanOrEqualDot,
    TokenKind::LessThanOrEqualDot,
    // Boolean
    TokenKind::And,
    TokenKind::Or,
]);

/// Literal tokens
const LITERAL_SET: TokenSet = TokenSet::new(&[
    TokenKind::Integer,
    TokenKind::BinaryInteger,
    TokenKind::OctalInteger,
    TokenKind::HexInteger,
    TokenKind::Float,
    TokenKind::FloatExponent,
    TokenKind::True,
    TokenKind::False,
    TokenKind::String,
    TokenKind::MultiLineString,
]);

/// Prefix operators
const PREFIX_SET: TokenSet = TokenSet::new(&[TokenKind::Minus, TokenKind::Bang]);

/// All tokens that can start an expression
pub(crate) const EXPR_FIRST: TokenSet = LITERAL_SET
    .union(TokenSet::single(TokenKind::Identifier))
    .union(PREFIX_SET)
    .union(TokenSet::single(TokenKind::LeftParenthesis))
    .union(TokenSet::single(TokenKind::LeftBrace))
    .union(TokenSet::single(TokenKind::LeftBracket))
    .union(TokenSet::single(TokenKind::If))
    .union(TokenSet::single(TokenKind::Fn))
    .union(TokenSet::single(TokenKind::Loop))
    .union(TokenSet::single(TokenKind::While))
    .union(TokenSet::single(TokenKind::For));

/// Recovery set for expression parsing (skip to newline or expr start)
const EXPR_RECOVERY: TokenSet = EXPR_FIRST.union(TokenSet::single(TokenKind::NewLine));

fn expression_with_binding_power(p: &mut Parser, min_bp: u8) -> Option<CompletedMarker> {
    let mut lhs = lhs(p)?;

    // Handle postfix operations (call expressions, index/slice, tuple access)
    loop {
        // Check for newline terminator, but allow `[` to continue if it's an index
        // expression (not a list literal). This allows `x\n[0]` to parse as `x[0]`.
        if p.at_newline_terminator() {
            // Special case: `[` after newline might be index, not list
            if p.at(TokenKind::LeftBracket) && p.is_bracket_index_not_list() {
                // Continue - this is an index expression
            } else {
                break;
            }
        }

        if p.at(TokenKind::LeftParenthesis) {
            lhs = call_expression(p, lhs);
        } else if p.at(TokenKind::LeftBracket) {
            lhs = index_or_slice_expression(p, lhs);
        } else if p.at(TokenKind::Dot) {
            lhs = tuple_access_expression(p, lhs);
        } else {
            break;
        }
    }

    inner_expression_with_binding_power(p, lhs, min_bp)
}

pub(crate) fn call_expression(p: &mut Parser, callee: CompletedMarker) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftParenthesis));

    let m = callee.precede(p);
    p.consume(); // eat '('
    p.enter_delimiter();

    let mut first = true;
    while !p.at(TokenKind::RightParenthesis) && !p.is_at_end() {
        if !first && !p.eat(TokenKind::Comma) {
            break;
        }
        first = false;

        // Handle trailing comma
        if p.at(TokenKind::RightParenthesis) {
            break;
        }

        expression(p);
    }

    p.exit_delimiter();
    if !p.eat(TokenKind::RightParenthesis) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "')'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::CallExpression)
}

pub(crate) fn inner_expression_with_binding_power(
    p: &mut Parser,
    mut lhs: CompletedMarker,
    min_bp: u8,
) -> Option<CompletedMarker> {
    while let Some(kind) = p.current() {
        // Newline terminates expression (unless inside delimiters)
        if p.at_newline_terminator() {
            break;
        }

        // Check for operator
        let Ok(op) = Operator::try_from(kind) else {
            break;
        };

        let Some((l_bp, r_bp)) = op.infix_binding_power() else {
            break;
        };
        if l_bp < min_bp {
            break;
        }

        let op_text = op.symbol();
        p.consume(); // eat operator

        // After operator, check if newline terminates or EOF (incomplete expression)
        if p.at_newline_terminator() || p.is_at_end() {
            let span = p.current_span();
            p.error(crate::ParseError::IncompleteExpression {
                at: span.into(),
                operator: op_text.to_string(),
            });
            // Complete partial infix expression
            lhs = lhs.precede(p).complete(p, SyntaxKind::InfixExpression);
            break;
        }

        let marker = lhs.precede(p);
        let parsed_rhs = expression_with_binding_power(p, r_bp).is_some();
        lhs = marker.complete(p, SyntaxKind::InfixExpression);

        if !parsed_rhs {
            // Incomplete expression - no RHS found
            let span = p.current_span();
            p.error(crate::ParseError::IncompleteExpression {
                at: span.into(),
                operator: op_text.to_string(),
            });
            break;
        }
    }

    Some(lhs)
}

fn lhs(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        Some(k) if LITERAL_SET.contains(k) => Some(literal(p)),
        Some(TokenKind::Identifier) => Some(variable_reference(p)),
        Some(TokenKind::Minus) | Some(TokenKind::Bang) => prefix_expression(p),
        Some(TokenKind::LeftParenthesis) => Some(parenthesis_expression(p)),
        Some(TokenKind::LeftBrace) => Some(block_expression(p)),
        Some(TokenKind::LeftBracket) => Some(list_expression(p)),
        Some(TokenKind::If) => Some(if_expression(p)),
        Some(TokenKind::Fn) => Some(crate::grammar::function_definition_or_expression(p)),
        Some(TokenKind::Loop) => Some(loop_expression(p)),
        Some(TokenKind::While) => Some(while_expression(p)),
        Some(TokenKind::For) => Some(for_expression(p)),
        _ => {
            p.recover("expected expression", EXPR_RECOVERY);
            None
        }
    }
}

fn literal(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.consume();
    m.complete(p, SyntaxKind::Literal)
}

fn variable_reference(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Identifier));
    let m = p.start();
    p.consume();
    m.complete(p, SyntaxKind::VariableReference)
}

fn prefix_expression(p: &mut Parser) -> Option<CompletedMarker> {
    let op = match p.current() {
        Some(TokenKind::Minus) => Operator::Minus,
        Some(TokenKind::Bang) => Operator::Bang,
        _ => return None,
    };

    let ((), r_bp) = op.prefix_binding_power()?;

    let m = p.start();
    p.consume(); // eat operator
    expression_with_binding_power(p, r_bp);
    Some(m.complete(p, SyntaxKind::PrefixExpression))
}

/// Parses parenthesis expression or tuple: `(expr)` or `(expr, expr, ...)` or `()`
fn parenthesis_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftParenthesis));

    let m = p.start();
    p.consume(); // eat '('
    p.enter_delimiter(); // newlines allowed inside parens

    // Check for empty tuple/unit: ()
    if p.at(TokenKind::RightParenthesis) {
        p.exit_delimiter();
        p.consume(); // eat ')'
        return m.complete(p, SyntaxKind::TupleExpression);
    }

    // Parse first expression
    expression_with_binding_power(p, 0);

    // Check if this is a tuple (has comma) or just parenthesized expression
    let is_tuple = if p.at(TokenKind::Comma) {
        p.consume(); // eat ','

        // Parse remaining elements
        while !p.at(TokenKind::RightParenthesis) && !p.is_at_end() {
            // Handle trailing comma
            if p.at(TokenKind::RightParenthesis) {
                break;
            }

            expression_with_binding_power(p, 0);

            if !p.at(TokenKind::RightParenthesis) && !p.eat(TokenKind::Comma) {
                break;
            }
        }
        true
    } else {
        false
    };

    p.exit_delimiter();
    if !p.eat(TokenKind::RightParenthesis) {
        // IDE-friendly: emit error but still complete the node
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "')'".to_string(),
            found,
        });
    }

    if is_tuple {
        m.complete(p, SyntaxKind::TupleExpression)
    } else {
        m.complete(p, SyntaxKind::ParenthesisExpression)
    }
}

pub(crate) fn block_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftBrace));

    let m = p.start();
    p.consume(); // eat '{'
    p.enter_delimiter(); // newlines allowed inside braces

    // Parse statements until '}'
    while !p.at(TokenKind::RightBrace) && !p.is_at_end() {
        // Skip newlines between statements
        while p.eat(TokenKind::NewLine) {}

        if p.at(TokenKind::RightBrace) {
            break;
        }

        // Parse an item (definition, assignment, or expression)
        if crate::grammar::item(p).is_none() {
            break;
        }
    }

    p.exit_delimiter();
    if !p.eat(TokenKind::RightBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'}'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::BlockExpression)
}

fn if_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::If));

    let m = p.start();
    p.consume(); // eat 'if'

    // Parse condition
    expression(p);

    // Expect '{' for then branch
    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    // Check for else (allows newline before else)
    if p.eat(TokenKind::Else) {
        if p.at(TokenKind::If) {
            // else if
            if_expression(p);
        } else {
            // else block
            if !p.at(TokenKind::LeftBrace) {
                let span = p.current_span();
                let found = p.current().map(|k| k.to_string());
                p.error(crate::ParseError::UnexpectedToken {
                    at: span.into(),
                    expected: "'{'".to_string(),
                    found,
                });
            }
            block_expression(p);
        }
    }

    m.complete(p, SyntaxKind::IfExpression)
}

fn loop_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Loop));
    let m = p.start();
    p.consume();

    if p.at(TokenKind::Colon) {
        p.consume();
        if !p.eat(TokenKind::Identifier) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "label identifier".to_string(),
                found,
            });
        }
    }

    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    m.complete(p, SyntaxKind::LoopExpression)
}

fn while_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::While));
    let m = p.start();
    p.consume();

    expression(p);

    if p.at(TokenKind::Colon) {
        p.consume();
        if !p.eat(TokenKind::Identifier) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "label identifier".to_string(),
                found,
            });
        }
    }

    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    m.complete(p, SyntaxKind::WhileExpression)
}

/// Parses for expression: `for item in collection { ... }` or `for item in start..end { ... }`
fn for_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::For));
    let m = p.start();
    p.consume(); // eat 'for'

    // Parse binding identifier
    if !p.eat(TokenKind::Identifier) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "identifier".to_string(),
            found,
        });
    }

    // Expect 'in'
    if !p.eat(TokenKind::In) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'in'".to_string(),
            found,
        });
    }

    // Parse iterable (expression or range)
    // First parse the start expression, then check for '..' for range
    range_or_expression(p);

    // Optional label
    if p.at(TokenKind::Colon) {
        p.consume();
        if !p.eat(TokenKind::Identifier) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "label identifier".to_string(),
                found,
            });
        }
    }

    // Expect block
    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    m.complete(p, SyntaxKind::ForExpression)
}

/// Parses a range expression or regular expression for 'for' loops.
/// If `..` is found after the first expression, wraps both in a RangeExpression.
/// Otherwise just parses the expression (it becomes a child of ForExpression directly).
fn range_or_expression(p: &mut Parser) {
    // Try to parse the start expression
    let Some(start) = expression(p) else {
        return;
    };

    // Check for '..' to make it a range
    if p.at(TokenKind::DotDot) {
        let m = start.precede(p);
        p.consume(); // eat '..'
        expression(p); // end expression
        m.complete(p, SyntaxKind::RangeExpression);
    }
    // Otherwise, the expression was already parsed and will be a direct child
}

/// Parses list literal: `[expr, expr, ...]`
fn list_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftBracket));

    let m = p.start();
    p.consume(); // eat '['
    p.enter_delimiter();

    let mut first = true;
    while !p.at(TokenKind::RightBracket) && !p.is_at_end() {
        if !first && !p.eat(TokenKind::Comma) {
            break;
        }
        first = false;

        // Handle trailing comma or empty list
        if p.at(TokenKind::RightBracket) {
            break;
        }

        expression(p);
    }

    p.exit_delimiter();
    if !p.eat(TokenKind::RightBracket) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "']'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::ListExpression)
}

/// Parses tuple field access: `expr.0`, `expr.1`, etc.
fn tuple_access_expression(p: &mut Parser, tuple: CompletedMarker) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Dot));

    let m = tuple.precede(p);
    p.consume(); // eat '.'

    // Expect an integer literal for tuple index
    if !p.at(TokenKind::Integer) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "tuple index (integer)".to_string(),
            found,
        });
    } else {
        p.consume(); // eat integer
    }

    m.complete(p, SyntaxKind::TupleAccessExpression)
}

/// Parses index or slice: `expr[idx]` or `expr[start..end]`
pub(crate) fn index_or_slice_expression(
    p: &mut Parser,
    collection: CompletedMarker,
) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftBracket));

    let m = collection.precede(p);
    p.consume(); // eat '['
    p.enter_delimiter();

    // Check for slice patterns:
    // [..end]  - from start
    // [start..]  - to end
    // [start..end] - range
    // [idx] - single index

    let is_slice = if p.at(TokenKind::DotDot) {
        // [..end] or [..]
        p.consume(); // eat '..'
        if !p.at(TokenKind::RightBracket) {
            expression(p); // end
        }
        true
    } else if !p.at(TokenKind::RightBracket) {
        // Has start expression
        expression(p);
        if p.at(TokenKind::DotDot) {
            // [start..] or [start..end]
            p.consume(); // eat '..'
            if !p.at(TokenKind::RightBracket) {
                expression(p); // end
            }
            true
        } else {
            // [idx]
            false
        }
    } else {
        // [] - empty, treat as error but complete
        false
    };

    p.exit_delimiter();
    if !p.eat(TokenKind::RightBracket) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "']'".to_string(),
            found,
        });
    }

    let kind = if is_slice {
        SyntaxKind::SliceExpression
    } else {
        SyntaxKind::IndexExpression
    };
    m.complete(p, kind)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    // Int arithmetic
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,
    // Float arithmetic
    PlusDot,
    MinusDot,
    AsteriskDot,
    SlashDot,
    // Comparison
    EqualsEquals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    // Float comparison
    GreaterThanDot,
    LessThanDot,
    GreaterThanOrEqualDot,
    LessThanOrEqualDot,
    // Boolean
    And,
    Or,
    Bang,
}

impl TryFrom<TokenKind> for Operator {
    type Error = TokenKind;

    fn try_from(value: TokenKind) -> Result<Self, TokenKind> {
        match value {
            // Int arithmetic
            TokenKind::Plus => Ok(Self::Plus),
            TokenKind::Minus => Ok(Self::Minus),
            TokenKind::Asterisk => Ok(Self::Asterisk),
            TokenKind::Slash => Ok(Self::Slash),
            TokenKind::Percent => Ok(Self::Percent),
            // Float arithmetic
            TokenKind::PlusDot => Ok(Self::PlusDot),
            TokenKind::MinusDot => Ok(Self::MinusDot),
            TokenKind::AsteriskDot => Ok(Self::AsteriskDot),
            TokenKind::SlashDot => Ok(Self::SlashDot),
            // Comparison
            TokenKind::EqualsEquals => Ok(Self::EqualsEquals),
            TokenKind::NotEquals => Ok(Self::NotEquals),
            TokenKind::GreaterThan => Ok(Self::GreaterThan),
            TokenKind::LessThan => Ok(Self::LessThan),
            TokenKind::GreaterThanOrEqual => Ok(Self::GreaterThanOrEqual),
            TokenKind::LessThanOrEqual => Ok(Self::LessThanOrEqual),
            // Float comparison
            TokenKind::GreaterThanDot => Ok(Self::GreaterThanDot),
            TokenKind::LessThanDot => Ok(Self::LessThanDot),
            TokenKind::GreaterThanOrEqualDot => Ok(Self::GreaterThanOrEqualDot),
            TokenKind::LessThanOrEqualDot => Ok(Self::LessThanOrEqualDot),
            // Boolean
            TokenKind::And => Ok(Self::And),
            TokenKind::Or => Ok(Self::Or),
            TokenKind::Bang => Ok(Self::Bang),
            _ => Err(value),
        }
    }
}

impl Operator {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Asterisk => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::PlusDot => "+.",
            Self::MinusDot => "-.",
            Self::AsteriskDot => "*.",
            Self::SlashDot => "/.",
            Self::EqualsEquals => "==",
            Self::NotEquals => "!=",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThanDot => ">.",
            Self::LessThanDot => "<.",
            Self::GreaterThanOrEqualDot => ">=.",
            Self::LessThanOrEqualDot => "<=.",
            Self::And => "and",
            Self::Or => "or",
            Self::Bang => "!",
        }
    }

    pub fn prefix_binding_power(&self) -> Option<((), u8)> {
        let result = match &self {
            Self::Bang | Self::Minus => ((), 15),
            _ => return None,
        };
        Some(result)
    }

    pub fn infix_binding_power(&self) -> Option<(u8, u8)> {
        // Binding powers (higher = tighter binding):
        // 1-2: or
        // 3-4: and
        // 5-6: equality (==, !=)
        // 7-8: comparison (>, <, >=, <=, and float variants)
        // 9-10: additive (+, -, +., -.)
        // 11-12: multiplicative (*, /, %, *., /.)
        let result = match &self {
            // Boolean - lowest precedence
            Self::Or => (1, 2),
            Self::And => (3, 4),
            // Equality
            Self::EqualsEquals | Self::NotEquals => (5, 6),
            // Comparison (int and float)
            Self::GreaterThan
            | Self::LessThan
            | Self::GreaterThanOrEqual
            | Self::LessThanOrEqual
            | Self::GreaterThanDot
            | Self::LessThanDot
            | Self::GreaterThanOrEqualDot
            | Self::LessThanOrEqualDot => (7, 8),
            // Additive (int and float)
            Self::Plus | Self::Minus | Self::PlusDot | Self::MinusDot => (9, 10),
            // Multiplicative (int and float)
            Self::Asterisk | Self::Slash | Self::Percent | Self::AsteriskDot | Self::SlashDot => {
                (11, 12)
            }
            // Bang is prefix-only
            Self::Bang => return None,
        };
        Some(result)
    }
}
