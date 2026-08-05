# Installation & Configuration Guide — Computer Vision via MCP

This guide contains step-by-step instructions to install, compile, and configure the **Computer Vision & UI Automation Server via MCP (Model Context Protocol)** on Windows, Linux, and macOS.

---

## 💎 Project Sponsors

- **Webstorage Tecnologia** — [www.webstorage.com.br](https://www.webstorage.com.br)
- **Imóvel Site** — [www.imovelsite.com.br](https://www.imovelsite.com.br)

---

## 📋 1. System Requirements

### Windows
- **Operating System:** Windows 10 / 11 (64-bit) or Windows Server 2019+
- **Shell:** PowerShell 5.1 or PowerShell 7+
- **Compiler (optional for local build):** Rust and Cargo (`rustc 1.75+`)
  ```powershell
  # Installing Rust on Windows (if building from source)
  winget install Rustlang.Rustup
  ```

### Linux
- **Supported Distributions:** Ubuntu 20.04+, Debian 11+, Fedora 36+, Arch Linux
- **System Dependencies (X11 / Wayland):**
  ```bash
  # Debian/Ubuntu
  sudo apt-get update && sudo apt-get install -y build-essential libx11-dev libxtst-dev libxcb1-dev
  ```

### macOS
- **Operating System:** macOS 12 Monterey or newer (Intel / Apple Silicon M1/M2/M3)
- **Required Permissions:** **Screen Recording** and **Accessibility** permissions under *System Settings > Privacy & Security*.

---

## ⚡ 2. Installation Methods

### A. Via NPM (Node.js Package Manager)
```bash
npm install -g fzcomputerai
```

### B. Windows — Graphical Installer (the only Windows installation path)

> The old root `install.ps1` has been **removed** — Windows installation is now done exclusively through the graphical installer (Inno Setup).

1. Download **`fzcomputerai-setup-windows-x64.exe`** from [https://github.com/RLuf/fzcomputerai/releases/latest](https://github.com/RLuf/fzcomputerai/releases/latest).
2. Run the file. Since the binaries are not code-signed yet, SmartScreen will show *"Windows protected your PC"* — click **More info → Run anyway**.
3. During installation, check the **"Install the `cua-driver` engine"** task (required for the MCP server; needs internet) — it runs the official cua project installer.

### C. Remote Installation via Bash (Linux & macOS One-liner)
```bash
curl -fsSL https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.sh | bash
```

### D. Local Installation from Source Code

#### On Windows (PowerShell) — build the graphical installer locally
Source builders generate the **same graphical installer** shipped in releases (requires [Inno Setup](https://jrsoftware.org/isinfo.php) with `ISCC.exe` accessible):
```powershell
# 1. Build the GUI
cargo build --release --manifest-path fzcomputerai/Cargo.toml

# 2. Build the installer (output: dist\fzcomputerai-setup-windows-x64.exe)
ISCC.exe /DAppVersion=<version> installer\fzcomputerai.iss
```

#### On Linux / macOS (Bash)
```bash
chmod +x ./install.sh
./install.sh
```

The `install.sh` script automatically handles:
1. Checking for Rust/Cargo compiler and building `cua-driver` engine and `fzcomputerai` GUI.
2. Configuring environment variables and adding binary directory to `PATH`.
3. Enabling `CUA_DRIVER_RS_MCP_HTTP_PORT=8000` for native HTTP TCP/IP remote transport.
4. Generating `.mcp.json` configuration file.
5. Performing system health check (`cua-driver doctor`).

---

## 🌐 3. Configuring HTTP TCP/IP Transport (Remote Orchestrators / FazAI-NG)

To allow agents running on remote servers (such as **FazAI-NG**) to send JSON-RPC calls over the TCP/IP network:

### On Windows (Target Machine to be Controlled):
```powershell
# Set environment variable in User environment
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '8000', 'User')
```

> **No scheduled task needed.** On startup, `fzcomputerai.exe` starts the engine itself, as a **child process**
> (only when nothing is already answering on the port), and the engine goes down with the app — including when the
> GUI is closed via the X button, the tray menu, or `taskkill /F`. If an engine from **another** MCP client is
> already answering on the port, the app detects it, does **not** duplicate it and does **not** stop it on exit
> (the UI says so). The old `cua-driver autostart kick` was removed from the application flow.

### Testing the HTTP TCP port:
```powershell
netstat -an | findstr 8000
# Expected output: TCP 127.0.0.1:8000 LISTENING
```

### On the Remote Client (FazAI-NG / Orchestrator):

> The engine listens **only on `127.0.0.1`** (that's what the `netstat` above shows). For
> `http://<WINDOWS_IP>:8000/mcp` to work from another machine, click **Publish on the network** on the GUI's
> *MCP & Network* tab — or use the **Tunnel** tab for internet access.
>
> Since v2.3.0, LAN publishing is done by a **TCP relay running inside the GUI process**: it listens on
> `0.0.0.0:<port>` (or on the IP chosen in the *Listen on* field) and forwards to the engine's `127.0.0.1:<port>`,
> copying bytes both ways without inspecting HTTP (keep-alive and SSE pass through intact). Measured differences
> against the old `netsh portproxy` rule: it **does not prompt for UAC**, **leaves no rule behind on the system**
> (the `netsh` one survives a reboot) and **goes down when the app closes**. Removing a **legacy** `portproxy` rule
> is still available on the same tab, and only shows up when one exists.

Send POST JSON-RPC requests to:
- **URL**: `http://<WINDOWS_IP>:8000/mcp`
- **Body**: `{"jsonrpc":"2.0","id":1,"method":"tools/list"}`

> **`cua-driver` engines 0.16+**: the HTTP endpoint requires the `CUA_DRIVER_RS_MCP_HTTP_TOKEN` token (32–4096 characters) and responds **401** to any call without the `Authorization: Bearer <token>` header — including when no token is configured at all (fail-closed: 401 for everything). Generate and activate the token from the GUI's **Tunnel** tab (**Generate and activate engine token** button) and include the header in your calls. Older releases (<= 0.8.x) have no authentication.
>
> These engines also **reject requests carrying a browser `Origin` header (HTTP 403)** — verified. Call them from a server/CLI, not from a browser tab.
>
> For clients that accept **a URL only**, with nowhere to paste a header (Claude Desktop is one), use the tunnel with the **password in the path** (`/s/<password>/mcp`, **Tunnel** tab): whoever proved the password is already authenticated as far as the app is concerned, so the gate **adds the `Authorization` header when talking to the engine**. If the client sends its own `Authorization`, the client's wins. The engine secret never travels over the internet; the public credential becomes the URL password.

---

## 🔧 4. Advanced Installation & Manual Build (Rust Cargo)

If you prefer to compile the computer vision server directly from native Rust source code:

### Step 1: Navigate to the Rust workspace
```bash
cd cua/libs/cua-driver/rust
```

### Step 2: Build in Release mode
```bash
cargo build --release --package cua-driver
```

The compiled executable will be generated at:
- **Windows:** `cua/libs/cua-driver/rust/target/release/cua-driver.exe`
- **Linux/macOS:** `cua/libs/cua-driver/rust/target/release/cua-driver`

### Step 3: Verify the binary
```bash
./target/release/cua-driver doctor
```

---

## 💻 5. Local MCP Client Setup

### A. Antigravity / Gemini CLI
Create or edit `.mcp.json` in your project root directory:

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

### C. Cursor / Windsurf / VS Code (MCP Extension)
In your IDE's `mcp.json` file, add:

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

## 🔍 6. Troubleshooting & Diagnostics

### Testing Server Communication
```bash
cua-driver mcp
```

> `cua-driver mcp` is **stdio** and exits when `stdin` closes (measured on 0.17) — so it cannot keep the HTTP
> endpoint up. The mode used by the GUI is `cua-driver serve`, which also opens the `\\.\pipe\cua-driver` pipe
> (the channel the CLI itself uses for `call`/`status`/`stop`). HTTP only comes up with
> `CUA_DRIVER_RS_MCP_HTTP_PORT` set in the environment.

### Testing from outside the network (`scripts/teste_remoto_mcp.py`)

End-to-end check written with the **Python 3 standard library only** (nothing to install). It runs `initialize`,
`tools/list`, opens a **new** browser window on the remote machine (it never hijacks an already open window),
navigates to `search.yahoo.com`, types the term, finds and clicks the search button (Search/Pesquisar/Buscar) or
sends Enter, and confirms the result by reading the screen back.

```bash
python scripts/teste_remoto_mcp.py <URL> [--token TOKEN] [--termo TEXT]
```

If the URL already carries the password in its path (`/s/<password>/mcp`), `--token` is not needed.

### Health Diagnostics (`doctor`)
```bash
cua-driver doctor
```

---

## 📧 Support & Contact

- **Author:** Roger Luft
- **Company:** Webstorage Tecnologia (`www.webstorage.com.br`)
- **Partner:** Imóvel Site (`www.imovelsite.com.br`)
- **Email:** `roger@webstorage.com.br`
- **WhatsApp:** +55 51 99242539
