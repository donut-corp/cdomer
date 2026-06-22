// ============================================================
// CDOMER VSCode Extension - Client
// Inicia o processo cdomer-lsp e conecta o VSCode a ele via
// stdin/stdout usando o protocolo LSP padrao (highlighting,
// diagnosticos, hover, autocomplete).
//
// Tambem registra o comando "CDOMER: Run", que compila e executa
// o arquivo .cdo ativo direto do editor (botao de play na barra
// de titulo), sem a pessoa precisar abrir um terminal.
//
// Se cdomer-lsp ou cdomer (compilador) nao estiverem instalados,
// a extensao oferece instalar automaticamente via cargo install,
// e se faltar um compilador C (gcc/clang), oferece instalar via
// apt quando disponivel (ex: Codespaces/Linux).
// ============================================================

const path = require("path");
const { workspace, window, ProgressLocation, commands } = require("vscode");
const { execFile, exec } = require("child_process");
const { promisify } = require("util");
const execFileAsync = promisify(execFile);
const execAsync = promisify(exec);
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;
let outputChannel;

/// Verifica se um comando existe e roda sem erro (--version).
async function commandExists(cmd) {
  try {
    await execFileAsync(cmd, ["--version"]);
    return true;
  } catch {
    return false;
  }
}

/// Versao generica de ensureLsp: garante que um binario instalavel via
/// `cargo install <crateName>` esteja disponivel, oferecendo instalar
/// automaticamente quando ausente. Usado tanto para o cdomer-lsp quanto
/// para o compilador cdomer em si.
async function ensureBinary(binPath, crateName, friendlyName) {
  if (await commandExists(binPath)) {
    return binPath;
  }

  const hasCargo = await commandExists("cargo");
  if (!hasCargo) {
    window.showErrorMessage(
      `CDOMER: '${friendlyName}' nao foi encontrado e o Rust (cargo) tambem nao esta ` +
      "instalado. Instale o Rust em https://rustup.rs e reabra o VSCode."
    );
    return null;
  }

  const installed = await window.withProgress(
    {
      location: ProgressLocation.Notification,
      title: `CDOMER: instalando ${friendlyName}...`,
      cancellable: false,
    },
    async () => {
      try {
        await execAsync(`cargo install ${crateName}`, { timeout: 10 * 60 * 1000 });
        return true;
      } catch (err) {
        window.showErrorMessage(
          `CDOMER: falha ao instalar ${friendlyName} automaticamente (${err.message}). ` +
          `Voce pode instalar manualmente rodando 'cargo install ${crateName}' no terminal.`
        );
        return false;
      }
    }
  );

  if (installed) {
    window.showInformationMessage(`CDOMER: ${friendlyName} instalado com sucesso!`);
    return crateName;
  }
  return null;
}

/// Garante que o gcc (ou clang como fallback) esteja disponivel, ja que
/// o compilador cdomer depende dele para gerar o binario final a partir
/// do C transpilado. Diferente do cdomer-lsp, isso NAO pode ser resolvido
/// via cargo install -- e' uma ferramenta de sistema.
async function ensureCCompiler() {
  if (await commandExists("gcc")) return "gcc";
  if (await commandExists("clang")) return "clang";

  const choice = await window.showErrorMessage(
    "CDOMER: nenhum compilador C (gcc/clang) foi encontrado. O CDOMER precisa " +
    "de um deles instalado no sistema para gerar o binario final.",
    "Instalar com apt (Codespaces/Linux)",
    "Cancelar"
  );

  if (choice === "Instalar com apt (Codespaces/Linux)") {
    return window.withProgress(
      {
        location: ProgressLocation.Notification,
        title: "CDOMER: instalando gcc via apt...",
        cancellable: false,
      },
      async () => {
        try {
          await execAsync("sudo apt-get update && sudo apt-get install -y gcc", {
            timeout: 5 * 60 * 1000,
          });
          window.showInformationMessage("CDOMER: gcc instalado com sucesso!");
          return "gcc";
        } catch (err) {
          window.showErrorMessage(
            `CDOMER: falha ao instalar gcc automaticamente (${err.message}). ` +
            "No Termux, rode 'pkg install clang' manualmente."
          );
          return null;
        }
      }
    );
  }
  return null;
}

/// Comando "CDOMER: Run" -- compila e executa o arquivo .cdo ativo,
/// mostrando a saida em um Output Channel dedicado. A pessoa nunca
/// precisa abrir um terminal manualmente: basta clicar no botao de
/// play que aparece na barra de titulo do editor quando um .cdo
/// esta aberto.
async function runActiveFile() {
  const editor = window.activeTextEditor;
  if (!editor || editor.document.languageId !== "cdomer") {
    window.showWarningMessage("CDOMER: abra um arquivo .cdo para executar.");
    return;
  }

  await editor.document.save();

  const config = workspace.getConfiguration("cdomer");
  const compilerPath = config.get("compiler.path") || "cdomer";

  const resolvedCompiler = await ensureBinary(compilerPath, "cdomer", "compilador cdomer");
  if (!resolvedCompiler) return;

  const hasCC = await ensureCCompiler();
  if (!hasCC) return;

  if (!outputChannel) {
    outputChannel = window.createOutputChannel("CDOMER");
  }
  outputChannel.clear();
  outputChannel.show(true);
  outputChannel.appendLine(`$ cdomer run ${path.basename(editor.document.fileName)}`);
  outputChannel.appendLine("");

  const cwd = path.dirname(editor.document.fileName);
  const fileName = path.basename(editor.document.fileName);

  await new Promise((resolve) => {
    execFile(
      resolvedCompiler,
      ["run", fileName],
      { cwd, timeout: 30 * 1000 },
      (error, stdout, stderr) => {
        if (stdout) outputChannel.append(stdout);
        if (stderr) outputChannel.append(stderr);
        if (error && !stdout && !stderr) {
          outputChannel.appendLine(`Erro: ${error.message}`);
        }
        resolve();
      }
    );
  });
}

async function activate(context) {
  context.subscriptions.push(
    commands.registerCommand("cdomer.run", runActiveFile)
  );

  const config = workspace.getConfiguration("cdomer");
  const configuredPath = config.get("lsp.path") || "cdomer-lsp";

  const lspPath = await ensureBinary(configuredPath, "cdomer-lsp", "language server (cdomer-lsp)");
  if (!lspPath) {
    // Sem o LSP nao ha o que iniciar -- a extensao ainda assim
    // mantem o syntax highlighting e o comando Run funcionando
    // normalmente, ja que nenhum dos dois depende do language server.
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
