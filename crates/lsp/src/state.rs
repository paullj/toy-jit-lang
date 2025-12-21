use std::collections::HashMap;

use analyse::Document;
use tower_lsp::lsp_types::{Range, Url};

use crate::convert::from_lsp_position;

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

    /// Apply a full text change (replaces entire document).
    pub fn change_full(&mut self, uri: &Url, text: String) {
        self.documents.insert(uri.clone(), Document::new(text));
    }

    /// Apply an incremental text change.
    pub fn change_incremental(&mut self, uri: &Url, range: Range, text: &str) {
        if let Some(doc) = self.documents.get(uri) {
            let mut new_text = doc.text.clone();
            let start = doc.line_index.offset(from_lsp_position(range.start)) as usize;
            let end = doc.line_index.offset(from_lsp_position(range.end)) as usize;
            new_text.replace_range(start..end, text);
            self.documents.insert(uri.clone(), Document::new(new_text));
        }
    }

    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }
}
