const { LanguageClient } = require("vscode-languageclient/node");
const path = require("path");

let client;

function activate(context) {
  // Path to your compiled CLI binary
  const serverPath = path.join(context.extensionPath, "..", "..", "target", "debug", "toy");

  const serverOptions = {
    command: serverPath,
    args: ["server"],
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "toy" }],
  };

  client = new LanguageClient("toy-lang", "Toy Language Server", serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
