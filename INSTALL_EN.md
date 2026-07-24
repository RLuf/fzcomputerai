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

### B. Remote Installation via PowerShell (Windows One-liner)
```powershell
iwr -useb https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.ps1 | iex
```

### C. Remote Installation via Bash (Linux & macOS One-liner)
```bash
curl -fsSL https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.sh | bash
```

### D. Local Installation from Source Code

#### On Windows (PowerShell)
```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

#### On Linux / macOS (Bash)
```bash
chmod +x ./install.sh
./install.sh
```

The installation script automatically handles:
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

# Restart cua-driver daemon
cua-driver stop
cua-driver autostart kick
```

### Testing the HTTP TCP port:
```powershell
netstat -an | findstr 8000
# Expected output: TCP 127.0.0.1:8000 LISTENING
```

### On the Remote Client (FazAI-NG / Orchestrator):
Send POST JSON-RPC requests to:
- **URL**: `http://<WINDOWS_IP>:8000/mcp`
- **Body**: `{"jsonrpc":"2.0","id":1,"method":"tools/list"}`

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
