# Contador de palavras

## Instalação do Rust
---
```bash
sudo apt install -y cargo rustup mingw-w64
rustup update
rustup default stable
```

## Como compilar
---
Na compilação, o build.sh verifica as depenências e instala se necessário.

- Compilando no linux
./build.sh -[l|w]
```bash
./build.sh -l 
```

## Exemplo
---

```bash
cat << 'EOF' > "arquivo.txt"
joao joao jose maria maria maria maria antonio
EOF
wordcounter arquivo.txt
```
Resultado
```bash
  4 maria
  2 joao
  1 antonio
  1 jose

```

### Contato

<douglas@sti.ufpb.br>