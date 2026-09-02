# Guia de Instalação e Configuração — Computer Vision via MCP

Este guia contém as instruções passo a passo para instalar, compilar e configurar o servidor de **Visão Computacional e Automação de Interface via MCP (Model Context Protocol)** no Windows, Linux e macOS.

---

## 💎 Patrocinadores do Projeto

- **Webstorage Tecnologia** — [www.webstorage.com.br](https://www.webstorage.com.br)
- **Imóvel Site** — [www.imovelsite.com.br](https://www.imovelsite.com.br)

---

## 📋 1. Pré-Requisitos

### Windows
- **Sistema Operacional:** Windows 10 / 11 (64-bit) ou Windows Server 2019+
- **Shell:** PowerShell 5.1 ou PowerShell 7+
- **Compilador (opcional para build local):** Rust e Cargo (`rustc 1.75+`)
  ```powershell
  # Instalação do Rust no Windows (se desejar compilar do código-fonte)
  winget install Rustlang.Rustup
  ```

### Linux
- **Distribuições suportadas:** Ubuntu 20.04+, Debian 11+, Fedora 36+, Arch Linux
- **Dependências de sistema (X11 / Wayland):**
  ```bash
  # Debian/Ubuntu
  sudo apt-get update && sudo apt-get install -y build-essential libx11-dev libxtst-dev libxcb1-dev
  ```

### macOS
- **Sistema Operacional:** macOS 12 Monterey ou superior (Intel / Apple Silicon M1/M2/M3)
- **Permissões exigidas:** Permissão de **Gravação de Tela** e **Acessibilidade** em *Ajustes do Sistema > Privacidade e Segurança*.

---

## ⚡ 2. Métodos de Instalação

### A. Via NPM (Gerenciador de Pacotes Node.js)
```bash
npm install -g fzcomputerai
```

### B. Windows — Instalador Gráfico (único caminho de instalação no Windows)

> O antigo `install.ps1` da raiz do repositório foi **removido** — a instalação no Windows agora é exclusivamente pelo instalador gráfico (Inno Setup).

1. Baixe o **`fzcomputerai-setup-windows-x64.exe`** em [https://github.com/RLuf/fzcomputerai/releases/latest](https://github.com/RLuf/fzcomputerai/releases/latest).
2. Execute o arquivo. Como os binários ainda não são assinados, o SmartScreen exibirá *"O Windows protegeu o seu PC"* — clique em **Mais informações → Executar assim mesmo**.
3. Durante a instalação, marque a task **"Instalar o motor `cua-driver`"** (necessário para o servidor MCP; requer internet) — ela executa o instalador oficial do projeto cua.
4. Ao final, o setup gera o **certificado auto-assinado do endpoint HTTPS** (`fzcomputerai --tls-init`, em `%APPDATA%\FzComputerAI\tls\`) — se não gerar, a GUI gera no primeiro run. Nada é instalado em store de confiança. O listener HTTPS é ligado depois, na aba *MCP & Rede* → **HTTPS** (ver [docs/https.md](docs/https.md)).

### C. Instalação Remota via Bash (Linux & macOS One-liner)
```bash
curl -fsSL https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.sh | bash
```

### D. Instalação Local a partir do Código-Fonte

#### No Windows (PowerShell) — gerar o instalador gráfico localmente
Quem compila do fonte gera o **mesmo instalador gráfico** do release (requer [Inno Setup](https://jrsoftware.org/isinfo.php) com `ISCC.exe` acessível):
```powershell
# 1. Compilar a GUI
cargo build --release --manifest-path fzcomputerai/Cargo.toml

# 2. Gerar o instalador (saída em dist\fzcomputerai-setup-windows-x64.exe)
ISCC.exe /DAppVersion=<versao> installer\fzcomputerai.iss
```

#### No Linux / macOS (Bash)
```bash
chmod +x ./install.sh
./install.sh
```

O script `install.sh` realizará automaticamente:
1. Verificação do compilador Rust/Cargo e compilação do motor `cua-driver` e da GUI `fzcomputerai`.
2. Configuração das variáveis de ambiente e adição dos binários ao `PATH`.
3. Ativação automática da variável `CUA_DRIVER_RS_MCP_HTTP_PORT=8000` para suporte nativo a HTTP TCP/IP.
4. Criação do arquivo de configuração `.mcp.json`.
5. Diagnóstico de saúde do ambiente (`cua-driver doctor`).

---

## 🌐 3. Configuração do Transporte HTTP TCP/IP (Orquestradores Remotos / FazAI-NG)

Para permitir que agentes rodando em servidores remotos (como o **FazAI-NG**) enviem chamadas JSON-RPC via rede TCP/IP:

### No Windows (Servidor Alvo a ser controlado):
```powershell
# Configura a variável no ambiente de usuário
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '8000', 'User')

# Reinicia o serviço de inicialização do cua-driver
cua-driver stop
cua-driver autostart kick
```

### Testando a porta HTTP:
```powershell
netstat -an | findstr 8000
# Deve retornar: TCP 127.0.0.1:8000 LISTENING
```

### No Cliente Remoto (FazAI-NG / Orquestrador):
Envie chamadas POST JSON-RPC para:
- **URL**: `http://<IP_DO_WINDOWS>:8000/mcp`
- **Body**: `{"jsonrpc":"2.0","id":1,"method":"tools/list"}`
- **HTTPS** (v2.2.0, opcional): ligue em *MCP & Rede* → HTTPS e use `https://<IP_DO_WINDOWS>:8443/mcp`; com auto-assinado, passe o `.crt` ao cliente (`curl --cacert`); com Let's Encrypt use o domínio. Guia: [docs/https.md](docs/https.md).

---

## 🔧 4. Instalação Avançada & Compilação Manual (Rust Cargo)

Se você deseja compilar o servidor de visão computacional diretamente a partir do código-fonte nativo em Rust:

### Passo 1: Navegar até a workspace Rust
```bash
cd cua/libs/cua-driver/rust
```

### Passo 2: Compilar em modo Release
```bash
cargo build --release --package cua-driver
```

O binário executável será gerado em:
- **Windows:** `cua/libs/cua-driver/rust/target/release/cua-driver.exe`
- **Linux/macOS:** `cua/libs/cua-driver/rust/target/release/cua-driver`

### Passo 3: Testar o executável
```bash
./target/release/cua-driver doctor
```

---

## 💻 5. Configuração nos Clientes MCP Locais

### A. Antigravity / Gemini CLI
Crie ou edite o arquivo `.mcp.json` no diretório raiz do projeto:

```json
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
```

### B. Claude Code CLI
```bash
claude mcp add --transport stdio fz-computer-vision -- cua-driver mcp
```

### C. Cursor / Windsurf / VS Code (Extensão MCP)
No arquivo de configuração de servidores MCP da IDE, adicione:

```json
{
  "mcpServers": {
    "fz-computer-vision": {
      "command": "cua-driver",
      "args": ["mcp"]
    }
  }
}
```

---

## 🔍 6. Diagnóstico e Resolução de Problemas (Troubleshooting)

### Testando a Comunicação do Servidor
Para iniciar o servidor MCP manualmente via linha de comando em modo interativo (stdio):

```bash
cua-driver mcp
```

### Verificação de Saúde (`doctor`)
```bash
cua-driver doctor
```

---

## 📧 Suporte & Contato

- **Autor:** Roger Luft
- **Empresa:** Webstorage Tecnologia (`www.webstorage.com.br`)
- **Parceiro:** Imóvel Site (`www.imovelsite.com.br`)
- **E-mail:** `roger@webstorage.com.br`
- **WhatsApp:** +55 51 99242539
