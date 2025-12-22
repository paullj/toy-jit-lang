use hir::HirDiagnostic;
use infer::InferDiagnostic;
use miette::{Diagnostic, SourceSpan};
use parse::ParseError;
use std::fmt;

use crate::Range;
use crate::line_index::LineIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Unified diagnostic type wrapping all diagnostic sources.
#[derive(Debug, Clone)]
pub enum AnalyseDiagnostic {
    Parse(ParseError),
    Hir(HirDiagnostic),
    Infer(InferDiagnostic),
}

impl fmt::Display for AnalyseDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{}", e),
            Self::Hir(e) => write!(f, "{}", e),
            Self::Infer(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for AnalyseDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => e.source(),
            Self::Hir(e) => e.source(),
            Self::Infer(e) => e.source(),
        }
    }
}

impl Diagnostic for AnalyseDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match self {
            Self::Parse(e) => e.code(),
            Self::Hir(e) => e.code(),
            Self::Infer(e) => e.code(),
        }
    }

    fn severity(&self) -> Option<miette::Severity> {
        match self {
            Self::Parse(e) => e.severity(),
            Self::Hir(e) => e.severity(),
            Self::Infer(e) => e.severity(),
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match self {
            Self::Parse(e) => e.help(),
            Self::Hir(e) => e.help(),
            Self::Infer(e) => e.help(),
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        match self {
            Self::Parse(e) => e.labels(),
            Self::Hir(e) => e.labels(),
            Self::Infer(e) => e.labels(),
        }
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        match self {
            Self::Parse(e) => e.related(),
            Self::Hir(e) => e.related(),
            Self::Infer(e) => e.related(),
        }
    }

    fn url<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match self {
            Self::Parse(e) => e.url(),
            Self::Hir(e) => e.url(),
            Self::Infer(e) => e.url(),
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match self {
            Self::Parse(e) => e.source_code(),
            Self::Hir(e) => e.source_code(),
            Self::Infer(e) => e.source_code(),
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        match self {
            Self::Parse(e) => e.diagnostic_source(),
            Self::Hir(e) => e.diagnostic_source(),
            Self::Infer(e) => e.diagnostic_source(),
        }
    }
}

impl From<ParseError> for AnalyseDiagnostic {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<HirDiagnostic> for AnalyseDiagnostic {
    fn from(e: HirDiagnostic) -> Self {
        Self::Hir(e)
    }
}

impl From<InferDiagnostic> for AnalyseDiagnostic {
    fn from(e: InferDiagnostic) -> Self {
        Self::Infer(e)
    }
}

// Helper to get the primary span from a diagnostic
fn get_primary_span(diag: &AnalyseDiagnostic) -> Option<SourceSpan> {
    diag.labels()?.next().map(|l| *l.inner())
}

fn source_span_to_range(span: SourceSpan, line_index: &LineIndex) -> Range {
    let start = span.offset() as u32;
    let end = start + span.len() as u32;
    Range::new(line_index.position(start), line_index.position(end))
}

fn miette_to_severity(sev: Option<miette::Severity>) -> DiagnosticSeverity {
    match sev {
        Some(miette::Severity::Error) | None => DiagnosticSeverity::Error,
        Some(miette::Severity::Warning) => DiagnosticSeverity::Warning,
        Some(miette::Severity::Advice) => DiagnosticSeverity::Hint,
    }
}

/// Convert an AnalyseDiagnostic to LSP-compatible fields
pub fn to_lsp_fields(
    diag: &AnalyseDiagnostic,
    line_index: &LineIndex,
) -> (
    Range,
    DiagnosticSeverity,
    Option<String>,
    String,
    Option<String>,
) {
    let range = get_primary_span(diag)
        .map(|s| source_span_to_range(s, line_index))
        .unwrap_or_default();
    let severity = miette_to_severity(diag.severity());
    let code = diag.code().map(|c| c.to_string());
    let message = diag.to_string();
    let help = diag.help().map(|h| h.to_string());
    (range, severity, code, message, help)
}
