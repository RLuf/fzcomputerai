# Installation & Configuration Guide — Computer Vision via MCP

This guide contains step-by-step instructions to install, compile, and configure the **Computer Vision & UI Automation Server via MCP (Model Context Protocol)** on Windows, Linux, and macOS.

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

## ⚡ 2. Automatic Installation (Default Mode)

### On Windows (PowerShell)
Run the installation script in PowerShell:

```powershell
# Run from repository root
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The script automatically handles:
1. Checking for Rust/Cargo compiler or existing prebuilt binary.
2. Compiling the `cua-driver` engine in `--release` mode (if Rust is installed).
3. Adding the binary directory to the User `PATH`.
4. Generating the `.mcp.json` configuration file in the project folder.
5. Performing system health check (`cua-driver doctor`).

### On Linux / macOS (Bash)
Run the Shell script:

```bash
chmod +x ./install.sh
./install.sh
```

---

## 🔧 3. Advanced Installation & Manual Build (Rust Cargo)

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

## 💻 4. MCP Client Setup

After completing installation, connect the computer vision server to your favorite AI tools:

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
Execute the command below to register the MCP server with Claude Code:

```bash
claude mcp add --transport stdio fz-computer-vision -- cua-driver mcp
```

To list registered MCP servers:
```bash
claude mcp list
```

### C. Cursor / Windsurf / VS Code (MCP Extension)
1. Open Cursor or VS Code settings (`Ctrl+,` or `Cmd+,`).
2. Go to **MCP Servers** settings.
3. Add the following configuration structure:

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

## 🔍 5. Troubleshooting & Diagnostics

### Testing Server Communication
To start the MCP server manually in interactive stdio mode:

```bash
cua-driver mcp
```

### Health Diagnostics (`doctor`)
Run the built-in diagnostic tool to identify permission issues or missing system packages:

```bash
cua-driver doctor
```

### Common Issues:

1. **`cua-driver` is not recognized as an internal or external command:**
   - Restart your terminal or PowerShell window after running `install.ps1` to reload environment variables.
2. **Screen capture failure on Windows:**
   - Ensure the session is not locked at the Logon screen or running via headless SSH/WinRM without a graphical display session.
3. **Screen Recording permission error on macOS:**
   - Go to *System Settings > Privacy & Security > Screen & System Audio Recording* and grant permission to your terminal app or `cua-driver`.

---

## 📧 Support & Contact

For questions or technical support during deployment:
- **Author:** Roger Luft
- **Company:** Webstorage Tecnologia
- **Email:** `roger@webstorage.com.br`
- **WhatsApp:** +55 51 99242539
