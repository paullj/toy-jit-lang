mod event;
mod grammar;
mod marker;

#[macro_use]
mod utils;

pub mod error;
pub mod parser;

use event::{Sink, Source};
use syntax::SyntaxNode;

pub use error::ParseError;
pub use parser::Parser;

pub fn parse(input: &str) -> (SyntaxNode, Vec<ParseError>) {
    let source = Source::new(input);
    let parser = Parser::new(source);

    let (events, lexer_errors) = parser.parse();

    let sink = Sink::new();
    let (green_node, mut parse_errors) = sink.build(events);

    // Convert lexer errors to parse errors and include them
    for lexer_error in lexer_errors {
        let parse_error = ParseError::from(lexer_error);
        parse_errors.push(parse_error);
    }

    (SyntaxNode::new_root(green_node), parse_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{Expect, expect};

    fn check(input: &str, expected_tree: Expect) {
        let (tree, errors) = parse(input);
        let actual = format!("{tree:#?}");
        expected_tree.assert_eq(&actual);
        assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
    }

    #[test]
    fn parse_variable_definition_inferred() {
        check(
            "x := 10",
            expect![[r#"
                Root@0..7
                  VariableDefinition@0..7
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    Literal@4..7
                      Whitespace@4..5 " "
                      Integer@5..7 "10"
            "#]],
        );
    }

    #[test]
    fn parse_variable_definition_typed() {
        check(
            "x: int = 10",
            expect![[r#"
                Root@0..11
                  VariableDefinition@0..11
                    Identifier@0..1 "x"
                    Colon@1..2 ":"
                    TypeAnnotation@2..6
                      Whitespace@2..3 " "
                      Identifier@3..6 "int"
                    Whitespace@6..7 " "
                    Equals@7..8 "="
                    Literal@8..11
                      Whitespace@8..9 " "
                      Integer@9..11 "10"
            "#]],
        );
    }

    #[test]
    fn parse_variable_assignment() {
        check(
            "x = 42",
            expect![[r#"
                Root@0..6
                  VariableAssignment@0..6
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Equals@2..3 "="
                    Literal@3..6
                      Whitespace@3..4 " "
                      Integer@4..6 "42"
            "#]],
        );
    }

    #[test]
    fn parse_infix_expression() {
        check(
            "x := 1 + 2",
            expect![[r#"
                Root@0..10
                  VariableDefinition@0..10
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    InfixExpression@4..10
                      Literal@4..6
                        Whitespace@4..5 " "
                        Integer@5..6 "1"
                      Whitespace@6..7 " "
                      Plus@7..8 "+"
                      Literal@8..10
                        Whitespace@8..9 " "
                        Integer@9..10 "2"
            "#]],
        );
    }

    #[test]
    fn parse_operator_precedence() {
        check(
            "x := 1 + 2 * 3",
            expect![[r#"
                Root@0..14
                  VariableDefinition@0..14
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    InfixExpression@4..14
                      Literal@4..6
                        Whitespace@4..5 " "
                        Integer@5..6 "1"
                      Whitespace@6..7 " "
                      Plus@7..8 "+"
                      InfixExpression@8..14
                        Literal@8..10
                          Whitespace@8..9 " "
                          Integer@9..10 "2"
                        Whitespace@10..11 " "
                        Asterisk@11..12 "*"
                        Literal@12..14
                          Whitespace@12..13 " "
                          Integer@13..14 "3"
            "#]],
        );
    }

    #[test]
    fn parse_parenthesis_expression() {
        check(
            "x := (1 + 2) * 3",
            expect![[r#"
                Root@0..16
                  VariableDefinition@0..16
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    InfixExpression@4..16
                      ParenthesisExpression@4..12
                        Whitespace@4..5 " "
                        LeftParenthesis@5..6 "("
                        InfixExpression@6..11
                          Literal@6..7
                            Integer@6..7 "1"
                          Whitespace@7..8 " "
                          Plus@8..9 "+"
                          Literal@9..11
                            Whitespace@9..10 " "
                            Integer@10..11 "2"
                        RightParenthesis@11..12 ")"
                      Whitespace@12..13 " "
                      Asterisk@13..14 "*"
                      Literal@14..16
                        Whitespace@14..15 " "
                        Integer@15..16 "3"
            "#]],
        );
    }

    #[test]
    fn parse_prefix_expression() {
        check(
            "x := -5",
            expect![[r#"
                Root@0..7
                  VariableDefinition@0..7
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    PrefixExpression@4..7
                      Whitespace@4..5 " "
                      Minus@5..6 "-"
                      Literal@6..7
                        Integer@6..7 "5"
            "#]],
        );
    }

    #[test]
    fn parse_boolean_literal() {
        check(
            "x := true",
            expect![[r#"
                Root@0..9
                  VariableDefinition@0..9
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    Literal@4..9
                      Whitespace@4..5 " "
                      TrueKeyword@5..9 "true"
            "#]],
        );
    }

    #[test]
    fn parse_variable_reference() {
        check(
            "x = y",
            expect![[r#"
                Root@0..5
                  VariableAssignment@0..5
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Equals@2..3 "="
                    VariableReference@3..5
                      Whitespace@3..4 " "
                      Identifier@4..5 "y"
            "#]],
        );
    }

    #[test]
    fn parse_float_operators() {
        check(
            "x := 1.0 +. 2.0",
            expect![[r#"
                Root@0..15
                  VariableDefinition@0..15
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    InfixExpression@4..15
                      Literal@4..8
                        Whitespace@4..5 " "
                        Float@5..8 "1.0"
                      Whitespace@8..9 " "
                      PlusDot@9..11 "+."
                      Literal@11..15
                        Whitespace@11..12 " "
                        Float@12..15 "2.0"
            "#]],
        );
    }

    #[test]
    fn parse_comparison() {
        check(
            "x := a > b",
            expect![[r#"
                Root@0..10
                  VariableDefinition@0..10
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    InfixExpression@4..10
                      VariableReference@4..6
                        Whitespace@4..5 " "
                        Identifier@5..6 "a"
                      Whitespace@6..7 " "
                      GreaterThan@7..8 ">"
                      VariableReference@8..10
                        Whitespace@8..9 " "
                        Identifier@9..10 "b"
            "#]],
        );
    }

    #[test]
    fn parse_boolean_operators() {
        check(
            "x := a and b or c",
            expect![[r#"
                Root@0..17
                  VariableDefinition@0..17
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    InfixExpression@4..17
                      InfixExpression@4..12
                        VariableReference@4..6
                          Whitespace@4..5 " "
                          Identifier@5..6 "a"
                        Whitespace@6..7 " "
                        AndKeyword@7..10 "and"
                        VariableReference@10..12
                          Whitespace@10..11 " "
                          Identifier@11..12 "b"
                      Whitespace@12..13 " "
                      OrKeyword@13..15 "or"
                      VariableReference@15..17
                        Whitespace@15..16 " "
                        Identifier@16..17 "c"
            "#]],
        );
    }

    #[test]
    fn parse_invalid_character() {
        let (tree, errors) = parse("x := @");
        let actual = format!("{tree:#?}");
        expect![[r#"
            Root@0..4
              VariableDefinition@0..4
                Identifier@0..1 "x"
                Whitespace@1..2 " "
                Colon@2..3 ":"
                Equals@3..4 "="
        "#]]
        .assert_eq(&actual);
        assert!(!errors.is_empty(), "Should have errors for invalid char");
    }

    #[test]
    fn parse_multiple_statements() {
        check(
            "x := 1\ny := 2",
            expect![[r#"
                Root@0..13
                  VariableDefinition@0..6
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    Literal@4..6
                      Whitespace@4..5 " "
                      Integer@5..6 "1"
                  VariableDefinition@6..13
                    NewLine@6..7 "\n"
                    Identifier@7..8 "y"
                    Whitespace@8..9 " "
                    Colon@9..10 ":"
                    Equals@10..11 "="
                    Literal@11..13
                      Whitespace@11..12 " "
                      Integer@12..13 "2"
            "#]],
        );
    }

    #[test]
    fn parse_string_literal() {
        check(
            r#"x := "hello""#,
            expect![[r#"
                Root@0..12
                  VariableDefinition@0..12
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    Literal@4..12
                      Whitespace@4..5 " "
                      String@5..12 "\"hello\""
            "#]],
        );
    }

    #[test]
    fn parse_bang_prefix() {
        check(
            "x := !true",
            expect![[r#"
                Root@0..10
                  VariableDefinition@0..10
                    Identifier@0..1 "x"
                    Whitespace@1..2 " "
                    Colon@2..3 ":"
                    Equals@3..4 "="
                    PrefixExpression@4..10
                      Whitespace@4..5 " "
                      Bang@5..6 "!"
                      Literal@6..10
                        TrueKeyword@6..10 "true"
            "#]],
        );
    }
}
