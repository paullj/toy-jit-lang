use std::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::diagnostics::{infer_diagnostic_to_diagnostic, parse_error_to_diagnostic};
use crate::state::State;

pub struct Backend {
    client: Client,
    state: RwLock<State>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: RwLock::new(State::new()),
        }
    }

    async fn publish_diagnostics(&self, uri: Url) {
        // Collect diagnostics while holding lock, then release before await
        let diagnostics = {
            let state = self.state.read().unwrap();
            let Some(doc) = state.get(&uri) else {
                return;
            };

            let mut diagnostics = Vec::new();

            // Parse errors
            for err in &doc.parse_errors {
                diagnostics.push(parse_error_to_diagnostic(err, &doc.line_index));
            }

            // Type errors
            if let Some(infer) = &doc.infer_result {
                for err in &infer.diagnostics {
                    diagnostics.push(infer_diagnostic_to_diagnostic(err, &doc.line_index));
                }
            }

            diagnostics
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "toy-lang LSP initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.client
            .log_message(MessageType::INFO, format!("Opened: {}", uri))
            .await;

        let text = params.text_document.text;
        {
            let mut state = self.state.write().unwrap();
            state.open(uri.clone(), text);
        }

        self.publish_diagnostics(uri).await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // We use TextDocumentSyncKind::FULL, so there's exactly one change with full content
        if let Some(change) = params.content_changes.into_iter().next() {
            {
                let mut state = self.state.write().unwrap();
                state.change(&uri, change.text);
            }
            self.publish_diagnostics(uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        {
            let mut state = self.state.write().unwrap();
            state.close(&uri);
        }

        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}
