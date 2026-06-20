#!/bin/bash
# Compila e roda todos os exemplos em examples/, conferindo que cada um
# compila sem erro. Use depois de `cargo build --release`.
set -e

BIN="../target/release/cdomer"
if [ ! -f "$BIN" ]; then
    echo "Binario nao encontrado em $BIN -- rode 'cargo build --release' primeiro."
    exit 1
fi

cd "$(dirname "$0")/../examples"

for f in *.cdo; do
    echo "=== $f ==="
    "../target/release/cdomer" run "$f"
    echo
done

echo "Todos os exemplos rodaram com sucesso."
