#!/bin/bash -x
# Contador de palavras em um arquivo txt ou docx
set -euo pipefail

BIN_NAME="wordcounter"
OUT_DIR="dist"
WIN=false
TARGET_WINDOWS="x86_64-pc-windows-gnu"
TARGET_LINUX="x86_64-unknown-linux-gnu"

function print_usage() {
  echo "Compila o projeto. Uso:"
  echo "./build.sh [-w] [-h] "
  echo "  -l : compila o projeto LINUX"
  echo "  -w : compila o projeto WINDOWS"
  echo "  -h : exibe esta ajuda"
  echo "Exemplos de como compilar: "
  echo " ./build.sh #Compila o projeto"
  echo " ./build.sh -l #Compila o projeto para Linux"
  echo " ./build.sh -w #Compila o projeto para Windows"
}

while getopts ':lwh' flag; do
  case "${flag}" in
    l) WIN=false ;; # compila o projeto para Linux
    w) WIN=true ;; # compila o projeto para Windows
    h|*)
      print_usage
      exit 1 ;;
  esac
done

sudo apt-get install -y rustup mingw-w64
rustup update
rustup default stable

if [ "$WIN" = true ] ; then
  # compila para windows
  TARGET="$TARGET_WINDOWS"
  OUT_DIR="${OUT_DIR}/windows"
  BIN_NAME="${BIN_NAME}.exe"
  rustup target remove "$TARGET_LINUX"
  rustup target add "$TARGET_WINDOWS"
else
  #x86_64-unknown-linux-gnu
  TARGET="$TARGET_LINUX"
  OUT_DIR="${OUT_DIR}/linux"
  rustup target remove "$TARGET_WINDOWS"
  rustup target add "$TARGET_LINUX"
fi

mkdir -p "${OUT_DIR}"
cargo build --release --target "$TARGET"
cp "target/${TARGET}/release/${BIN_NAME}" "${OUT_DIR}/${BIN_NAME}"

echo "Binário gerado em: ${OUT_DIR}/${BIN_NAME}"