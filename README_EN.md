# FzComputerAI — Computer Vision via Model Context Protocol (MCP)

<div align="center">

![GitHub Release](https://img.shields.io/github/v/release/RLuf/fzcomputerai)
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

## 🖥️ Native GUI (Rust `fzcomputerai`)

Native Rust GUI (`egui`/`eframe`, no Chromium or WebView), bilingual **PT-BR / English** with real-time language toggle. Organized into **5 tabs**:

| Tab | Purpose |
| :--- | :--- |
| **MCP & Network** | MCP server HTTP port configuration (`CUA_DRIVER_RS_MCP_HTTP_PORT`), portproxy rule (with UAC elevation when required), real `/mcp` endpoint test over TCP, network URL with auto-detected LAN IP and a **Copy** button, **Start with Windows** (autostart) option, and a built-in **Debug Console**. |
| **Calibration & Vision** | Screen calibration, DPI scaling, and coordinate click testing. |
| **Windows & Apps** | Active window listing, UIA inspection, and application launching. |
| **Recording Trajectory** | Start and stop session/trajectory recordings. |
| **Doctor & Skills** | Health diagnostics (`doctor`) and skill install/update/uninstall. |

Highlights:
- **Debug Console**: every command executed by the GUI is logged (command, exit code, stdout, stderr) in a dedicated panel on the MCP & Network tab.
- **Honest status**: the displayed port/daemon state comes from a real TCP test against the `/mcp` endpoint, not assumptions.
- **Start with Windows**: checkbox that registers/removes the GUI from the user's autostart (`HKCU\...\Run`).
- **Copy MCP URL**: one click copies `http://<LAN_IP>:<port>/mcp` to the clipboard.

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

### 1. Via NPM (Global)
```bash
npm install -g fzcomputerai
```

### 2. Remote Installation via PowerShell (Windows One-liner)
```powershell
iwr -useb https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.ps1 | iex
```

### 3. Remote Installation via Bash (Linux & macOS One-liner)
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash
```

To simulate the installation without changing anything on your system (`--dry-run`):
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash -s -- --dry-run
```

> **Note:** the remote installer using the official release binary installs the **`fzcomputerai` GUI**. The stdio MCP server is still `cua-driver` — the script prints the corresponding `.mcp.json` snippet and points you to `npx fzcomputerai mcp` (the source-build fallback also compiles `cua-driver`).

### 4. Compilation from Source Code / Tarball (.tgz)
```bash
# Download or extract source code tarball (.tgz):
tar -xzf fzcomputerai-<version>.tgz
cd package (or fzcomputerai)

# Build engine and Rust GUI:
cargo build --release --manifest-path fzcomputerai/Cargo.toml
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

## 🤝 Official Sponsors & Patrons

<div align="center">

| Sponsor | Logo | Official Website |
| :--- | :---: | :--- |
| **Webstorage Tecnologia** | <a href="https://www.webstorage.com.br"><img src="assets/img/webstorage-logo.png" width="180" alt="Webstorage Tecnologia"></a> | [www.webstorage.com.br](https://www.webstorage.com.br) |
| **Imóvel Site** | <a href="https://www.imovelsite.com.br"><img src="assets/img/imovelsite-logo.png" width="180" alt="Imóvel Site"></a> | [www.imovelsite.com.br](https://www.imovelsite.com.br) |

</div>

---

## 📜 License & Credits

- **Base Project / Engine:** Based on the open-source project [Cua (`trycua/cua`)](https://github.com/trycua/cua) created by [Cua.ai](https://cua.ai).
- **Owner & Lead Developer:** Roger Luft — Webstorage Tecnologia (`roger@webstorage.com.br`)
- **Contact / Support:** +55 51 99242539
- **Sponsors:** [Webstorage Tecnologia](https://www.webstorage.com.br) | [Imóvel Site](https://www.imovelsite.com.br)
- **License:** [Creative Commons Attribution 4.0 International (CC BY 4.0)](https://creativecommons.org/licenses/by/4.0/)
