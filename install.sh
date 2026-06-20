#!/bin/bash
# ============================================================
# Instalador do CDOMER para Termux / Linux
# Uso:
#   curl -sL https://raw.githubusercontent.com/donut-corp/cdomer/master/install.sh | bash
# ou, com o repo ja clonado:
#   bash install.sh
# ============================================================
set -e

REPO_URL="https://github.com/donut-corp/cdomer.git"
INSTALL_DIR="$HOME/.cdomer-src"
BIN_NAME="cdomer"

echo "==> Instalando CDOMER..."

# 1) garante que o Rust existe
if ! command -v cargo &> /dev/null; then
    echo "==> Rust nao encontrado."
    if command -v pkg &> /dev/null; then
        echo "==> Instalando rust e clang via pkg (Termux)..."
        pkg install -y rust clang
    else
        echo "Instale o Rust manualmente: https://rustup.rs"
        exit 1
    fi
fi

# 2) garante que tem um compilador C (gcc/clang) para o backend de codegen
if ! command -v gcc &> /dev/null && ! command -v clang &> /dev/null; then
    echo "==> Nenhum compilador C encontrado."
    if command -v pkg &> /dev/null; then
        pkg install -y clang
    else
        echo "Instale gcc ou clang manualmente antes de continuar."
        exit 1
    fi
fi

# 3) clona ou atualiza o repositorio
if [ -d "$INSTALL_DIR/.git" ]; then
    echo "==> Atualizando repositorio existente..."
    git -C "$INSTALL_DIR" pull
else
    echo "==> Clonando repositorio..."
    rm -rf "$INSTALL_DIR"
    git clone "$REPO_URL" "$INSTALL_DIR"
fi

# 4) compila em modo release
echo "==> Compilando (pode levar alguns minutos)..."
cd "$INSTALL_DIR"
cargo build --release

# 5) instala no PATH
BIN_DIR="${PREFIX:-/usr/local}/bin"
mkdir -p "$BIN_DIR"
cp "target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
chmod +x "$BIN_DIR/$BIN_NAME"

echo ""
echo "==> CDOMER instalado com sucesso!"
echo "    Binario em: $BIN_DIR/$BIN_NAME"
echo ""
"$BIN_DIR/$BIN_NAME" || true
