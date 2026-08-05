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
```

> **Não é preciso tarefa agendada.** Ao abrir, o `fzcomputerai.exe` sobe o motor sozinho, como **processo filho**
> (apenas quando nada está respondendo na porta), e o motor cai junto com o app — inclusive quando a GUI é fechada
> no X, pela bandeja ou por `taskkill /F`. Se já houver um motor de **outro** cliente MCP respondendo na porta, o app
> o detecta, **não** o duplica e **não** o encerra ao fechar (a UI avisa). O antigo `cua-driver autostart kick` foi
> removido do fluxo do aplicativo.

### Testando a porta HTTP:
```powershell
netstat -an | findstr 8000
# Deve retornar: TCP 127.0.0.1:8000 LISTENING
```

### No Cliente Remoto (FazAI-NG / Orquestrador):

> O motor escuta **somente em `127.0.0.1`** (é o que o `netstat` acima mostra). Para o endereço
> `http://<IP_DO_WINDOWS>:8000/mcp` funcionar de outra máquina, clique em **Publicar na rede** na aba *MCP & Rede*
> da GUI — ou use a aba **Túnel** para acesso pela internet.
>
> Desde a v2.3.0 a publicação na LAN é feita por um **relay TCP dentro do processo da GUI**: ele escuta em
> `0.0.0.0:<porta>` (ou no IP escolhido no campo *Escutar em*) e encaminha para `127.0.0.1:<porta>` do motor,
> copiando os bytes nos dois sentidos sem inspecionar o HTTP (keep-alive e SSE passam intactos). Diferenças
> medidas em relação à antiga regra `netsh portproxy`: **não pede UAC**, **não deixa regra no sistema** (a do
> `netsh` sobrevive a reboot) e **cai quando o app fecha**. A remoção de regra `portproxy` **legada** continua
> disponível na mesma aba e só aparece quando existe alguma.

Envie chamadas POST JSON-RPC para:
- **URL**: `http://<IP_DO_WINDOWS>:8000/mcp`
- **Body**: `{"jsonrpc":"2.0","id":1,"method":"tools/list"}`

> **Motores `cua-driver` 0.16+**: o endpoint HTTP exige o token `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32–4096 caracteres) e responde **401** a qualquer chamada sem o header `Authorization: Bearer <token>` — inclusive quando nenhum token foi configurado (fail-closed: 401 para tudo). Gere e ative o token pela aba **Túnel** da GUI (botão **Gerar e ativar token do motor**) e inclua o header nas chamadas. Versões antigas (<= 0.8.x) não têm autenticação.
>
> Esses motores também **recusam requisição com header `Origin` de navegador (HTTP 403)** — verificado. Chame do servidor/CLI, não da aba de um browser.
>
> Para clientes que aceitam **apenas uma URL** e não têm onde colar header (é o caso do Claude Desktop), use o túnel com **senha no caminho** (`/s/<senha>/mcp`, aba **Túnel**): quem provou a senha já está autenticado perante o app, e o porteiro **acrescenta o `Authorization` ao falar com o motor**. Se o cliente enviar o próprio `Authorization`, o dele prevalece. O segredo do motor não trafega pela internet; a credencial pública passa a ser a senha da URL.

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

> `cua-driver mcp` é **stdio** e encerra quando o `stdin` fecha (medido no 0.17) — por isso não serve para manter o
> endpoint HTTP de pé. O modo usado pela GUI é `cua-driver serve`, que também abre o pipe `\\.\pipe\cua-driver`
> (o canal que o próprio CLI usa em `call`/`status`/`stop`). O HTTP só liga com `CUA_DRIVER_RS_MCP_HTTP_PORT` no
> ambiente.

### Teste de fora da rede (`scripts/teste_remoto_mcp.py`)

Script de verificação ponta a ponta, escrito **só com a biblioteca padrão do Python 3** (nada para instalar). Ele
faz `initialize`, `tools/list`, abre uma **janela nova** de navegador na máquina remota (nunca sequestra uma janela
já aberta), navega até `search.yahoo.com`, digita o termo, localiza e clica no botão de pesquisa
(Search/Pesquisar/Buscar) ou envia Enter, e confere o resultado lendo a tela de volta.

```bash
python scripts/teste_remoto_mcp.py <URL> [--token TOKEN] [--termo TEXTO]
```

Se a URL já tiver a senha no caminho (`/s/<senha>/mcp`), o `--token` não é necessário.

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
