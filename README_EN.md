# FzComputerAI — Computer Vision via Model Context Protocol (MCP)

<div align="center">

![GitHub Release](https://img.shields.io/github/v/release/RLuf/fzcomputerai)
![MIT License](https://img.shields.io/badge/License-MIT-green.svg)
![Platforms](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen.svg)
![MCP Enabled](https://img.shields.io/badge/MCP-Server%20Ready-orange.svg)
![HTTP TCP Transport](https://img.shields.io/badge/Transport-Stdio%20%7C%20HTTP%20TCP%20:8000-purple.svg)
[![Sponsor](https://img.shields.io/badge/%E2%99%A5-Sponsor-e91e63.svg)](https://github.com/sponsors/RLuf)

<p align="center">
  <strong>Native Multimodal Computer Vision & Desktop Automation Server for AI Agents</strong>
</p>

[Português (BR)](README.md) | [English (US)](README_EN.md)

</div>

---

> **FzComputerAI** is the **native graphical interface** that manages a **Computer Vision and UI Automation** server accessible via the **Model Context Protocol (MCP)**. It lets AI Agents (Claude Code, Antigravity, FazAI-NG, Cursor, Windsurf, local LLMs) see the screen and operate any desktop application — and it handles the tedious part: starting the engine, configuring the port, proving the endpoint really answers, and publishing access over the LAN or the internet safely.
>
> The automation engine is **`cua-driver`**, from the open-source [**Cua**](https://github.com/trycua/cua) project (MIT, Cua AI, Inc.). FzComputerAI **does not replace the engine** — it is the cockpit for it.

---

## 🖼️ The tool

<div align="center">

**MCP & Network** — engine control, port, forwarding and diagnostics with **real** state (nothing assumed)

![MCP & Network tab](assets/img/screenshot-mcp-rede.png)

**Tunnel (Internet)** — publishes the local MCP on an HTTPS URL via Cloudflare, ngrok or reverse SSH

![Tunnel tab](assets/img/screenshot-tunel.png)

**MCP Tools** — catalog of the engine's tools, with filtering and one-click execution

![MCP Tools tab](assets/img/screenshot-mcp-tools.png)

</div>

---

## ✨ What the interface delivers

| Feature | What it does |
| :--- | :--- |
| **Engine lifecycle** | Start, stop and restart `cua-driver` in one click, with Windows autostart. Closing the app shuts the engine down and undoes temporary configuration. |
| **Honest status** | Nothing is assumed: the check is a real JSON-RPC `POST initialize`, and the LAN green badge only appears with a listener confirmed in `netstat` **and** the endpoint answering. |
| **LAN access** | `netsh portproxy` forwarding applied by the app (elevating only when needed), a **3-state** badge (working / no effect / no rule) and tracked cleanup — it removes only the rules it created itself. |
| **Internet access** | **Tunnel** tab: Cloudflare Tunnel (quick or named), ngrok and reverse SSH. **Outbound** tunnel — no router port forwarding required. |
| **URL password** | Level-1 authentication through a local gate: the URL becomes `https://…/s/<password>/mcp`, and without the password requests get a 404. |
| **Exposure probe** | The app tests the **public URL** with a credential-less request and reports the verified result — exposed, blocked at the edge, or not verifiable. |
| **Tunnel never outlives the app** | Four cleanup layers (including a watchdog that acts on `taskkill /F` and crashes), killing only the process provably ours. |
| **Update Center** | Checks and updates **two** components: this interface (installer downloaded in the background with verified SHA256 — only the final swap asks for confirmation) and the **engine** (end-to-end automatic update through its own official API, `check-update` / `update --apply`, with fallback to the official Cua installer). |
| **MCP Tools catalog** | List, filter and run the vision and automation tools without leaving the interface. |
| **Single console** | One global console at the bottom, visible in every section, scrolling like `tail -f`: it follows on its own and pauses when you scroll up to read. |
| **Bilingual and native** | PT-BR / English in real time. Rust + `egui`, no Chromium, no WebView, no Node runtime. |

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

## 🖥️ Native GUI (Rust `fzcomputerai v2.1.0`)

Native Rust GUI (`egui`/`eframe`, no Chromium or WebView), bilingual **PT-BR / English** with real-time language toggle. Organized into **7 tabs**:

| Tab | Purpose |
| :--- | :--- |
| **MCP & Network** | MCP server HTTP port configuration (`CUA_DRIVER_RS_MCP_HTTP_PORT`), Windows PortProxy rule (netsh) connecting to confirmed CUA port, real `/mcp` endpoint test over TCP, network URL with auto-detected LAN IP, **Check & Update** button (GitHub Releases auto-installer), **Start with Windows** (autostart) option, and deduplicated **Debug Console** with auto-scroll. |
| **MCP Tools** | **[NEW v2.0.0]** Interactive visual catalog to list, filter by category, and run any MCP vision & automation tool directly. |
| **Tunnel (Internet)** | **[NEW v2.1.0]** Exposes the local MCP HTTP endpoint to the internet (public HTTPS -> local HTTP) via **Cloudflare Tunnel** (quick + named, OAuth login/token), **ngrok**, and **reverse SSH** (own server or localhost.run/serveo). Captures the public URL, builds the `mcpServers` snippet, and truly tests it with a `POST initialize` exposure probe. Level-1 authentication = **URL password** through a local gate (`/s/<password>/mcp`). Clean lifecycle: the tunnel never outlives the app. **The engine does have authentication of its own — measured on 2026-08-03 against `cua-driver` 0.17.0: every request without `Authorization: Bearer <token>` gets HTTP 401 `{"code":-32001,"message":"Authentication required"}`. The URL password is an extra layer at the edge, not a replacement for the token.** |
| **Calibration & Vision** | Screen calibration, DPI scaling, and coordinate click testing. |
| **Windows & Apps** | Active window listing, UIA inspection, and application launching. |
| **Recording Trajectory** | Start and stop session/trajectory recordings. |
| **Doctor & Skills** | Health diagnostics (`doctor`) and skill install/update/uninstall. |

v2.0.0 Stable Highlights:
- **MCP Tools Catalog**: run CLI calls for computer vision and automation tools directly from the GUI.
- **Smart Auto-Upgrade**: direct GitHub Releases check with background download and automatic installation.
- **Formatted Debug Console**: organized logs with 2 blank lines spacing and auto-scroll to bottom.
- **Installer with Auto-Cleanup**: terminates legacy processes and removes stale Registry keys before fresh installation.

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
# Mandatory token: any random string you generate yourself
# (the engine itself calls it a "host-generated bearer token" — no command generates it for you)
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_TOKEN', '<your-token>', 'User')
cua-driver stop
cua-driver autostart kick
```

> **Measured on 2026-08-03 against the `cua-driver` 0.17.0 binary** (an actual run, not documentation quoting documentation): without `CUA_DRIVER_RS_MCP_HTTP_TOKEN` in the process environment, `cua-driver serve` **does not even start** — it exits with code 1 and the message `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`.
>
> Watch out for autostart: the `cua-driver-serve` Scheduled Task (used by `autostart kick`) **inherits the logon environment**, so a token written after you logged in is only seen at the next logon — until then the daemon starts without a token, dies immediately and leaves the port silent. The v2.1.0 GUI works around this: when the kick does not open the port, it reads port and token from the registry, stops the previous daemon and launches `serve` with the variables injected into the child process (only **one** daemon may exist at a time).

### Configuring the HTTP Client / Orchestrator:
- **Endpoint**: `http://<WINDOWS_IP>:8000/mcp`
- **Method**: `POST`
- **Header**: `Content-Type: application/json`
- **Header**: `Authorization: Bearer <your-token>` — **mandatory**. Measured on 2026-08-03: without it, `POST /mcp`, `GET /mcp` and `GET /` all three return the same HTTP 401 with `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` (and **no** `WWW-Authenticate` header). The TCP connection itself is accepted normally — the refusal happens at the application layer. With the correct header: HTTP 200 carrying the `initialize` `result`.

---

## 📦 Quick Start

### 🪟 Windows — Installer (recommended)

Download **`fzcomputerai-setup-windows-x64.exe`** from the [releases page](https://github.com/RLuf/fzcomputerai/releases/latest) and run it.

The installer (Inno Setup, bilingual PT-BR / English) does the following:

- **Installs the GUI** into `%LOCALAPPDATA%\Programs\FzComputerAI` — the default case **does not trigger UAC**; you may choose to install for all users in the wizard (that path does elevate).
- **Creates a Start Menu shortcut** and, optionally, a Desktop shortcut.
- **"Start FzComputerAI with Windows" option** (autostart) — writes exactly the same `HKCU\...\Run` key used by the checkbox on the *MCP & Network* tab, so the GUI and the installer never contradict each other.
- **"Install the `cua-driver` engine" option** (unchecked by default, requires internet) — runs the **official** cua project installer, which installs the **latest stable** release.
- **Registers an uninstaller** under *Settings → Apps → Installed apps*. It removes the GUI; `cua-driver` has its own lifecycle and is **not** removed along with it (the uninstaller says so on screen).

> ⚠️ **SmartScreen warning — read before running**
>
> This project's binaries are **not code-signed yet**. When you open the installer, Windows will show *"Windows protected your PC"*: click **More info → Run anyway**.
>
> **The installer does not bypass that warning** — an unsigned installer gets exactly the same block as a standalone `.exe`. Before running it, verify the `.sha256` file published next to the binary in the release:
> ```powershell
> Get-FileHash .\fzcomputerai-setup-windows-x64.exe -Algorithm SHA256
> ```
> Full context, certificate options and costs (in Portuguese): **[SIGNING.md](SIGNING.md)**.

### 🪟 Windows — Alternatives

**a) Portable binary** — download `fzcomputerai-windows-x64.exe` from the release and run it directly: no installation, no shortcuts, no autostart and no uninstaller; updates are manual. The same SmartScreen warning applies.

**b) Local installer build (for source builders)** — the old root `install.ps1` has been removed; the graphical installer is the only Windows installation path. If you build from source, generate the same installer locally:
```powershell
cargo build --release --manifest-path fzcomputerai/Cargo.toml
ISCC.exe /DAppVersion=<version> installer\fzcomputerai.iss
```
> Requires [Inno Setup](https://jrsoftware.org/isinfo.php) installed (`ISCC.exe` on PATH or full path). The resulting `fzcomputerai-setup-windows-x64.exe` lands in `dist/`.

### 🐧 Linux & 🍎 macOS — Remote Installation via Bash (One-liner)
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash
```

To simulate the installation without changing anything on your system (`--dry-run`):
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash -s -- --dry-run
```

> **Note:** the remote installer using the official release binary installs the **`fzcomputerai` GUI**. The stdio MCP server is still `cua-driver` — the script prints the corresponding `.mcp.json` snippet and points you to `npx fzcomputerai mcp` (the source-build fallback also compiles `cua-driver`).

### 📦 Via NPM (Global)
```bash
npm install -g fzcomputerai
```

### 🧱 Compilation from Source Code / Tarball (.tgz)
```bash
# Download or extract source code tarball (.tgz):
tar -xzf fzcomputerai-<version>.tgz
cd package (or fzcomputerai)

# Build engine and Rust GUI:
cargo build --release --manifest-path fzcomputerai/Cargo.toml
```

For detailed source code compilation instructions and advanced configuration, refer to [INSTALL_EN.md](INSTALL_EN.md).

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

## 📚 Documentation

Detailed documentation lives in [`docs/`](docs/README.md) (written in Portuguese):

| Document | What for |
| :--- | :--- |
| [Architecture](docs/arquitetura.md) | How the GUI and the engine split responsibilities, MCP transport, where state lives, honest-status principle |
| [MCP & Network tab](docs/uso-mcp-rede.md) | Engine lifecycle, port, LAN forwarding and how to read the diagnostics |
| [Tunnel tab](docs/uso-tunel.md) | Cloudflare, ngrok and reverse SSH step by step, URL password and exposure probe |
| [Remote access](docs/acesso-remoto.md) | LAN vs tunnel vs VPN, and **why there is no `0.0.0.0` bind** |
| [Updating](docs/atualizacao.md) | Update Center: interface and engine, and what is verified for each |
| [Troubleshooting](docs/solucao-de-problemas.md) | Symptom → cause → check → fix |
| [Development](docs/desenvolvimento.md) | Building, mandatory code conventions and installer build |
| [FAQ](docs/faq.md) | Direct questions, honest answers |

---

## 📜 License & Credits

- **Engine / Base project:** the `cua-driver` engine is part of the open-source [**Cua** (`trycua/cua`)](https://github.com/trycua/cua) project, developed and maintained by **Cua AI, Inc.** (the [cua.ai](https://cua.ai) team) under the **MIT License** — `Copyright (c) 2025 Cua AI, Inc.` FzComputerAI is an **independent graphical interface** built on top of that engine; it neither modifies nor redistributes it. **Our sincere thanks to Cua AI, Inc. and the Cua community** — this project would not exist without their work. Community: [Discord](https://discord.gg/mVnXXpdE85) · Docs: [cua.ai/docs](https://cua.ai/docs)
- **Author & FzComputerAI integrations:** Roger Luft (VeilWalker) — Webstorage Tecnologia (`roger@webstorage.com.br`)
- **License:** [MIT](LICENSE.md) — the same as the Cua project, for maximum compatibility. Full text, third-party components and the formal Cua citation are in [`LICENSE.md`](LICENSE.md).
- **Support the project:** [GitHub Sponsors](https://github.com/sponsors/RLuf)
- **Sponsors:** [Webstorage Tecnologia](https://www.webstorage.com.br) | [Imóvel Site](https://www.imovelsite.com.br)
