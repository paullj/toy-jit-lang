use infer::InferDiagnostic;
use miette::SourceSpan;
use parse::ParseError;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::line_index::LineIndex;

/// Convert a SourceSpan (offset, len) to LSP Range.
fn source_span_to_range(span: SourceSpan, line_index: &LineIndex) -> Range {
    let start = span.offset() as u32;
    let end = start + span.len() as u32;
    Range::new(line_index.position(start), line_index.position(end))
}

/// Convert a ParseError to LSP Diagnostic.
pub fn parse_error_to_diagnostic(error: &ParseError, line_index: &LineIndex) -> Diagnostic {
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
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(
                    "parse:unexpected_token".into(),
                )),
                message: msg,
                ..Default::default()
            }
        }
        ParseError::ExpectedVariableItem { at, expected } => {
            let msg = match expected {
                Some(e) => format!("expected variable definition, assignment, or expression. {e}"),
                None => "expected variable definition, assignment, or expression".into(),
            };
            Diagnostic {
                range: source_span_to_range(*at, line_index),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(
                    "parse:expected_variable_item".into(),
                )),
                message: msg,
                ..Default::default()
            }
        }
    }
}

/// Convert an InferDiagnostic to LSP Diagnostic.
pub fn infer_diagnostic_to_diagnostic(
    error: &InferDiagnostic,
    line_index: &LineIndex,
) -> Diagnostic {
    match error {
        InferDiagnostic::Mismatch {
            expected,
            found,
            span,
        } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                "infer:type_mismatch".into(),
            )),
            message: format!("type mismatch: expected `{expected}`, found `{found}`"),
            ..Default::default()
        },
        InferDiagnostic::Undefined { name, span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                "infer:undefined".into(),
            )),
            message: format!("undefined variable `{name}`"),
            ..Default::default()
        },
        InferDiagnostic::InfiniteType { var, ty, span } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                "infer:infinite_type".into(),
            )),
            message: format!("infinite type: `{var}` occurs in `{ty}`"),
            ..Default::default()
        },
        InferDiagnostic::ArityMismatch {
            expected,
            found,
            span,
        } => Diagnostic {
            range: source_span_to_range(*span, line_index),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                "infer:arity_mismatch".into(),
            )),
            message: format!("arity mismatch: expected {expected} argument(s), found {found}"),
            ..Default::default()
        },
    }
}
