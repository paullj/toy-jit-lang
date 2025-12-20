use std::collections::HashMap;

use analyse::Document;
use tower_lsp::lsp_types::Url;

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
