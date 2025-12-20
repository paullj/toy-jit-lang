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
            }
        }
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
        },
        InferDiagnostic::Undefined { name, span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:undefined".into(),
            message: format!("undefined variable `{name}`"),
        },
        InferDiagnostic::InfiniteType { var, ty, span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: DiagnosticSeverity::Error,
            code: "infer:infinite_type".into(),
            message: format!("infinite type: `{var}` occurs in `{ty}`"),
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
        },
    }
}
