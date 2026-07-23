# FzComputerAI — Computer Vision via Model Context Protocol (MCP)

![Licença CC BY 4.0](https://img.shields.io/badge/Licen%C3%A7a-CC%20BY%204.0-blue.svg)
![Plataformas](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen.svg)
![MCP Enabled](https://img.shields.io/badge/MCP-Server%20Ready-orange.svg)

> **FzComputerAI** é um servidor nativo de **Visão Computacional e Automação de Interface (UI)** acessível via **Model Context Protocol (MCP)**. Projetado para capacitar Agentes de Inteligência Artificial (como Claude Code, Antigravity, Cursor, Windsurf e LLMs locais) a enxergar a tela, analisar a estrutura visual de qualquer aplicativo de desktop e executar ações com precisão milimétrica, em primeiro plano ou em segundo plano.

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

## 🛠️ Arquitetura do Sistema

```
  ┌─────────────────────────────────────────────────────────┐
  │                 Agente de IA (MCP Client)               │
  │     (Antigravity / Claude Code / Cursor / Windsurf)     │
  └────────────────────────────┬────────────────────────────┘
                               │ JSON-RPC (Stdio / SSE)
                               ▼
  ┌─────────────────────────────────────────────────────────┐
  │         FzComputerAI — MCP Computer Vision Server       │
  │                     (cua-driver engine)                 │
  ├────────────────────────────┬────────────────────────────┤
  │     Captura de Tela        │     Injeção de Input       │
  │   (Windows WGC/DirectX /   │   (SendInput / X11 /     │
  │     macOS Quartz / X11)    │      CoreGraphics)         │
  └────────────────────────────┴────────────────────────────┘
```

O servidor é construído em **Rust** para garantir performance de baixíssima latência (< 50ms por frame) e consumo reduzido de memória.

---

## 📦 Instalação Rápida

### Windows (PowerShell)
```powershell
# Execução direta do script de instalação
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

### Linux & macOS (Bash)
```bash
chmod +x ./install.sh
./install.sh
```

Para instruções detalhadas de compilação a partir do código fonte e configurações avançadas, consulte o [INSTALL.md](file:///g:/fzcomcontrol/INSTALL.md).

---

## ⚙️ Configuração nos Clientes MCP

### 1. Antigravity / Gemini CLI (`.mcp.json`)
Adicione a seguinte entrada na configuração do seu workspace ou em `~/.gemini/config`:

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
No arquivo `mcp.json` da sua IDE, adicione:
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

## 📜 Licença & Créditos

- **Titular & Desenvolvedor:** Roger Luft — Webstorage Tecnologia (`roger@webstorage.com.br`)
- **Contato / Suporte:** +55 51 99242539
- **Licença:** [Creative Commons Attribution 4.0 International (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/)

Você tem a liberdade de compartilhar e adaptar este projeto para qualquer fim, desde que atribuído o devido crédito ao autor original.
