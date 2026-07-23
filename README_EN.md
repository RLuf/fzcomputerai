# FzComputerAI — Computer Vision via Model Context Protocol (MCP)

<div align="center">

![CC BY 4.0 License](https://img.shields.io/badge/License-CC%20BY%204.0-blue.svg)
![Platforms](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen.svg)
![MCP Enabled](https://img.shields.io/badge/MCP-Server%20Ready-orange.svg)
![HTTP TCP Transport](https://img.shields.io/badge/Transport-Stdio%20%7C%20HTTP%20TCP%20:8000-purple.svg)

<p align="center">
  <strong>Native Multimodal Computer Vision & Desktop Automation Server for AI Agents</strong>
</p>

[Português (BR)](README.md) | [English (US)](README_EN.md)

</div>

---

> **FzComputerAI** is a native **Computer Vision and UI Automation** server accessible via the **Model Context Protocol (MCP)**. Designed to empower AI Agents (such as Claude Code, Antigravity, FazAI-NG, Cursor, Windsurf, and local LLMs) to see the screen, analyze the visual structure of any desktop application, and execute actions with millimeter precision, both locally and remotely over **HTTP TCP/IP**.

---

## 💎 Sponsors & Support

<div align="center">

| Sponsor | Website | Focus |
| :--- | :--- | :--- |
| **Webstorage Tecnologia** | [www.webstorage.com.br](https://www.webstorage.com.br) | Infrastructure Solutions, Cloud & Intelligent Automation |
| **Imóvel Site** | [www.imovelsite.com.br](https://www.imovelsite.com.br) | Real Estate Management & PropTech Platform |

</div>

---

## 🚀 Key Features (Computer Vision via MCP)

The server exposes a standardized set of MCP tools (*MCP Tools*) for multimodal computer vision analysis and desktop control:

### 👁️ Vision & Visual Inspection

| MCP Tool | Description |
|---|---|
| `get_desktop_state` | Captures the full desktop image (Computer Vision), lists all active windows, coordinates, and cursor state. |
| `get_window_state` | Performs focused capture of a specific window and extracts the accessibility tree (UI Automation / Accessibility Tokens). |
| `take_screenshot` | Generates an optimized multimodal screenshot (PNG/JPEG base64) for direct consumption by vision models (Gemini 1.5/2.0, Claude 3.5 Sonnet/Opus, GPT-4o). |

### 🖱️ Pointer Actions & Automation

| MCP Tool | Description |
|---|---|
| `mouse_click` | Performs left, right, or middle mouse clicks at specific coordinates $(x, y)$ or on identified elements. |
| `mouse_move` | Moves the cursor to absolute desktop positions or relative positions within a target window. |
| `mouse_drag` | Executes drag and drop operations with smooth trajectory control. |
| `mouse_down` / `mouse_up` | Granular control for pressing and releasing mouse buttons. |

### ⌨️ Keyboard & Shortcuts

| MCP Tool | Description |
|---|---|
| `keyboard_type` | Simulates text typing with support for unicode sanitization and international accentuation. |
| `keyboard_press` | Sends individual keys or specific key combinations (e.g., `Enter`, `Tab`, `Escape`). |
| `shortcut` | Triggers complex system shortcuts (e.g., `Ctrl+C`, `Ctrl+V`, `Alt+Tab`, `Cmd+Space`). |

### 🛠️ Application Management & Recording

| MCP Tool | Description |
|---|---|
| `launch_app` | Launches system applications by name or executable path. |
| `close_app` | Closes running windows or processes. |
| `recording_start` / `recording_stop` | Starts and stops real-time screen session video recordings. |

---

## 🛠️ System Architecture & Connection Modes

```
  ┌────────────────────────────────────────────────────────────────────────┐
  │                   AI Agent / Remote Orchestrator                       │
  │        (Antigravity / FazAI-NG / Claude Code / Cursor / Windsurf)       │
  └───────────────────────────────────┬────────────────────────────────────┘
                                      │
           ┌──────────────────────────┴──────────────────────────┐
           │ Stdio Mode (Local)       │ HTTP TCP/IP Mode (:8000) │
           ▼                          ▼                          ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │               FzComputerAI — MCP Computer Vision Server                │
  │                           (cua-driver engine)                          │
  ├────────────────────────────────────┬───────────────────────────────────┤
  │       Screen Capture (WGC/DX)      │    Input Injection (SendInput)    │
  └────────────────────────────────────┴───────────────────────────────────┘
```

---

## 🌐 Remote Connection via HTTP TCP/IP (Orchestrators like FazAI-NG)

In addition to local `stdio` mode, the server supports remote connections via the **HTTP TCP/IP** protocol. This allows an orchestrator running on a separate server (e.g. Linux) to control desktop machines over the network:

### Enabling the HTTP Port on the Server (Windows):
```powershell
# Set TCP port 8000 for the MCP server
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '8000', 'User')
cua-driver stop
cua-driver autostart kick
```

### Configuring the HTTP Client / Orchestrator:
- **Endpoint**: `http://<WINDOWS_IP>:8000/mcp`
- **Method**: `POST`
- **Header**: `Content-Type: application/json`

---

## 📦 Quick Start

### Windows (PowerShell)
```powershell
# Direct execution of the installation script
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

### Linux & macOS (Bash)
```bash
chmod +x ./install.sh
./install.sh
```

For detailed source code compilation instructions and advanced configuration, refer to [INSTALL_EN.md](file:///g:/fzcomcontrol/INSTALL_EN.md).

---

## ⚙️ Local MCP Client Setup

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

## 📜 License & Credits

- **Base Project / Engine:** Based on the open-source project [Cua (`trycua/cua`)](https://github.com/trycua/cua) created by [Cua.ai](https://cua.ai).
- **Owner & Lead Developer:** Roger Luft — Webstorage Tecnologia (`roger@webstorage.com.br`)
- **Contact / Support:** +55 51 99242539
- **Sponsors:** [Webstorage Tecnologia](https://www.webstorage.com.br) | [Imóvel Site](https://www.imovelsite.com.br)
- **License:** [Creative Commons Attribution 4.0 International (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/)
