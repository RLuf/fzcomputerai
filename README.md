# FzComputerAI — Computer Vision via Model Context Protocol (MCP)

<div align="center">

![GitHub Release](https://img.shields.io/github/v/release/RLuf/fzcomputerai)
![CC BY 4.0 License](https://img.shields.io/badge/Licen%C3%A7a-CC%20BY%204.0-blue.svg)
![Plataformas](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen.svg)
![MCP Enabled](https://img.shields.io/badge/MCP-Server%20Ready-orange.svg)
![HTTP TCP Transport](https://img.shields.io/badge/Transport-Stdio%20%7C%20HTTP%20TCP%20:8000-purple.svg)

<p align="center">
  <strong>Servidor Nativo de Visão Computacional Multimodal & Automação Desktop para Agentes de IA</strong>
</p>

[Português (BR)](README.md) | [English (US)](README_EN.md)

</div>

---

> **FzComputerAI** é um servidor nativo de **Visão Computacional e Automação de Interface (UI)** acessível via **Model Context Protocol (MCP)**. Projetado para capacitar Agentes de Inteligência Artificial (como Claude Code, Antigravity, FazAI-NG, Cursor, Windsurf e LLMs locais) a enxergar a tela, analisar a estrutura visual de qualquer aplicativo de desktop e executar ações com precisão milimétrica, tanto localmente quanto via rede **TCP/IP HTTP**.

---

## 💎 Patrocinadores & Apoio

<div align="center">

| Patrocinador | Website | Foco |
| :--- | :--- | :--- |
| **Webstorage Tecnologia** | [www.webstorage.com.br](https://www.webstorage.com.br) | Soluções em Infraestrutura, Cloud & Automação Inteligente |
| **Imóvel Site** | [www.imovelsite.com.br](https://www.imovelsite.com.br) | Plataforma de Gestão e Tecnologia Imobiliária |

</div>

---

## 🚀 Recursos Principais (Computer Vision via MCP)

O servidor expõe um conjunto de ferramentas MCP (*MCP Tools*) padronizadas para análise de visão computacional multimodal e controle de desktop:

### 👁️ Visão & Inspeção Visual

| Ferramenta MCP | Descrição |
|---|---|
| `get_desktop_state` | Captura a imagem completa do Desktop (Visão Computacional), lista todas as janelas ativas, coordenadas e estado do cursor. |
| `get_window_state` | Realiza a captura focada de uma janela específica e extrai a árvore de acessibilidade (UI Automation / Accessibility Tokens). |
| `take_screenshot` | Gera uma captura de tela multimodal otimizada (PNG/JPEG base64) para consumo direto por modelos de visão (Gemini 1.5/2.0, Claude 3.5 Sonnet/Opus, GPT-4o). |

### 🖱️ Ações de Ponteiro & Automação

| Ferramenta MCP | Descrição |
|---|---|
| `mouse_click` | Executa cliques com o botão esquerdo, direito ou do meio em coordenadas específicas $(x, y)$ ou sobre elementos identificados. |
| `mouse_move` | Move o cursor para posições absolutas no desktop ou relativas dentro de uma janela. |
| `mouse_drag` | Executa movimentos de arrastar e soltar (Drag and Drop) com controle suave de trajetória. |
| `mouse_down` / `mouse_up` | Controle granular de pressionar e soltar botões do mouse. |

### ⌨️ Teclado & Atalhos

| Ferramenta MCP | Descrição |
|---|---|
| `keyboard_type` | Simula a digitação de texto com suporte a sanitização de caracteres unicode e acentuação PT-BR. |
| `keyboard_press` | Envia teclas individuais ou combinações específicas (ex: `Enter`, `Tab`, `Escape`). |
| `shortcut` | Executa atalhos de sistema complexos (ex: `Ctrl+C`, `Ctrl+V`, `Alt+Tab`, `Cmd+Space`). |

### 🛠️ Gerenciamento de Aplicações & Gravação

| Ferramenta MCP | Descrição |
|---|---|
| `launch_app` | Inicia aplicativos do sistema por nome ou caminho executável. |
| `close_app` | Encerra janelas ou processos em execução. |
| `recording_start` / `recording_stop` | Inicia e finaliza gravações de vídeo da sessão em tempo real. |

---

## 🖥️ Interface Gráfica Nativa (GUI Rust `fzcomputerai`)

GUI nativa em Rust (`egui`/`eframe`, sem Chromium ou WebView), bilíngue **PT-BR / English** com alternância em tempo real. Organizada em **5 abas**:

| Aba | Função |
| :--- | :--- |
| **MCP & Rede** | Configuração da porta HTTP do servidor MCP (`CUA_DRIVER_RS_MCP_HTTP_PORT`), regra portproxy (com elevação UAC quando necessária), teste real do endpoint `/mcp` via TCP, URL de rede com IP LAN detectado automaticamente e botão **Copiar**, opção **Iniciar com o Windows** (autostart) e **Console Debug** integrado. |
| **Calibração & Visão** | Calibração de tela, DPI scaling e teste de clique por coordenadas. |
| **Janelas & Processos** | Listagem de janelas ativas, inspeção UIA e lançamento de aplicativos. |
| **Gravação Trajetória** | Início e parada de gravações de sessão/trajetória. |
| **Doctor & Skills** | Diagnóstico de saúde (`doctor`) e instalação/atualização/remoção de skills. |

Destaques:
- **Console Debug**: todo comando executado pela GUI é registrado (comando, exit code, stdout, stderr) em painel dedicado na aba MCP & Rede.
- **Status honesto**: o estado da porta/daemon exibido vem de um teste TCP real no endpoint `/mcp`, não de suposições.
- **Iniciar com o Windows**: checkbox que registra/remove a GUI no autostart do usuário (`HKCU\...\Run`).
- **Copiar URL MCP**: um clique copia `http://<IP_LAN>:<porta>/mcp` para a área de transferência.

---

## 🛠️ Arquitetura do Sistema & Modos de Conexão

```
  ┌────────────────────────────────────────────────────────────────────────┐
  │                 Agente de IA / Orquestrador Remoto                     │
  │        (Antigravity / FazAI-NG / Claude Code / Cursor / Windsurf)       │
  └───────────────────────────────────┬────────────────────────────────────┘
                                      │
           ┌──────────────────────────┴──────────────────────────┐
           │ Modo Stdio (Local)       │ Modo HTTP TCP/IP (:8000) │
           ▼                          ▼                          ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │               FzComputerAI — MCP Computer Vision Server                │
  │                           (cua-driver engine)                          │
  ├────────────────────────────────────┬───────────────────────────────────┤
  │       Captura de Tela (WGC/DX)     │    Injeção de Input (SendInput)   │
  └────────────────────────────────────┴───────────────────────────────────┘
```

---

## 🌐 Conexão Remota via TCP/IP HTTP (Orquestradores como FazAI-NG)

Além do modo local `stdio`, o servidor suporta conexão remota via protocolo **HTTP TCP/IP**. Isso permite que um orquestrador rodando em um servidor separado (ex: Linux) controle desktops na rede:

### Ativando a Porta HTTP no Servidor (Windows):
```powershell
# Ativar porta TCP 8000 para o servidor MCP
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '8000', 'User')
cua-driver stop
cua-driver autostart kick
```

### Configurando o Cliente HTTP / Orquestrador:
- **Endpoint**: `http://<IP_DO_WINDOWS>:8000/mcp`
- **Método**: `POST`
- **Header**: `Content-Type: application/json`

---

## 📦 Instalação Rápida

### 1. Via NPM (Global)
```bash
npm install -g fzcomputerai
```

### 2. Instalação Remota via PowerShell (Windows One-liner)
```powershell
iwr -useb https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.ps1 | iex
```

### 3. Instalação Remota via Bash (Linux & macOS One-liner)
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash
```

Para simular a instalação sem alterar nada no sistema (`--dry-run`):
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash -s -- --dry-run
```

> **Nota:** o instalador remoto via binário oficial instala a **GUI `fzcomputerai`**. O servidor MCP stdio continua sendo o `cua-driver` — o script imprime o snippet `.mcp.json` correspondente e orienta o uso de `npx fzcomputerai mcp` (o fallback de compilação a partir do código-fonte também compila o `cua-driver`).

### 4. Compilação a partir do Código Fonte / Pacote Tarball (.tgz)
```bash
# Baixar ou extrair o pacote de código-fonte .tgz:
tar -xzf fzcomputerai-<versão>.tgz
cd package (ou fzcomputerai)

# Compilação do motor e da GUI Rust:
cargo build --release --manifest-path fzcomputerai/Cargo.toml
```

Para instruções detalhadas de compilação e configurações avançadas, consulte o [INSTALL.md](file:///g:/fzcomcontrol/INSTALL.md).

---

## ⚙️ Configuração nos Clientes MCP Locais

### 1. Antigravity / Gemini CLI (`.mcp.json`)
```json
{
  "mcpServers": {
    "fz-computer-vision": {
      "command": "cua-driver",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### 2. Claude Code CLI
```bash
claude mcp add --transport stdio fz-computer-vision -- cua-driver mcp
```

### 3. Cursor / Windsurf / VS Code
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

## 🤝 Patrocinadores Oficiais & Apoio (Patrons)

<div align="center">

| Patrocinador | Logo | Website Oficial |
| :--- | :---: | :--- |
| **Webstorage Tecnologia** | <a href="https://www.webstorage.com.br"><img src="assets/img/webstorage-logo.png" width="180" alt="Webstorage Tecnologia"></a> | [www.webstorage.com.br](https://www.webstorage.com.br) |
| **Imóvel Site** | <a href="https://www.imovelsite.com.br"><img src="assets/img/imovelsite-logo.png" width="180" alt="Imóvel Site"></a> | [www.imovelsite.com.br](https://www.imovelsite.com.br) |

</div>

---

## 📜 Licença & Créditos

- **Projeto Base / Motor Original:** Baseado no projeto open-source [Cua (`trycua/cua`)](https://github.com/trycua/cua) desenvolvido pela equipe [Cua.ai](https://cua.ai).
- **Titular & Integrações FzComputerAI:** Roger Luft — Webstorage Tecnologia (`roger@webstorage.com.br`)
- **Contato / Suporte:** +55 51 99242539
- **Patrocinadores:** [Webstorage Tecnologia](https://www.webstorage.com.br) | [Imóvel Site](https://www.imovelsite.com.br)
- **Licença:** [Creative Commons Attribution 4.0 International (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/)
