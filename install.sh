#!/usr/bin/env bash
# ==============================================================================
# Script de Instalação do FzComputerAI (Computer Vision via MCP & GUI Rust)
# Suporte Nativo: Linux (X11/Wayland) & macOS
# Licença: Creative Commons Attribution 4.0 International (CC BY 4.0)
# Desenvolvido por: Roger Luft (Webstorage Tecnologia)
# Patrocinadores: www.webstorage.com.br | www.imovelsite.com.br
#
# Uso local (dentro do checkout do repositório):
#   bash install.sh
# Uso remoto (one-liner):
#   curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash
# Simulação (nada é executado/instalado):
#   curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash -s -- --dry-run
# ==============================================================================

set -euo pipefail

REPO_OWNER="RLuf"
REPO_NAME="fzcomputerai"
REPO_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}"
API_LATEST="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()    { echo -e "${CYAN}[INFO] $*${NC}"; }
success() { echo -e "${GREEN}[SUCCESS] $*${NC}"; }
warn()    { echo -e "${YELLOW}[AVISO] $*${NC}"; }
err()     { echo -e "${RED}[ERRO] $*${NC}" >&2; }
dry()     { echo -e "${YELLOW}[DRY-RUN]${NC} $*"; }

# Executa o comando, ou apenas o imprime em modo --dry-run
run() {
    if [ "$DRY_RUN" -eq 1 ]; then
        dry "$*"
    else
        "$@"
    fi
}

usage() {
    echo "Uso: install.sh [--dry-run]"
    echo ""
    echo "  --dry-run, -n   Mostra o que seria feito, sem executar nada."
    echo "  --help, -h      Mostra esta ajuda."
}

DRY_RUN=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run|-n) DRY_RUN=1 ;;
        --help|-h) usage; exit 0 ;;
        *) err "Opção desconhecida: $1"; usage; exit 2 ;;
    esac
    shift
done

echo -e "${CYAN}======================================================================${NC}"
echo -e "${YELLOW}   FzComputerAI — Servidor de Computer Vision via MCP & GUI Rust${NC}"
echo -e "${GREEN}   Webstorage Tecnologia (www.webstorage.com.br)${NC}"
echo -e "${GREEN}   Imóvel Site (www.imovelsite.com.br)${NC}"
echo -e "${GREEN}   Autor: Roger Luft <roger@webstorage.com.br>${NC}"
echo -e "${CYAN}======================================================================${NC}"
echo ""

if [ "$DRY_RUN" -eq 1 ]; then
    warn "Modo --dry-run ativo: nenhuma alteração será feita no sistema."
fi

# ------------------------------------------------------------------------------
# Windows não é alvo deste script (.sh) — orientar para o install.ps1
# ------------------------------------------------------------------------------
OS_NAME="$(uname -s)"
case "$OS_NAME" in
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
        warn "Windows detectado. Este script (.sh) atende apenas Linux e macOS."
        info "No Windows, utilize o instalador PowerShell:"
        echo "    powershell -ExecutionPolicy Bypass -File install.ps1"
        info "Ou, direto da internet (PowerShell):"
        echo "    irm ${REPO_URL}/raw/master/install.ps1 | iex"
        exit 0
        ;;
esac

# ------------------------------------------------------------------------------
# Detecção de modo: local (dentro do checkout) ou remoto (curl | bash)
# ------------------------------------------------------------------------------
SCRIPT_SOURCE="${BASH_SOURCE[0]:-}"
SCRIPT_DIR=""
MODE="remote"
if [ -n "$SCRIPT_SOURCE" ] && [ -f "$SCRIPT_SOURCE" ]; then
    SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_SOURCE")" && pwd)"
    if [ -f "$SCRIPT_DIR/fzcomputerai/Cargo.toml" ]; then
        MODE="local"
    fi
fi
info "Modo de instalação: ${MODE}"

# ------------------------------------------------------------------------------
# Verificações de dependências
# ------------------------------------------------------------------------------
HAS_CARGO=0
if command -v cargo >/dev/null 2>&1; then
    HAS_CARGO=1
    success "Rust/Cargo detectado: $(cargo --version 2>/dev/null || echo 'versão desconhecida')"
fi

HAS_CUA=0
if command -v cua-driver >/dev/null 2>&1; then
    HAS_CUA=1
    success "cua-driver detectado: $(cua-driver --version 2>/dev/null || echo 'versão desconhecida')"
fi

# Bibliotecas de sistema necessárias para compilar a GUI no Linux (X11/GTK)
check_linux_build_deps() {
    [ "$OS_NAME" = "Linux" ] || return 0
    local apt_hint="sudo apt-get install -y pkg-config libx11-dev libgl1-mesa-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libwayland-dev libfontconfig1-dev libgtk-3-dev"
    if ! command -v pkg-config >/dev/null 2>&1; then
        warn "pkg-config não encontrado. A compilação da GUI pode falhar."
        warn "Em Debian/Ubuntu, instale as dependências com:"
        echo "    $apt_hint"
        return 0
    fi
    local missing=""
    local lib
    for lib in x11 xkbcommon fontconfig gtk+-3.0; do
        if ! pkg-config --exists "$lib" 2>/dev/null; then
            missing="$missing $lib"
        fi
    done
    if [ -n "$missing" ]; then
        warn "Bibliotecas de desenvolvimento ausentes:${missing}. A compilação da GUI pode falhar."
        warn "Em Debian/Ubuntu, instale com:"
        echo "    $apt_hint"
    fi
}

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "Dependência obrigatória não encontrada: $1. $2"
        exit 1
    fi
}

# ------------------------------------------------------------------------------
# Diretório de instalação (modo remoto)
# ------------------------------------------------------------------------------
INSTALL_DIR=""
SUDO_CMD=""

path_contains() {
    case ":$PATH:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

choose_install_dir() {
    if [ "$(id -u)" -eq 0 ]; then
        INSTALL_DIR="/usr/local/bin"
        SUDO_CMD=""
        return 0
    fi
    INSTALL_DIR="$HOME/.local/bin"
    SUDO_CMD=""
    # Se ~/.local/bin não está no PATH e há sudo sem senha, preferir /usr/local/bin
    if ! path_contains "$HOME/.local/bin" && [ "$DRY_RUN" -eq 0 ] \
        && command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
        INSTALL_DIR="/usr/local/bin"
        SUDO_CMD="sudo"
    fi
}

install_binary() {
    # $1 = arquivo de origem, $2 = nome de destino
    local src="$1" name="$2"
    if [ "$DRY_RUN" -eq 1 ]; then
        dry "${SUDO_CMD:+$SUDO_CMD }mkdir -p $INSTALL_DIR"
        dry "${SUDO_CMD:+$SUDO_CMD }install -m 0755 $src $INSTALL_DIR/$name"
        return 0
    fi
    if [ -n "$SUDO_CMD" ]; then
        "$SUDO_CMD" mkdir -p "$INSTALL_DIR"
        "$SUDO_CMD" install -m 0755 "$src" "$INSTALL_DIR/$name"
    else
        mkdir -p "$INSTALL_DIR"
        install -m 0755 "$src" "$INSTALL_DIR/$name"
    fi
    success "Instalado: $INSTALL_DIR/$name"
}

warn_path_if_needed() {
    if [ -n "$INSTALL_DIR" ] && ! path_contains "$INSTALL_DIR"; then
        warn "$INSTALL_DIR não está no seu PATH. Adicione ao seu ~/.bashrc ou ~/.zshrc:"
        echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
}

# ------------------------------------------------------------------------------
# Limpeza de temporários
# ------------------------------------------------------------------------------
TMP_SRC_DIR=""
TMP_BIN_FILE=""
TMP_SUMS_FILE=""
cleanup() {
    if [ -n "$TMP_SRC_DIR" ]; then rm -rf "$TMP_SRC_DIR" 2>/dev/null || true; fi
    if [ -n "$TMP_BIN_FILE" ]; then rm -f "$TMP_BIN_FILE" 2>/dev/null || true; fi
    if [ -n "$TMP_SUMS_FILE" ]; then rm -f "$TMP_SUMS_FILE" 2>/dev/null || true; fi
}
trap cleanup EXIT

# ------------------------------------------------------------------------------
# Integridade — verificação SHA-256 do binário oficial
#
# Os binários publicados NÃO são assinados digitalmente; o arquivo .sha256
# publicado ao lado de cada artefato é hoje a única garantia de integridade.
# Por isso o download só é instalado depois de conferido contra esse hash.
# ------------------------------------------------------------------------------
API_JSON=""

# Nome da ferramenta de hash disponível no sistema (vazio se não houver nenhuma)
checksum_tool() {
    if command -v sha256sum >/dev/null 2>&1; then
        echo "sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        echo "shasum -a 256"
    fi
}

# Calcula o SHA-256 do arquivo ($1) e imprime somente o hash, em minúsculas.
# Retorna 1 se não houver ferramenta de hash disponível.
compute_sha256() {
    local file="$1" out=""
    if command -v sha256sum >/dev/null 2>&1; then
        out="$(sha256sum "$file" 2>/dev/null || true)"
    elif command -v shasum >/dev/null 2>&1; then
        out="$(shasum -a 256 "$file" 2>/dev/null || true)"
    else
        return 1
    fi
    printf '%s' "$out" | awk '{print tolower($1)}'
}

# Extrai apenas o hash do arquivo .sha256 ($1), ignorando o nome/caminho que o
# acompanha — releases antigas trazem o caminho junto do hash, então depender de
# 'sha256sum -c' e do nome bater falharia. Aceita os formatos:
#   "<hash>  arquivo" | "<hash>" isolado | "SHA256 (arquivo) = <hash>"
extract_sha256() {
    local file="$1" line="" hash=""
    line="$(tr -d '\r' < "$file" 2>/dev/null | head -n1 || true)"
    hash="$(printf '%s' "$line" | awk '{print $1}')"
    if ! printf '%s' "$hash" | grep -qiE '^[0-9a-f]{64}$'; then
        hash="$(printf '%s' "$line" | grep -oiE '[0-9a-f]{64}' | head -n1 || true)"
    fi
    printf '%s' "$hash" | tr 'ABCDEF' 'abcdef'
}

# Descobre a URL de download de um asset ($1) na resposta da API do GitHub.
find_asset_url() {
    local want="$1"
    [ -n "$API_JSON" ] || return 0
    if command -v jq >/dev/null 2>&1; then
        printf '%s' "$API_JSON" | jq -r --arg n "$want" \
            '.assets[]? | select(.name == $n) | .browser_download_url' 2>/dev/null \
            | head -n1 || true
        return 0
    fi
    # Fallback sem jq: compara o nome do arquivo ao final de cada URL
    printf '%s\n' "$API_JSON" | tr ',' '\n' \
        | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | grep -o 'https://[^"]*' \
        | while IFS= read -r url; do
              if [ "${url##*/}" = "$want" ]; then printf '%s\n' "$url"; fi
          done | head -n1 || true
}

# Verifica o arquivo baixado ($1) contra o <asset>.sha256 ($2) da mesma release.
# Retorno: 0 = pode prosseguir (verificado, ou verificação comprovadamente
#              indisponível — sempre com aviso explícito);
#          1 = hash divergente; o chamador deve descartar o arquivo e abortar.
verify_download_checksum() {
    local file="$1" asset_name="$2"
    local sums_url="" expected="" actual=""

    sums_url="$(find_asset_url "${asset_name}.sha256")"
    if [ -z "$sums_url" ]; then
        warn "Checksum oficial (${asset_name}.sha256) não publicado nesta release."
        warn "Não foi possível verificar a integridade do download; prosseguindo assim mesmo."
        return 0
    fi

    if [ -z "$(checksum_tool)" ]; then
        warn "Nem 'sha256sum' nem 'shasum' foram encontrados neste sistema."
        warn "VERIFICAÇÃO DE INTEGRIDADE PULADA — o binário será instalado SEM conferência."
        warn "Instale o coreutils (sha256sum) e confira manualmente com: $sums_url"
        return 0
    fi

    # Template explícito: o mktemp do BSD/macOS não aceita ser chamado sem template
    TMP_SUMS_FILE="$(mktemp "${TMPDIR:-/tmp}/fzcomputerai-sha256.XXXXXXXX" 2>/dev/null || true)"
    if [ -z "$TMP_SUMS_FILE" ]; then
        warn "Falha ao criar arquivo temporário para o checksum. Verificação PULADA."
        return 0
    fi

    info "Baixando checksum oficial: $sums_url"
    if ! curl -fL --silent --show-error -o "$TMP_SUMS_FILE" "$sums_url"; then
        warn "Falha ao baixar ${asset_name}.sha256. VERIFICAÇÃO DE INTEGRIDADE PULADA."
        return 0
    fi

    expected="$(extract_sha256 "$TMP_SUMS_FILE")"
    if [ -z "$expected" ]; then
        warn "Arquivo de checksum em formato inesperado (nenhum SHA-256 encontrado)."
        warn "VERIFICAÇÃO DE INTEGRIDADE PULADA."
        return 0
    fi

    actual="$(compute_sha256 "$file" || true)"
    if [ -z "$actual" ]; then
        warn "Não foi possível calcular o SHA-256 do arquivo baixado."
        warn "VERIFICAÇÃO DE INTEGRIDADE PULADA."
        return 0
    fi

    if [ "$expected" != "$actual" ]; then
        err "CHECKSUM NÃO CONFERE — instalação abortada."
        err "  esperado: $expected"
        err "  obtido:   $actual"
        err "  checksum: $sums_url"
        err "Isso indica download corrompido ou artefato adulterado."
        err "O arquivo baixado será apagado. Não execute esse binário."
        return 1
    fi

    success "Integridade verificada via SHA-256 ($(checksum_tool)): $actual"
    return 0
}

# ------------------------------------------------------------------------------
# Modo remoto — fallback: clonar o repositório e compilar do código-fonte
# ------------------------------------------------------------------------------
build_from_source() {
    info "Compilando do código-fonte (fallback)..."
    require_cmd git "Instale o git para clonar o repositório."
    if [ "$HAS_CARGO" -eq 0 ]; then
        err "Rust/Cargo não encontrado e não há binário oficial aplicável."
        err "Instale o Rust pelo instalador oficial: https://rustup.rs"
        echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    check_linux_build_deps

    if [ "$DRY_RUN" -eq 1 ]; then
        dry "git clone --depth 1 ${REPO_URL}.git <tmpdir>/src"
        dry "cargo build --release --manifest-path <tmpdir>/src/fzcomputerai/Cargo.toml"
        dry "install -m 0755 <tmpdir>/src/fzcomputerai/target/release/fzcomputerai $INSTALL_DIR/fzcomputerai"
        dry "(opcional) cargo build --release --package cua-driver --manifest-path <tmpdir>/src/cua/libs/cua-driver/rust/Cargo.toml"
        return 0
    fi

    TMP_SRC_DIR="$(mktemp -d)"
    info "Clonando ${REPO_URL}.git (raso) em $TMP_SRC_DIR..."
    git clone --depth 1 "${REPO_URL}.git" "$TMP_SRC_DIR/src"

    info "Compilando a GUI FzComputerAI (cargo build --release)..."
    cargo build --release --manifest-path "$TMP_SRC_DIR/src/fzcomputerai/Cargo.toml"
    local gui_bin="$TMP_SRC_DIR/src/fzcomputerai/target/release/fzcomputerai"
    if [ ! -f "$gui_bin" ]; then
        err "Compilação concluída, mas o binário não foi encontrado em $gui_bin."
        exit 1
    fi
    install_binary "$gui_bin" "fzcomputerai"

    # Motor cua-driver (opcional — usado pelo servidor MCP stdio)
    local engine_manifest="$TMP_SRC_DIR/src/cua/libs/cua-driver/rust/Cargo.toml"
    if [ ! -f "$engine_manifest" ]; then
        engine_manifest="$TMP_SRC_DIR/src/libs/cua-driver/rust/Cargo.toml"
    fi
    if [ -f "$engine_manifest" ]; then
        info "Compilando o motor cua-driver (opcional, pode demorar)..."
        if cargo build --release --package cua-driver --manifest-path "$engine_manifest"; then
            local engine_bin
            engine_bin="$(dirname "$engine_manifest")/target/release/cua-driver"
            if [ -f "$engine_bin" ]; then
                install_binary "$engine_bin" "cua-driver"
            fi
        else
            warn "Falha ao compilar o cua-driver. A GUI foi instalada; o motor MCP pode ser instalado depois com: npx fzcomputerai mcp"
        fi
    fi
}

# ------------------------------------------------------------------------------
# Modo remoto — preferir binário oficial do GitHub Releases (API oficial)
# ------------------------------------------------------------------------------
remote_install() {
    require_cmd curl "Instale o curl para baixar o instalador."
    choose_install_dir
    info "Diretório de instalação: $INSTALL_DIR"

    local arch asset=""
    arch="$(uname -m)"
    case "$OS_NAME" in
        Linux)
            case "$arch" in
                x86_64|amd64) asset="fzcomputerai-linux-x64" ;;
                *) warn "Sem binário oficial para Linux/$arch. Usando compilação do código-fonte." ;;
            esac
            ;;
        Darwin)
            asset="fzcomputerai-macos"
            ;;
        *)
            warn "Sistema $OS_NAME sem binário oficial. Usando compilação do código-fonte."
            ;;
    esac

    if [ -z "$asset" ]; then
        build_from_source
        return 0
    fi

    if [ "$DRY_RUN" -eq 1 ]; then
        local tool
        tool="$(checksum_tool)"
        dry "Consultaria a API oficial do GitHub: $API_LATEST"
        dry "Procuraria o asset: $asset"
        dry "Baixaria: curl -fL -o <tmpfile> <browser_download_url>"
        dry "Baixaria o checksum oficial ${asset}.sha256 da mesma release"
        if [ -n "$tool" ]; then
            dry "Verificaria a integridade com '$tool' ANTES de instalar"
        else
            warn "Nem 'sha256sum' nem 'shasum' existem aqui: a verificação seria PULADA (com aviso)."
        fi
        dry "  hash confere        => chmod +x e instalação"
        dry "  hash diverge        => apaga o download, erro e abort (possível adulteração)"
        dry "  .sha256 inexistente => avisa que não deu para verificar e prossegue"
        dry "Instalaria: install -m 0755 <tmpfile> $INSTALL_DIR/fzcomputerai"
        dry "Fallback sem release/asset: git clone --depth 1 ${REPO_URL}.git + cargo build --release"
        return 0
    fi

    info "Consultando a API oficial do GitHub Releases: $API_LATEST"
    local download_url="" tag=""
    API_JSON="$(curl -fsSL -H 'Accept: application/vnd.github+json' "$API_LATEST" 2>/dev/null || true)"

    if [ -n "$API_JSON" ]; then
        download_url="$(find_asset_url "$asset")"
        if command -v jq >/dev/null 2>&1; then
            tag="$(printf '%s' "$API_JSON" | jq -r '.tag_name // empty' 2>/dev/null || true)"
        else
            tag="$(printf '%s\n' "$API_JSON" | tr ',' '\n' \
                | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' \
                | head -n1 | sed 's/.*: *"\([^"]*\)"$/\1/' || true)"
        fi
    fi

    if [ -z "$download_url" ]; then
        warn "Nenhum release/asset oficial encontrado para $asset. Ativando fallback de compilação."
        build_from_source
        return 0
    fi

    info "Baixando binário oficial ${tag:+($tag) }de: $download_url"
    TMP_BIN_FILE="$(mktemp)"
    if ! curl -fL -o "$TMP_BIN_FILE" "$download_url"; then
        warn "Falha no download do binário oficial. Ativando fallback de compilação."
        build_from_source
        return 0
    fi

    # Os binários não são assinados: o .sha256 da release é a única garantia de
    # integridade disponível hoje. Conferir ANTES de instalar.
    if ! verify_download_checksum "$TMP_BIN_FILE" "$asset"; then
        rm -f "$TMP_BIN_FILE" 2>/dev/null || true
        TMP_BIN_FILE=""
        exit 1
    fi

    chmod +x "$TMP_BIN_FILE"
    install_binary "$TMP_BIN_FILE" "fzcomputerai"
}

# ------------------------------------------------------------------------------
# Modo local — comportamento clássico dentro do checkout do repositório
# ------------------------------------------------------------------------------
write_mcp_json() {
    local mcp_json="$SCRIPT_DIR/.mcp.json"
    info "Criando/atualizando configuração MCP em $mcp_json..."
    if [ "$DRY_RUN" -eq 1 ]; then
        dry "Escreveria $mcp_json com o servidor fz-computer-vision (cua-driver mcp)"
        return 0
    fi
    cat <<'EOF' > "$mcp_json"
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
    success "Configuração .mcp.json criada."
}

local_install() {
    local bin_path=""

    # Compilar a GUI Rust (fzcomputerai/Cargo.toml existe — é o gatilho do modo local)
    if [ "$HAS_CARGO" -eq 1 ]; then
        check_linux_build_deps
        info "Compilando a interface gráfica FzComputerAI (GUI Rust)..."
        run cargo build --release --manifest-path "$SCRIPT_DIR/fzcomputerai/Cargo.toml" \
            || warn "Falha ao compilar GUI Rust."
    else
        warn "Rust/Cargo não encontrado; a GUI não será compilada. Instale via https://rustup.rs"
    fi

    # Compilar o motor cua-driver, se o workspace existir
    local rust_workspace="$SCRIPT_DIR/cua/libs/cua-driver/rust/Cargo.toml"
    if [ ! -f "$rust_workspace" ]; then
        rust_workspace="$SCRIPT_DIR/libs/cua-driver/rust/Cargo.toml"
    fi

    if [ "$HAS_CARGO" -eq 1 ] && [ -f "$rust_workspace" ]; then
        info "Compilando o motor cua-driver via Cargo (--release)..."
        run cargo build --release --package cua-driver --manifest-path "$rust_workspace"
        if [ -f "$(dirname "$rust_workspace")/target/release/cua-driver" ]; then
            bin_path="$(dirname "$rust_workspace")/target/release/cua-driver"
            success "Binário compilado com sucesso: $bin_path"
        fi
    fi

    if [ -z "$bin_path" ]; then
        if [ "$HAS_CUA" -eq 1 ]; then
            bin_path="$(command -v cua-driver)"
            info "Utilizando binário do PATH: $bin_path"
        else
            info "Baixando e instalando pacote oficial do Cua Driver..."
            if [ "$DRY_RUN" -eq 1 ]; then
                dry "curl -fsSL https://cua.ai/driver/install.sh | bash"
            else
                curl -fsSL https://cua.ai/driver/install.sh | bash
            fi
            bin_path="$HOME/.cua-driver/packages/current/cua-driver"
        fi
    fi

    write_mcp_json

    # Testar diagnóstico
    info "Verificando saúde do sistema (cua-driver doctor)..."
    if [ "$DRY_RUN" -eq 1 ]; then
        dry "cua-driver doctor"
    elif command -v cua-driver >/dev/null 2>&1; then
        cua-driver doctor || true
    elif [ -x "$bin_path" ]; then
        "$bin_path" doctor || true
    fi
}

# ------------------------------------------------------------------------------
# Execução
# ------------------------------------------------------------------------------
if [ "$MODE" = "local" ]; then
    local_install
else
    remote_install
    warn_path_if_needed
    echo ""
    info "Para usar como servidor MCP (ex.: Claude Code), adicione ao seu .mcp.json:"
    cat <<'EOF'
{
  "mcpServers": {
    "fz-computer-vision": {
      "command": "cua-driver",
      "args": ["mcp"],
      "env": { "RUST_LOG": "info" }
    }
  }
}
EOF
    info "Se o cua-driver não estiver instalado, use: npx fzcomputerai mcp"
fi

echo ""
echo -e "${CYAN}======================================================================${NC}"
if [ "$DRY_RUN" -eq 1 ]; then
    echo -e "${YELLOW}   Simulação (--dry-run) concluída. Nada foi alterado no sistema.${NC}"
else
    echo -e "${GREEN}   Instalação do FzComputerAI concluída com sucesso!${NC}"
fi
echo -e "${YELLOW}   O Servidor de Computer Vision via MCP está pronto para uso.${NC}"
echo -e "${CYAN}   GUI: fzcomputerai${NC}"
echo -e "${CYAN}   Comando MCP stdio: cua-driver mcp${NC}"
echo -e "${CYAN}   Comando CLI NPM: npx fzcomputerai mcp${NC}"
echo -e "${CYAN}======================================================================${NC}"
