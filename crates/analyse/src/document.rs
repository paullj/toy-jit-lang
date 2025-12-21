use ast::AstNode;
use hir::{LowerResult, Symbol};
use infer::{InferenceResult, Type};
use parse::ParseError;

use crate::diagnostics::{Diagnostic, convert_infer_diagnostic, convert_parse_error};
use crate::line_index::{LineIndex, Position};

/// Cached analysis state for source text.
///
/// Currently uses full reparse on each change. Future incremental improvements:
/// - Store GreenNode for efficient tree sharing
/// - Track which items are affected by edits
/// - Re-lower/re-infer only changed items
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

    /// Find symbol at a given LSP position.
    pub fn symbol_at(&self, pos: Position) -> Option<&Symbol> {
        let offset = self.line_index.offset(pos);
        self.lower_result.as_ref()?.symbols.symbol_at(offset)
    }

    /// Get the type of a symbol (if type inference succeeded).
    pub fn type_of_symbol(&self, symbol: &Symbol) -> Option<&Type> {
        self.infer_result.as_ref()?.get_variable_type(&symbol.name)
    }

    /// Get type at position (convenience combining symbol_at + type_of_symbol).
    pub fn type_at(&self, pos: Position) -> Option<&Type> {
        let symbol = self.symbol_at(pos)?;
        self.type_of_symbol(symbol)
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
