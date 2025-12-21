use std::collections::HashMap;
use std::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::convert::{from_lsp_position, to_lsp_diagnostic, to_lsp_range};
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
        let diagnostics = {
            let state = self.state.read().unwrap();
            let Some(doc) = state.get(&uri) else {
                return;
            };

            doc.diagnostics()
                .iter()
                .map(to_lsp_diagnostic)
                .collect::<Vec<_>>()
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
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
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

        {
            let mut state = self.state.write().unwrap();
            for change in params.content_changes {
                if let Some(range) = change.range {
                    // Incremental change
                    state.change_incremental(&uri, range, &change.text);
                } else {
                    // Full change (fallback)
                    state.change_full(&uri, change.text);
                }
            }
        }
        self.publish_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        {
            let mut state = self.state.write().unwrap();
            state.close(&uri);
        }

        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = from_lsp_position(params.text_document_position_params.position);

        let state = self.state.read().unwrap();
        let Some(doc) = state.get(uri) else {
            return Ok(None);
        };

        let Some(symbol) = doc.symbol_at(pos) else {
            return Ok(None);
        };

        let ty = doc.type_of_symbol(symbol);
        let ty_str = ty
            .map(|t| format!("{t}"))
            .unwrap_or_else(|| "unknown".to_string());
        let content = format!("{}: {}", symbol.name, ty_str);

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(content)),
            range: Some(to_lsp_range(doc.line_index.range(symbol.name_span))),
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = from_lsp_position(params.text_document_position_params.position);

        let state = self.state.read().unwrap();
        let Some(doc) = state.get(&uri) else {
            return Ok(None);
        };

        let Some(symbol) = doc.symbol_at(pos) else {
            return Ok(None);
        };

        let range = to_lsp_range(doc.line_index.range(symbol.name_span));
        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            uri, range,
        ))))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = from_lsp_position(params.text_document_position.position);
        let include_declaration = params.context.include_declaration;

        let state = self.state.read().unwrap();
        let Some(doc) = state.get(&uri) else {
            return Ok(None);
        };

        let Some(symbol) = doc.symbol_at(pos) else {
            return Ok(None);
        };

        let mut locations: Vec<Location> = symbol
            .references
            .iter()
            .map(|&span| Location::new(uri.clone(), to_lsp_range(doc.line_index.range(span))))
            .collect();

        if include_declaration {
            locations.insert(
                0,
                Location::new(
                    uri.clone(),
                    to_lsp_range(doc.line_index.range(symbol.name_span)),
                ),
            );
        }

        Ok(Some(locations))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = from_lsp_position(params.text_document_position.position);
        let new_name = params.new_name;

        let state = self.state.read().unwrap();
        let Some(doc) = state.get(&uri) else {
            return Ok(None);
        };

        let Some(symbol) = doc.symbol_at(pos) else {
            return Ok(None);
        };

        // Collect all locations where the symbol appears
        let mut edits: Vec<TextEdit> = Vec::new();

        // Definition
        edits.push(TextEdit::new(
            to_lsp_range(doc.line_index.range(symbol.name_span)),
            new_name.clone(),
        ));

        // References
        for &span in &symbol.references {
            edits.push(TextEdit::new(
                to_lsp_range(doc.line_index.range(span)),
                new_name.clone(),
            ));
        }

        let mut changes = HashMap::new();
        changes.insert(uri, edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }
}
