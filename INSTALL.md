# Guia de Instalação e Configuração — Computer Vision via MCP

Este guia contém as instruções passo a passo para instalar, compilar e configurar o servidor de **Visão Computacional e Automação de Interface via MCP (Model Context Protocol)** no Windows, Linux e macOS.

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

## ⚡ 2. Instalação Automática (Modo Padrão)

### No Windows (PowerShell)
Execute o script de instalação no PowerShell:

```powershell
# Executar a partir da raiz do repositório
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

O script realizará automaticamente:
1. Verificação do compilador Rust/Cargo ou busca do executável compilado.
2. Compilação do motor `cua-driver` em modo `--release` (se Rust estiver instalado).
3. Adição do diretório de binários ao `PATH` do usuário.
4. Criação do arquivo de configuração `.mcp.json` na pasta do projeto.
5. Diagnóstico de saúde do ambiente (`cua-driver doctor`).

### No Linux / macOS (Bash)
Execute o script Shell:

```bash
chmod +x ./install.sh
./install.sh
```

---

## 🔧 3. Instalação Avançada & Compilação Manual (Rust Cargo)

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

## 💻 4. Configuração nos Clientes MCP

Após concluir a instalação, conecte o servidor de visão computacional aos seus clientes de IA favoritos:

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
Execute o comando abaixo para adicionar o servidor MCP ao Claude Code:

```bash
claude mcp add --transport stdio fz-computer-vision -- cua-driver mcp
```

Para verificar se o servidor foi registrado corretamente:
```bash
claude mcp list
```

### C. Cursor / Windsurf / VS Code (Extensão MCP)
1. Abra as configurações do Cursor ou VS Code (`Ctrl+,` ou `Cmd+,`).
2. Acesse as configurações de **MCP Servers**.
3. Adicione a seguinte estrutura de configuração:

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

## 🔍 5. Diagnóstico e Resolução de Problemas (Troubleshooting)

### Testando a Comunicação do Servidor
Para iniciar o servidor MCP manualmente via linha de comando em modo interativo (stdio):

```bash
cua-driver mcp
```

### Verificação de Saúde (`doctor`)
Execute o diagnóstico integrado para identificar se há restrições de permissão ou falta de dependências:

```bash
cua-driver doctor
```

### Problemas Frequentes:

1. **`cua-driver` não é reconhecido como um comando interno ou externo:**
   - Feche e reabra o terminal/PowerShell após rodar o `install.ps1` para carregar a nova variável `PATH`.
2. **Erro de captura de tela no Windows:**
   - Certifique-se de que a sessão não está bloqueada na tela de Logon ou rodando via WinRM/SSH sem sessão gráfica ativa.
3. **Erro de permissão de gravação no macOS:**
   - Acesse *Ajustes do Sistema > Privacidade e Segurança > Gravação de Tela e Áudio do Sistema* e autorize o aplicativo do terminal ou o binário `cua-driver`.

---

## 📧 Suporte & Contato

Em caso de dúvidas ou necessidade de suporte técnico na implantação:
- **Autor:** Roger Luft
- **Empresa:** Webstorage Tecnologia
- **E-mail:** `roger@webstorage.com.br`
- **WhatsApp:** +55 51 99242539
