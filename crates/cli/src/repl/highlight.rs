use lex::TokenKind;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Get the style for a token kind.
pub fn token_style(kind: TokenKind) -> Style {
    match kind {
        // Numbers
        TokenKind::Integer
        | TokenKind::Float
        | TokenKind::FloatExponent
        | TokenKind::BinaryInteger
        | TokenKind::OctalInteger
        | TokenKind::HexInteger => Style::default().fg(Color::Yellow),

        // Strings
        TokenKind::String | TokenKind::MultiLineString => Style::default().fg(Color::Green),

        // Booleans and logical keywords
        TokenKind::True | TokenKind::False | TokenKind::And | TokenKind::Or => {
            Style::default().fg(Color::Magenta)
        }

        // Operators
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Asterisk
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::PlusDot
        | TokenKind::MinusDot
        | TokenKind::AsteriskDot
        | TokenKind::SlashDot
        | TokenKind::Equals
        | TokenKind::EqualsEquals
        | TokenKind::NotEquals
        | TokenKind::Bang
        | TokenKind::GreaterThan
        | TokenKind::LessThan
        | TokenKind::GreaterThanOrEqual
        | TokenKind::LessThanOrEqual
        | TokenKind::GreaterThanDot
        | TokenKind::LessThanDot
        | TokenKind::GreaterThanOrEqualDot
        | TokenKind::LessThanOrEqualDot => Style::default().fg(Color::Cyan),

        // Punctuation
        TokenKind::LeftParenthesis
        | TokenKind::RightParenthesis
        | TokenKind::Comma
        | TokenKind::Colon => Style::default().fg(Color::DarkGray),

        // Comments
        TokenKind::Comment => Style::default().fg(Color::DarkGray),

        // Identifiers
        TokenKind::Identifier => Style::default().fg(Color::White),

        // Trivia - keep default
        TokenKind::Whitespace | TokenKind::NewLine => Style::default(),
    }
}

/// Highlight source text and return styled spans.
pub fn highlight(text: &str) -> Vec<Span<'static>> {
    let tokens = lex::lex(text);
    tokens
        .into_iter()
        .filter_map(|result| result.ok())
        .map(|token| {
            let style = token_style(token.kind);
            Span::styled(token.text.to_string(), style)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_simple() {
        let spans = highlight("x := 42");
        assert_eq!(spans.len(), 6); // x, space, :, =, space, 42
    }

    #[test]
    fn test_token_style_numbers() {
        assert_eq!(token_style(TokenKind::Integer).fg, Some(Color::Yellow));
        assert_eq!(token_style(TokenKind::Float).fg, Some(Color::Yellow));
    }

    #[test]
    fn test_token_style_strings() {
        assert_eq!(token_style(TokenKind::String).fg, Some(Color::Green));
    }

    #[test]
    fn test_token_style_keywords() {
        assert_eq!(token_style(TokenKind::True).fg, Some(Color::Magenta));
        assert_eq!(token_style(TokenKind::False).fg, Some(Color::Magenta));
    }
}
