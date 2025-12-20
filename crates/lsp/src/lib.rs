mod backend;
mod convert;
mod state;

use tower_lsp::{LspService, Server};

use backend::Backend;

/// Run the LSP server on stdin/stdout.
/// This blocks until the client disconnects.
pub fn run() -> miette::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();

            let (service, socket) = LspService::new(Backend::new);
            Server::new(stdin, stdout, socket).serve(service).await;
        });

    Ok(())
}
