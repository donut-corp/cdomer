// ============================================================
// CDOMER VSCode Extension - Client
// Inicia o processo cdomer-lsp e conecta o VSCode a ele via
// stdin/stdout usando o protocolo LSP padrao.
// ============================================================

const path = require("path");
const { workspace } = require("vscode");
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;

function activate(context) {
  const config = workspace.getConfiguration("cdomer");
  const lspPath = config.get("lsp.path") || "cdomer-lsp";

  const serverOptions = {
    run: { command: lspPath, transport: TransportKind.stdio },
    debug: { command: lspPath, transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "cdomer" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.cdo"),
    },
  };

  client = new LanguageClient(
    "cdomerLanguageServer",
    "CDOMER Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
  context.subscriptions.push({
    dispose: () => client && client.stop(),
  });
}

function deactivate() {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

module.exports = { activate, deactivate };
