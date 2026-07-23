#!/usr/bin/env bash
# ==============================================================================
# Script de Instalação do FzComputerAI / CUA Driver (Computer Vision via MCP)
# Suporte Nativo: Linux (X11/Wayland) & macOS
# Licença: Creative Commons Attribution 4.0 International (CC BY 4.0)
# Desenvolvido por: Roger Luft (Webstorage Tecnologia)
# ==============================================================================

set -e

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}======================================================================${NC}"
echo -e "${YELLOW}   FzComputerAI — Servidor de Computer Vision via MCP (Linux/macOS)${NC}"
echo -e "${GREEN}   Webstorage Tecnologia — Roger Luft <roger@webstorage.com.br>${NC}"
echo -e "${CYAN}======================================================================${NC}"
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${CYAN}[INFO] Verificando dependências do sistema...${NC}"

# Verificar se Rust/Cargo existe
HAS_CARGO=0
if command -v cargo >/dev/null 2>&1; then
    HAS_CARGO=1
    echo -e "${GREEN}[SUCCESS] Rust/Cargo detectado: $(cargo --version)${NC}"
fi

# Verificar se cua-driver já está no PATH
HAS_CUA=0
if command -v cua-driver >/dev/null 2>&1; then
    HAS_CUA=1
    echo -e "${GREEN}[SUCCESS] cua-driver detectado: $(cua-driver --version)${NC}"
fi

RUST_WORKSPACE="$SCRIPT_DIR/cua/libs/cua-driver/rust/Cargo.toml"
if [ ! -f "$RUST_WORKSPACE" ]; then
    RUST_WORKSPACE="$SCRIPT_DIR/libs/cua-driver/rust/Cargo.toml"
fi

BIN_PATH=""

if [ "$HAS_CARGO" -eq 1 ] && [ -f "$RUST_WORKSPACE" ]; then
    echo -e "${CYAN}[INFO] Compilando o motor cua-driver via Cargo (--release)...${NC}"
    (cd "$(dirname "$RUST_WORKSPACE")" && cargo build --release --package cua-driver)
    if [ -f "$(dirname "$RUST_WORKSPACE")/target/release/cua-driver" ]; then
        BIN_PATH="$(dirname "$RUST_WORKSPACE")/target/release/cua-driver"
        echo -e "${GREEN}[SUCCESS] Binário compilado com sucesso: $BIN_PATH${NC}"
    fi
fi

if [ -z "$BIN_PATH" ]; then
    if [ "$HAS_CUA" -eq 1 ]; then
        BIN_PATH="$(command -v cua-driver)"
        echo -e "${CYAN}[INFO] Utilizando binário do PATH: $BIN_PATH${NC}"
    else
        echo -e "${CYAN}[INFO] Baixando e instalando pacote oficial do Cua Driver...${NC}"
        curl -fsSL https://cua.ai/driver/install.sh | bash
        BIN_PATH="$HOME/.cua-driver/packages/current/cua-driver"
    fi
fi

# Criar configuração .mcp.json local
MCP_JSON="$SCRIPT_DIR/.mcp.json"
echo -e "${CYAN}[INFO] Criando/atualizando configuração MCP em $MCP_JSON...${NC}"
cat <<EOF > "$MCP_JSON"
{
  "mcpServers": {
    "fz-computer-vision": {
      "command": "cua-driver",
      "args": [
        "mcp"
      ],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
EOF
echo -e "${GREEN}[SUCCESS] Configuração .mcp.json criada.${NC}"

# Testar diagnóstico
echo -e "${CYAN}[INFO] Verificando saúde do sistema (cua-driver doctor)...${NC}"
if command -v cua-driver >/dev/null 2>&1; then
    cua-driver doctor || true
elif [ -x "$BIN_PATH" ]; then
    "$BIN_PATH" doctor || true
fi

echo ""
echo -e "${CYAN}======================================================================${NC}"
echo -e "${GREEN}   Instalação concluída com sucesso!${NC}"
echo -e "${YELLOW}   O Servidor de Computer Vision via MCP está pronto para uso.${NC}"
echo -e "${CYAN}   Comando para executar manualmente: cua-driver mcp${NC}"
echo -e "${CYAN}======================================================================${NC}"
