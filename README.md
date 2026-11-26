# contador_palavras
Contator de palavras

## Instalação do Rust

Requer Cargo 

```bash
sudo apt install -y cargo rustup mingw-w64
rustup update
rustup default stable
```

## Como compilar
- Compilando no linux
```bash
cargo build
```

- Compilando para windows
```bash
sudo apt install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Como executar

```bash
cargo run --release -- /home/douglas/tmp/arquivo.txt
```


