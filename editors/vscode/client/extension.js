// ============================================================
// CDOMER VSCode Extension - Client
// Inicia o processo cdomer-lsp e conecta o VSCode a ele via
// stdin/stdout usando o protocolo LSP padrao.
//
// Se o cdomer-lsp nao estiver instalado, a extensao oferece
// instalar automaticamente via `cargo install cdomer-lsp`,
// para que a pessoa nao precise abrir terminal manualmente.
// ============================================================

const path = require("path");
const { workspace, window, ProgressLocation } = require("vscode");
const { execFile, exec } = require("child_process");
const { promisify } = require("util");
const execFileAsync = promisify(execFile);
const execAsync = promisify(exec);
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;

/// Verifica se um comando existe e roda sem erro (--version).
async function commandExists(cmd) {
  try {
    await execFileAsync(cmd, ["--version"]);
    return true;
  } catch {
    return false;
  }
}

/// Garante que o cdomer-lsp esta disponivel, instalando via cargo
/// se necessario. Retorna o caminho/comando para usar, ou null se
/// nao foi possivel garantir a instalacao.
async function ensureLsp(lspPath) {
  if (await commandExists(lspPath)) {
    return lspPath;
  }

  const hasCargo = await commandExists("cargo");
  if (!hasCargo) {
    window.showErrorMessage(
      "CDOMER: o language server 'cdomer-lsp' nao foi encontrado e o Rust (cargo) " +
      "tambem nao esta instalado. Instale o Rust em https://rustup.rs e reabra o VSCode " +
      "para ativar autocomplete e diagnosticos em tempo real."
    );
    return null;
  }

  const installed = await window.withProgress(
    {
      location: ProgressLocation.Notification,
      title: "CDOMER: instalando o language server (cdomer-lsp)...",
      cancellable: false,
    },
    async () => {
      try {
        await execAsync("cargo install cdomer-lsp", { timeout: 10 * 60 * 1000 });
        return true;
      } catch (err) {
        window.showErrorMessage(
          `CDOMER: falha ao instalar cdomer-lsp automaticamente (${err.message}). ` +
          "Voce pode instalar manualmente rodando 'cargo install cdomer-lsp' no terminal."
        );
        return false;
      }
    }
  );

  if (installed) {
    window.showInformationMessage("CDOMER: language server instalado com sucesso!");
    return "cdomer-lsp";
  }
  return null;
}

async function activate(context) {
  const config = workspace.getConfiguration("cdomer");
  const configuredPath = config.get("lsp.path") || "cdomer-lsp";

  const lspPath = await ensureLsp(configuredPath);
  if (!lspPath) {
    // Sem o LSP nao ha o que iniciar -- a extensao ainda assim
    // mantem o syntax highlighting funcionando normalmente, ja
    // que isso nao depende do language server.
    return;
  }

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
