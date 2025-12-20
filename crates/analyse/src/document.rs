use ast::AstNode;
use hir::LowerResult;
use infer::InferenceResult;
use parse::ParseError;

use crate::diagnostics::{Diagnostic, convert_infer_diagnostic, convert_parse_error};
use crate::line_index::LineIndex;

/// Cached analysis state for source text.
#[derive(Debug)]
pub struct Document {
    pub text: String,
    pub line_index: LineIndex,
    pub parse_errors: Vec<ParseError>,
    pub lower_result: Option<LowerResult>,
    pub infer_result: Option<InferenceResult>,
}

impl Document {
    /// Analyze source text and cache all results.
    pub fn new(text: String) -> Self {
        let line_index = LineIndex::new(&text);

        let (syntax, parse_errors) = parse::parse(&text);

        let root = ast::Root::cast(syntax);
        let lower_result = root.map(hir::lower);

        let infer_result = lower_result.as_ref().map(infer::infer);

        Self {
            text,
            line_index,
            parse_errors,
            lower_result,
            infer_result,
        }
    }

    /// Get all diagnostics (parse + type errors).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics: Vec<_> = self
            .parse_errors
            .iter()
            .map(|e| convert_parse_error(e, &self.line_index))
            .collect();

        if let Some(ref infer) = self.infer_result {
            diagnostics.extend(
                infer
                    .diagnostics
                    .iter()
                    .map(|e| convert_infer_diagnostic(e, &self.line_index)),
            );
        }

        diagnostics
    }
}
