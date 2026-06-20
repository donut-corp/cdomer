// ============================================================
// CDOMER Language Server
// Implementa o protocolo LSP para a linguagem CDOMER, dando
// suporte a:
//   - diagnosticos em tempo real (erros de lexico/sintaxe/tipo)
//   - autocomplete de palavras-chave, tipos, funcoes e variaveis
//   - hover mostrando o tipo de uma variavel/funcao
//
// Roda como um processo separado que troca mensagens JSON-RPC
// via stdin/stdout com o editor (VSCode, neovim, etc).
// ============================================================

mod lexer;
mod ast;
mod parser;
mod typechecker;

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use ast::{Program, TopLevel, Stmt, Type};
use lexer::Lexer;
use parser::Parser as CdomerParser;
use typechecker::TypeChecker;

const KEYWORDS: &[&str] = &[
    "let", "fn", "return", "if", "else", "while", "for", "struct",
    "true", "false", "break", "continue", "print",
];

const TYPES: &[&str] = &["int", "float", "bool", "string", "void"];

/// Severidade de um diagnostico. So existe Error por enquanto -- a linguagem
/// nao tem warnings hoje, mas o campo fica pronto para extensao futura
/// (ex: variavel nao usada).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
}

#[derive(Debug, Clone)]
pub struct CdomerDiagnostic {
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub severity: Severity,
}

pub struct AnalysisResult {
    pub program: Option<Program>,
    pub diagnostics: Vec<CdomerDiagnostic>,
}

/// Roda o pipeline completo (lexer -> parser -> typechecker) e coleta
/// diagnosticos. Usado pelo LSP a cada vez que o documento muda.
fn analyze(source: &str) -> AnalysisResult {
    let mut diagnostics = Vec::new();

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            diagnostics.push(CdomerDiagnostic {
                message: e.message,
                line: e.line,
                col: e.col,
                severity: Severity::Error,
            });
            return AnalysisResult { program: None, diagnostics };
        }
    };

    let mut parser = CdomerParser::new(tokens);
    let mut program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            diagnostics.push(CdomerDiagnostic {
                message: e.message,
                line: e.line,
                col: e.col,
                severity: Severity::Error,
            });
            return AnalysisResult { program: None, diagnostics };
        }
    };

    let mut checker = TypeChecker::new();
    if let Err(e) = checker.check_program(&mut program) {
        diagnostics.push(CdomerDiagnostic {
            message: e.message,
            line: e.line,
            col: 1,
            severity: Severity::Error,
        });
    }

    AnalysisResult { program: Some(program), diagnostics }
}

struct Backend {
    client: Client,
    /// Guarda o texto-fonte mais recente de cada arquivo aberto.
    documents: DashMap<Url, String>,
}

impl Backend {
    async fn publish_diagnostics_for(&self, uri: Url, text: &str) {
        let result = analyze(text);
        let diags: Vec<Diagnostic> = result
            .diagnostics
            .iter()
            .map(|d| {
                // LSP usa posicoes 0-indexed; nosso compilador usa 1-indexed.
                let line = d.line.saturating_sub(1) as u32;
                let col = d.col.saturating_sub(1) as u32;
                Diagnostic {
                    range: Range {
                        start: Position { line, character: col },
                        end: Position { line, character: col + 1 },
                    },
                    severity: Some(match d.severity {
                        Severity::Error => DiagnosticSeverity::ERROR,
                    }),
                    source: Some("cdomer".to_string()),
                    message: d.message.clone(),
                    ..Default::default()
                }
            })
            .collect();

        self.client.publish_diagnostics(uri, diags, None).await;
    }

    /// Coleta nomes de funcoes e structs declarados no programa, usados
    /// para sugestoes de autocomplete.
    fn collect_top_level_completions(program: &Program) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        for item in &program.items {
            match item {
                TopLevel::Fn(f) => {
                    let params: Vec<String> = f
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, p.ty))
                        .collect();
                    items.push(CompletionItem {
                        label: f.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("fn {}({}) -> {}", f.name, params.join(", "), f.return_type)),
                        ..Default::default()
                    });
                }
                TopLevel::Struct(s) => {
                    items.push(CompletionItem {
                        label: s.name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        detail: Some(format!("struct {}", s.name)),
                        ..Default::default()
                    });
                }
            }
        }
        items
    }

    /// Coleta variaveis locais visiveis (heuristica simples: todas as
    /// declaracoes `let` em todas as funcoes, sem escopo fino -- suficiente
    /// para autocomplete util sem reimplementar resolucao de escopo aqui).
    fn collect_local_completions(program: &Program) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        for item in &program.items {
            if let TopLevel::Fn(f) = item {
                for p in &f.params {
                    items.push(CompletionItem {
                        label: p.name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(format!("{}: {} (parametro)", p.name, p.ty)),
                        ..Default::default()
                    });
                }
                collect_let_in_stmts(&f.body, &mut items);
            }
        }
        items
    }
}

fn collect_let_in_stmts(stmts: &[Stmt], out: &mut Vec<CompletionItem>) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, declared_type, .. } => {
                let ty = declared_type.clone().unwrap_or(Type::Unknown);
                out.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(format!("{}: {}", name, ty)),
                    ..Default::default()
                });
            }
            Stmt::If { then_branch, else_branch, .. } => {
                collect_let_in_stmts(then_branch, out);
                if let Some(eb) = else_branch {
                    collect_let_in_stmts(eb, out);
                }
            }
            Stmt::While { body, .. } => collect_let_in_stmts(body, out),
            Stmt::For { body, .. } => collect_let_in_stmts(body, out),
            Stmt::Block(b) => collect_let_in_stmts(b, out),
            _ => {}
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "cdomer-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "cdomer-lsp inicializado")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone());
        self.publish_diagnostics_for(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // Com TextDocumentSyncKind::FULL o editor manda o documento inteiro
        // a cada mudanca, entao pegamos so a ultima mudanca da lista.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.insert(uri.clone(), change.text.clone());
            self.publish_diagnostics_for(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let mut items: Vec<CompletionItem> = Vec::new();

        for kw in KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        for ty in TYPES {
            items.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..Default::default()
            });
        }

        if let Some(text) = self.documents.get(&uri) {
            let result = analyze(&text);
            if let Some(program) = &result.program {
                items.extend(Backend::collect_top_level_completions(program));
                items.extend(Backend::collect_local_completions(program));
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let text = match self.documents.get(&uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };

        let word = match word_at_position(&text, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        let result = analyze(&text);
        let program = match &result.program {
            Some(p) => p,
            None => return Ok(None),
        };

        // procura primeiro em funcoes/structs top-level
        for item in &program.items {
            match item {
                TopLevel::Fn(f) if f.name == word => {
                    let params: Vec<String> = f.params.iter().map(|p| format!("{}: {}", p.name, p.ty)).collect();
                    return Ok(Some(make_hover(format!(
                        "```cdomer\nfn {}({}) -> {}\n```",
                        f.name, params.join(", "), f.return_type
                    ))));
                }
                TopLevel::Struct(s) if s.name == word => {
                    let fields: Vec<String> = s.fields.iter().map(|f| format!("    {}: {}", f.name, f.ty)).collect();
                    return Ok(Some(make_hover(format!(
                        "```cdomer\nstruct {} {{\n{}\n}}\n```",
                        s.name, fields.join(",\n")
                    ))));
                }
                _ => {}
            }
        }

        // procura em parametros e `let`s
        for item in &program.items {
            if let TopLevel::Fn(f) = item {
                for p in &f.params {
                    if p.name == word {
                        return Ok(Some(make_hover(format!("```cdomer\n{}: {}\n```\n(parametro)", p.name, p.ty))));
                    }
                }
                if let Some(hover) = find_let_hover(&f.body, &word) {
                    return Ok(Some(hover));
                }
            }
        }

        Ok(None)
    }
}

fn find_let_hover(stmts: &[Stmt], word: &str) -> Option<Hover> {
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, declared_type, .. } if name == word => {
                let ty = declared_type.clone().unwrap_or(Type::Unknown);
                return Some(make_hover(format!("```cdomer\nlet {}: {}\n```", name, ty)));
            }
            Stmt::If { then_branch, else_branch, .. } => {
                if let Some(h) = find_let_hover(then_branch, word) {
                    return Some(h);
                }
                if let Some(eb) = else_branch {
                    if let Some(h) = find_let_hover(eb, word) {
                        return Some(h);
                    }
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } | Stmt::Block(body) => {
                if let Some(h) = find_let_hover(body, word) {
                    return Some(h);
                }
            }
            _ => {}
        }
    }
    None
}

fn make_hover(markdown: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    }
}

/// Extrai a "palavra" (identificador) sob a posicao do cursor, usada
/// tanto para hover quanto futuramente para go-to-definition.
fn word_at_position(text: &str, pos: Position) -> Option<String> {
    let line = text.lines().nth(pos.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let col = (pos.character as usize).min(chars.len());

    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_';

    if col >= chars.len() || !is_ident_char(chars[col]) {
        // tenta a posicao imediatamente anterior (cursor logo apos a palavra)
        if col == 0 || !is_ident_char(chars[col - 1]) {
            return None;
        }
    }

    let mut start = col.min(chars.len().saturating_sub(1));
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(chars[start..end].iter().collect())
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: DashMap::new(),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
