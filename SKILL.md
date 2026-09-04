---
name: fzcomputerai-computer-vision
description: >-
  Use when you need to visually interact with a Windows desktop — see the screen,
  move the mouse, click, type, drag, manage windows, take screenshots, automate
  user flows, or perform end-to-end QA. FzComputerAI wraps the CUA Driver
  (Computer Use Agent) with an MCP server accessible via HTTP JSON-RPC on any
  interface (0.0.0.0:8000 by default), enabling remote agents on Linux, macOS,
  or other machines to control a Windows desktop over the network.
  Install: npm install -g fzcomputerai or the graphical Windows installer
  (fzcomputerai-setup-windows-x64.exe) from GitHub Releases.
---

# FzComputerAI — Computer Vision & Desktop Automation via MCP

FzComputerAI gives AI agents **eyes and hands on a real Windows desktop**: see
the screen, move the mouse, click, type, drag, scroll, and manage windows —
like a human at the keyboard. Accessible from **any machine on the network**
via MCP over HTTP.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Remote Agent (Linux, macOS, Windows, Cloud)             │
│  POST http://<windows-ip>:8000/mcp                       │
│  {"jsonrpc":"2.0","method":"tools/call",...}              │
└──────────────────────────┬───────────────────────────────┘
                           │ HTTP JSON-RPC
                           ▼
┌──────────────────────────────────────────────────────────┐
│  FzComputerAI (Windows)                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ GUI (egui)   │  │ MCP HTTP     │  │ MCP Stdio     │  │
│  │ Port config  │  │ 0.0.0.0:8000 │  │ cua-driver mcp│  │
│  │ Daemon ctrl  │  │ JSON-RPC     │  │ pipe           │  │
│  └──────────────┘  └──────┬───────┘  └───────┬───────┘  │
│                           │                   │          │
│                    ┌──────▼───────────────────▼──────┐   │
│                    │       CUA Driver (Rust)         │   │
│                    │  Computer Vision & UI Automation │   │
│                    │  Win32 + UIA + Screenshots       │   │
│                    └─────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

## Setup

### Option A: npm (recommended)
```bash
npm install -g fzcomputerai
fzcomputerai doctor    # verify installation
```

### Option B: Graphical installer (Windows)

Download `fzcomputerai-setup-windows-x64.exe` from
<https://github.com/RLuf/fzcomputerai/releases/latest> and run it. The
installer offers the cua-driver engine install task (required to control the
machine; needs internet). Unsigned binaries trigger SmartScreen: use
"More info > Run anyway".

### Option C: Build from source (Windows)
```powershell
git clone https://github.com/RLuf/fzcomputerai.git
cd fzcomputerai
cargo build --release --manifest-path fzcomputerai/Cargo.toml
# Optional: build the graphical installer (requires Inno Setup / ISCC.exe)
ISCC.exe /DAppVersion=2.3.4 installer\fzcomputerai.iss
```

After installation, the MCP server listens on `http://0.0.0.0:8000/mcp` by
default. Use the GUI (`fzcomputerai.exe`) to start/stop the daemon, configure
the port, and test tools.

## Workflow: Look → Act → Verify

Every interaction follows a three-step cycle:

```
1. screenshot        → see what's on screen
2. click 450 280     → act on the UI
3. screenshot        → verify the result
```

> **CRITICAL**: Always re-screenshot after every UI change — coordinates go
> stale when the screen changes.

## MCP JSON-RPC Protocol

All tools are invoked via `POST http://<ip>:8000/mcp` with a JSON-RPC 2.0 body:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "<tool_name>",
    "arguments": { ... }
  }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{ "type": "text", "text": "..." }]
  }
}
```

## Complete Tool Reference

### Vision & Screenshot

| Tool | Arguments | Description |
|------|-----------|-------------|
| `screenshot` | `{}` | Capture full desktop as base64 PNG image |
| `get_screen_size` | `{}` | Get screen resolution (width, height) and DPI |
| `get_cursor_position` | `{}` | Get current cursor (x, y) position |
| `get_desktop_state` | `{}` | Full desktop state: screenshot + window list |

### Mouse & Cursor

| Tool | Arguments | Description |
|------|-----------|-------------|
| `click` | `{"x": 450, "y": 280}` | Left click at coordinates |
| `double_click` | `{"x": 450, "y": 280}` | Double click at coordinates |
| `right_click` | `{"x": 450, "y": 280}` | Right click at coordinates |
| `drag` | `{"start_x": 100, "start_y": 200, "end_x": 300, "end_y": 400}` | Drag from start to end |
| `move_cursor` | `{"x": 450, "y": 280}` | Move cursor without clicking |
| `scroll` | `{"x": 450, "y": 280, "direction": "down", "amount": 3}` | Scroll at position |

### Keyboard & Typing

| Tool | Arguments | Description |
|------|-----------|-------------|
| `type_text` | `{"text": "Hello World"}` | Type text via input simulation |
| `type_text_chars` | `{"text": "precise"}` | Type character by character (slower, more precise) |
| `press_key` | `{"key": "enter"}` | Press a single key (enter, tab, escape, f1-f12, etc.) |
| `hotkey` | `{"keys": "ctrl+c"}` | Keyboard shortcut combination |
| `set_value` | `{"pid": 1234, "element_id": "...", "value": "text"}` | Set value on UIA control |

### Windows & Applications

| Tool | Arguments | Description |
|------|-----------|-------------|
| `list_apps` | `{}` | List all installed applications |
| `list_windows` | `{}` | List all active windows with PID, title, position |
| `get_window_state` | `{"pid": 1234}` | Get detailed state of a specific window + UIA tokens |
| `launch_app` | `{"app": "notepad"}` | Launch an application |
| `kill_app` | `{"pid": 1234}` | Kill an application by PID |
| `zoom` | `{"window_title": "Chrome"}` | Zoom to window (coordinates become window-relative) |

### Accessibility & UIA

| Tool | Arguments | Description |
|------|-----------|-------------|
| `get_accessibility_tree` | `{"pid": 1234}` | Get the full UI Automation accessibility tree |
| `check_permissions` | `{}` | Check system permissions for automation |

### Agent Cursor

| Tool | Arguments | Description |
|------|-----------|-------------|
| `set_agent_cursor_enabled` | `{"enabled": true}` | Enable/disable the agent cursor overlay |
| `set_agent_cursor_motion` | `{"motion": "smooth"}` | Set cursor motion animation style |
| `set_agent_cursor_style` | `{"style": "default"}` | Set cursor visual style |
| `get_agent_cursor_state` | `{}` | Get current agent cursor state |

### Recording & Replay

| Tool | Arguments | Description |
|------|-----------|-------------|
| `start_recording` | `{}` | Start recording the session (screenshots + actions) |
| `stop_recording` | `{}` | Stop recording and save trajectory |
| `get_recording` | `{}` | Get current recording data |
| `replay_recording` | `{"path": "..."}` | Replay a saved recording |
| `install_ffmpeg` | `{}` | Install ffmpeg for video export |

### Session & Configuration

| Tool | Arguments | Description |
|------|-----------|-------------|
| `session_start` | `{"session": "my-session"}` | Start an isolated MCP session |
| `session_end` | `{"session": "my-session"}` | End an MCP session |
| `get_config` | `{}` | Read current CUA driver configuration |
| `set_config` | `{"key": "value"}` | Set configuration key-value pair |
| `health_report` | `{}` | Generate a system health report |
| `check_for_update` | `{}` | Check for CUA driver updates |

## Common Scenarios

### Open an app and interact

```json
// 1. Launch Notepad
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch_app","arguments":{"app":"notepad"}}}

// 2. Wait, then screenshot to find it
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"screenshot","arguments":{}}}

// 3. Click on the text area (coordinates from screenshot)
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"click","arguments":{"x":400,"y":300}}}

// 4. Type text
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"type_text","arguments":{"text":"Hello from remote agent!"}}}

// 5. Verify
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"screenshot","arguments":{}}}
```

### Fill a form

```json
// Click first field, type, tab to next, repeat
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"click","arguments":{"x":400,"y":200}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"type_text","arguments":{"text":"John Doe"}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"press_key","arguments":{"key":"tab"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"type_text","arguments":{"text":"john@example.com"}}}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"click","arguments":{"x":400,"y":500}}}
```

### Precision clicks with zoom

```json
// Zoom to a specific window for higher-resolution coordinates
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"zoom","arguments":{"window_title":"Google Chrome"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"screenshot","arguments":{}}}
// Now coordinates are window-relative — click small elements precisely
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"click","arguments":{"x":112,"y":44}}}
```

### Keyboard shortcuts

```json
// Save a file: Ctrl+S
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"hotkey","arguments":{"keys":"ctrl+s"}}}

// Select all: Ctrl+A
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"hotkey","arguments":{"keys":"ctrl+a"}}}

// Close window: Alt+F4
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"hotkey","arguments":{"keys":"alt+f4"}}}
```

## Network Configuration

The MCP server binds to `0.0.0.0` by default (all interfaces), making it
accessible from any machine on the LAN. For remote access beyond the LAN:

- **Port forwarding**: Forward port 8000 on your border firewall/router
- **VPN**: Connect remote agents via VPN, then use the Windows LAN IP
- **PortProxy (netsh)**: The GUI can configure netsh portproxy rules to redirect
  from any IP to localhost if needed
- **Tunnel (public internet)**: The GUI's **Tunnel** tab exposes the local MCP over
  a public HTTPS URL via Cloudflare Tunnel, ngrok, or reverse SSH — no inbound
  firewall/portproxy needed (outbound tunnel). Note: the engine's own auth is a
  single bearer token (measured on 0.17.0, 2026-08-03 — see the token contract
  below), so every tunnelled call must carry `Authorization: Bearer <token>`;
  add a layer in front with the tab's level-1 URL password (`/s/<password>/mcp`)
  or the provider's own auth (Cloudflare Access, ngrok policy, SSH key). The
  tunnel is torn down when the app closes.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CUA_DRIVER_RS_MCP_HTTP_PORT` | *(unset — HTTP off)* | TCP port for the HTTP MCP server. The listener is only created when this is set. |
| `CUA_DRIVER_RS_MCP_HTTP_TOKEN` | *(unset)* | Bearer token, **user-generated** (any random string). Without it in the process environment the daemon does not even start; with it, every request still needs `Authorization: Bearer <token>`. See the measured contract below. |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

#### Token contract — measured against the 0.17.0 binary on 2026-08-03

Not quoted from docs; run against the installed `cua-driver` 0.17.0:

- **No `CUA_DRIVER_RS_MCP_HTTP_TOKEN` in the process environment ⇒ the daemon
  refuses to start.** `cua-driver serve` exits 1 with stderr
  `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`.
  This is a startup failure, not a rejected request — the symptom is a silent port.
- With the daemon up, **any** request without an `Authorization` header gets
  **HTTP 401** and body
  `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` —
  identical on `POST /mcp`, `GET /mcp` and `GET /`, and with **no**
  `WWW-Authenticate` header. The TCP connection itself is accepted normally
  (`Test-NetConnection` on the port returns `True`): the refusal happens at the
  application layer, not the socket. With the correct bearer token: **HTTP 200**
  and the `initialize` result (`capabilities.tools`, cua-driver instructions).
- The token is generated by **you** — the engine itself calls it a
  *host-generated bearer token*. No `cua-driver` command and no official
  `install.ps1` generates one. Persisting it in `HKCU\Environment` is what makes
  the autostart task see it at the **next logon**: the `cua-driver-serve`
  Scheduled Task inherits the **logon** environment, so a token written after
  you logged in yields a tokenless daemon that dies immediately and leaves the
  port silent. Only one daemon may exist at a time (`cua-driver stop` first).

> **The listen address is NOT configurable.** The official engine binds **only to
> `127.0.0.1`** — the address is hardcoded in Cua's `mcp_http.rs`
> (`([127,0,0,1], port)`), and no bind variable exists upstream. A previous
> version of this document listed a `CUA_DRIVER_RS_MCP_HTTP_BIND` variable
> defaulting to `0.0.0.0`; **that was wrong** and has been removed (verified
> twice: the string does not exist in the installed official binary, and
> searching the `trycua/cua` repository for it returns zero hits).
> For LAN access use `netsh portproxy` (MCP & Network tab); for internet access
> use an outbound tunnel (Tunnel tab).

### CLI Access

```bash
cua-driver mcp                     # Start MCP server (stdio)
cua-driver autostart kick          # Start daemon (HTTP)
cua-driver stop                    # Stop daemon
cua-driver doctor                  # System health check
cua-driver call screenshot         # Direct CLI tool call
cua-driver call click --x 450 --y 280
```

## Sponsors

- [Webstorage Tecnologia](https://www.webstorage.com.br)
- [Imóvel Site](https://www.imovelsite.com.br)

Developed by Roger Luft <roger@webstorage.com.br>
License: CC BY 4.0
