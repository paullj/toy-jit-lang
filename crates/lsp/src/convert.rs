use analyse::{Diagnostic, DiagnosticSeverity, Position, Range};
use tower_lsp::lsp_types;

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

pub fn to_lsp_diagnostic(diag: &Diagnostic) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: to_lsp_range(diag.range),
        severity: Some(to_lsp_severity(diag.severity)),
        code: Some(lsp_types::NumberOrString::String(diag.code.clone())),
        message: diag.message.clone(),
        ..Default::default()
    }
}
