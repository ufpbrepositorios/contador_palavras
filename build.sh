#!/bin/bash
# Contador de palavras em um arquivo txt ou docx
set -euo pipefail

BIN_NAME="wordcounter"
OUT_DIR="dist"
WIN=false
TARGET_WINDOWS="x86_64-pc-windows-gnu"
TARGET_LINUX="x86_64-unknown-linux-gnu"

function print_usage() {
  echo "Compila o projeto. Uso:"
  echo "./build.sh [-l] [-w] [-h] "
  echo "  -l : compila o projeto LINUX"
  echo "  -w : compila o projeto WINDOWS"
  echo "  -h : exibe esta ajuda"
  echo "Exemplos de como compilar: "
  echo " ./build.sh -l #Compila o projeto para Linux"
  echo " ./build.sh -w #Compila o projeto para Windows"
}

function compila() {
  echo "Compilando o projeto..."
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Erro: 'cargo' não encontrado no PATH." >&2
    exit 1
  fi

  if ! command -v rustup >/dev/null 2>&1; then
    echo "Erro: 'rustup' não encontrado no PATH." >&2
    exit 1
  fi

  echo "Instalando dependências necessárias..."
  sudo apt-get install -y rustup mingw-w64
  rustup update
  rustup default stable

  # Adiciona os targets necessários
  [ -z "$(rustup target list | grep x86_64-unknown-linux-gnu)" ] && rustup target add x86_64-unknown-linux-gnu
  [ -z "$(rustup target list | grep x86_64-pc-windows-gnu)" ] && rustup target add x86_64-pc-windows-gnu  

  if [ "$WIN" = true ] ; then
    # compila para windows
    TARGET="$TARGET_WINDOWS"
    OUT_DIR="${OUT_DIR}/windows"
    BIN_NAME="${BIN_NAME}.exe"
  else
    #x86_64-unknown-linux-gnu
    TARGET="$TARGET_LINUX"
    OUT_DIR="${OUT_DIR}/linux"
  fi

  mkdir -p "${OUT_DIR}"
  cargo build --release --target "$TARGET"
  cp "target/${TARGET}/release/${BIN_NAME}" "${OUT_DIR}/${BIN_NAME}"

  echo "Binário gerado em: ${OUT_DIR}/${BIN_NAME}"
}

if [ "$OPTIND" -eq 1 ]; then
  WIN=false
  compila
fi

while getopts ':lwh' flag; do
  case "${flag}" in
    l) WIN=false
      compila ;;
    w) 
      WIN=true
      compila ;;
    h)
      print_usage
      exit 0 ;;
    \?)
      echo "Opção inválida: -$OPTARG" >&2
      print_usage
      exit 1 ;;
  esac
done   

