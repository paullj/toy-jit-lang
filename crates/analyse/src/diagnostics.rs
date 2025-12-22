use infer::InferDiagnostic;
use miette::SourceSpan;
use parse::ParseError;

use crate::Range;
use crate::line_index::LineIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub help: Option<String>,
}

fn source_span_to_range(span: SourceSpan, line_index: &LineIndex) -> Range {
    let start = span.offset() as u32;
    let end = start + span.len() as u32;
    Range::new(line_index.position(start), line_index.position(end))
}

pub fn convert_parse_error(error: &ParseError, line_index: &LineIndex) -> Diagnostic {
    match error {
        ParseError::UnexpectedToken {
            at,
            expected,
            found,
        } => {
            let msg = match found {
                Some(f) => format!("unexpected `{f}`, expected {expected}"),
                None => format!("unexpected token, expected {expected}"),
            };
            Diagnostic {
                range: source_span_to_range(*at, line_index),
                severity: DiagnosticSeverity::Error,
                code: "parse:unexpected_token".into(),
                message: msg,
                help: None,
            }
        }
        ParseError::ExpectedVariableItem { at, expected } => {
            let msg = match expected {
                Some(e) => format!("expected variable definition, assignment, or expression. {e}"),
                None => "expected variable definition, assignment, or expression".into(),
            };
            Diagnostic {
                range: source_span_to_range(*at, line_index),
                severity: DiagnosticSeverity::Error,
                code: "parse:expected_variable_item".into(),
                message: msg,
                help: None,
            }
        }
        ParseError::IncompleteExpression { at, operator } => Diagnostic {
            range: source_span_to_range(*at, line_index),
            severity: DiagnosticSeverity::Error,
            code: "parse:incomplete_expression".into(),
            message: format!("incomplete expression after `{operator}`"),
            help: Some("did you forget another operand?".into()),
        },
        ParseError::SuggestDefinition { at, name, hint } => Diagnostic {
            range: source_span_to_range(*at, line_index),
            severity: DiagnosticSeverity::Error,
            code: "parse:suggest_definition".into(),
            message: format!("cannot assign to undefined variable `{name}`"),
            help: Some(hint.clone()),
        },
        ParseError::UnnecessarySemicolon { at } => Diagnostic {
            range: source_span_to_range(*at, line_index),
            severity: DiagnosticSeverity::Error,
            code: "parse:unnecessary_semicolon".into(),
            message: "unexpected semicolon".into(),
            help: Some("semicolons are not needed - statements are separated by newlines".into()),
        },
    }
}

pub fn convert_infer_diagnostic(error: &InferDiagnostic, line_index: &LineIndex) -> Diagnostic {
    match error {
        InferDiagnostic::Mismatch {
            expected,
            found,
            span,
        } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:type_mismatch".into(),
            message: format!("type mismatch: expected `{expected}`, found `{found}`"),
            help: None,
        },
        InferDiagnostic::Undefined {
            name,
            span,
            suggestion,
        } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:undefined".into(),
            message: format!("undefined variable `{name}`"),
            help: suggestion.clone(),
        },
        InferDiagnostic::InfiniteType { var, ty, span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:infinite_type".into(),
            message: format!("infinite type: `{var}` occurs in `{ty}`"),
            help: None,
        },
        InferDiagnostic::ArityMismatch {
            expected,
            found,
            span,
        } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:arity_mismatch".into(),
            message: format!("arity mismatch: expected {expected} argument(s), found {found}"),
            help: None,
        },
        InferDiagnostic::WrongOperator {
            op,
            expected_type,
            actual_type,
            suggest_op,
            span,
        } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:wrong_operator".into(),
            message: format!(
                "operator `{op}` is for `{expected_type}`, but operands are `{actual_type}`"
            ),
            help: Some(format!("use `{suggest_op}` for `{actual_type}` operations")),
        },
        InferDiagnostic::DivisionByZero { span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:division_by_zero".into(),
            message: "division by zero".into(),
            help: None,
        },
        InferDiagnostic::OverflowLiteral { value, ty, span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:overflow_literal".into(),
            message: format!("literal `{value}` overflows type `{ty}`"),
            help: None,
        },
        InferDiagnostic::EmptyBlockAssignment { span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:empty_block_assignment".into(),
            message: "assigning empty block to variable".into(),
            help: Some("empty blocks have type `()` (unit). Did you mean to have an expression in the block?".into()),
        },
    }
}
