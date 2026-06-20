# CDOMER Language Support (VSCode)

Syntax highlighting, autocomplete e diagnosticos em tempo real para arquivos
`.cdo` da linguagem CDOMER.

## Funcionalidades

- **Syntax highlighting**: palavras-chave, tipos, strings, numeros, comentarios.
- **Diagnosticos em tempo real**: erros de lexico, sintaxe e tipo aparecem
  sublinhados conforme voce digita.
- **Autocomplete**: sugere palavras-chave, tipos primitivos, funcoes e
  variaveis declaradas no arquivo.
- **Hover**: passe o mouse sobre uma variavel/funcao/struct para ver seu tipo.

## Pre-requisitos

O `cdomer-lsp` (language server) precisa estar instalado e no PATH.

```bash
cargo install cdomer-lsp
```

Ou, se voce ja clonou o repositorio:

```bash
cd cdomer-lsp
cargo build --release
cp ../target/release/cdomer-lsp $PREFIX/bin/   # Termux
# ou: sudo cp ../target/release/cdomer-lsp /usr/local/bin/
```

## Instalacao da extensao

### A partir do codigo-fonte (modo desenvolvimento)

```bash
cd editors/vscode
npm install
```

No VSCode: `F5` (ou "Run Extension" no painel Run and Debug) abre uma
nova janela do VSCode com a extensao carregada.

### Empacotada (.vsix)

```bash
cd editors/vscode
npm install -g @vscode/vsce
vsce package
```

Isso gera um arquivo `cdomer-lang-0.1.0.vsix`. No VSCode: Extensions ->
"..." (menu) -> Install from VSIX... -> seleciona o arquivo gerado.

## Configuracao

Se o `cdomer-lsp` nao estiver no PATH, configure o caminho completo em
`settings.json`:

```json
{
  "cdomer.lsp.path": "/caminho/completo/para/cdomer-lsp"
}
```
