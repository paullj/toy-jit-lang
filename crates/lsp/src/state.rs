use std::collections::HashMap;

use ast::AstNode;
use hir::LowerResult;
use infer::InferenceResult;
use parse::ParseError;
use tower_lsp::lsp_types::Url;

// Note: We don't use lex directly - parse::parse handles lexing internally

use crate::line_index::LineIndex;

/// Cached analysis state for a single document.
/// Note: We don't store SyntaxNode because rowan nodes are not Send/Sync.
#[derive(Debug)]
#[allow(dead_code)]
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

        // Parse (includes lexing internally)
        let (syntax, parse_errors) = parse::parse(&text);

        // Lower to HIR (don't store syntax - it's not Send/Sync)
        let root = ast::Root::cast(syntax);
        let lower_result = root.map(hir::lower);

        // Type inference
        let infer_result = lower_result.as_ref().map(infer::infer);

        Self {
            text,
            line_index,
            parse_errors,
            lower_result,
            infer_result,
        }
    }
}

/// Global state for the LSP server.
#[derive(Debug, Default)]
pub struct State {
    pub documents: HashMap<Url, Document>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, uri: Url, text: String) {
        self.documents.insert(uri, Document::new(text));
    }

    pub fn change(&mut self, uri: &Url, text: String) {
        self.documents.insert(uri.clone(), Document::new(text));
    }

    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }
}
