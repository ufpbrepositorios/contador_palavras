#!/usr/bin/env bash
set -euo pipefail

TARGET="x86_64-pc-windows-gnu"
BIN_NAME="contador_palavras.exe"
OUT_DIR="dist/windows"

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "Erro: 'x86_64-w64-mingw32-gcc' não encontrado. Instale com: sudo apt update && sudo apt install -y mingw-w64" >&2
    exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
    echo "Erro: 'rustup' não encontrado no PATH." >&2
    exit 1
fi

rustup target add "$TARGET"
cargo build --release --target "$TARGET"

mkdir -p "$OUT_DIR"
cp "target/${TARGET}/release/${BIN_NAME}" "${OUT_DIR}/${BIN_NAME}"

echo "Binário gerado em: ${OUT_DIR}/${BIN_NAME}"