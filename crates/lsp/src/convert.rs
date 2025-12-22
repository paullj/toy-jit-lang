use analyse::{AnalyseDiagnostic, DiagnosticSeverity, LineIndex, Position, Range, to_lsp_fields};
use tower_lsp::lsp_types;

pub fn from_lsp_position(pos: lsp_types::Position) -> Position {
    Position::new(pos.line, pos.character)
}

pub fn to_lsp_position(pos: Position) -> lsp_types::Position {
    lsp_types::Position::new(pos.line, pos.character)
}

pub fn to_lsp_range(range: Range) -> lsp_types::Range {
    lsp_types::Range::new(to_lsp_position(range.start), to_lsp_position(range.end))
}

pub fn to_lsp_severity(severity: DiagnosticSeverity) -> lsp_types::DiagnosticSeverity {
    match severity {
        DiagnosticSeverity::Error => lsp_types::DiagnosticSeverity::ERROR,
        DiagnosticSeverity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        DiagnosticSeverity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
        DiagnosticSeverity::Hint => lsp_types::DiagnosticSeverity::HINT,
    }
}

pub fn to_lsp_diagnostic(
    diag: &AnalyseDiagnostic,
    line_index: &LineIndex,
) -> lsp_types::Diagnostic {
    let (range, severity, code, message, _help) = to_lsp_fields(diag, line_index);
    lsp_types::Diagnostic {
        range: to_lsp_range(range),
        severity: Some(to_lsp_severity(severity)),
        code: code.map(lsp_types::NumberOrString::String),
        message,
        ..Default::default()
    }
}
