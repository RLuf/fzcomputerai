---
name: fzcomputerai-computer-vision
description: >-
  Use when you need to visually interact with a Windows desktop — see the screen,
  move the mouse, click, type, drag, manage windows, take screenshots, automate
  user flows, or perform end-to-end QA. FzComputerAI wraps the CUA Driver
  (Computer Use Agent) with an MCP server over HTTP JSON-RPC on 127.0.0.1:8000
  (loopback only; the GUI publishes it on the LAN through an in-process TCP relay
  or on the internet via the Tunnel tab), enabling remote agents on Linux, macOS,
  or other machines to control a Windows desktop.
  Install: npm install -g fzcomputerai or the graphical Windows installer
  (fzcomputerai-setup-windows-x64.exe) from GitHub Releases.
---

# FzComputerAI — Computer Vision & Desktop Automation via MCP

FzComputerAI gives AI agents **eyes and hands on a real Windows desktop**: see
the screen, move the mouse, click, type, drag, scroll, and manage windows —
like a human at the keyboard. The engine listens on **loopback only**; the GUI
publishes it to the LAN (in-process TCP relay) or to the internet (Tunnel tab),
so remote agents can reach it over MCP HTTP. The GUI starts the engine as a
**child process** and takes it down with itself.

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
│  │ GUI (egui)   │  │ LAN relay    │  │ Tunnel        │  │
│  │ Port config  │  │0.0.0.0:<port>│  │ cloudflared / │  │
│  │ Engine child │  │ -> 127.0.0.1 │  │ ngrok / ssh   │  │
│  └──────────────┘  └──────┬───────┘  └───────┬───────┘  │
│                           │                   │          │
│                    ┌──────▼───────────────────▼──────┐   │
│                    │  cua-driver serve (child proc)  │   │
│                    │  MCP HTTP 127.0.0.1 + pipe      │   │
│                    │  Win32 + UIA + Screenshots      │   │
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
ISCC.exe /DAppVersion=2.4.2 installer\fzcomputerai.iss
```

After installation, the HTTP MCP listener only starts when
`CUA_DRIVER_RS_MCP_HTTP_PORT` is set (the GUI's *MCP & Network* tab does this
for you) and binds to `http://127.0.0.1:<port>/mcp` — loopback only. Use the
GUI (`fzcomputerai.exe`) to configure the port, publish on the LAN (relay) or
internet (Tunnel tab), and test tools.

On startup the GUI launches the engine itself, as a **child process**, when
nothing is already answering on the port — there is no scheduled task involved.
Every child (engine and tunnel) is adopted into a Windows **Job Object**
(`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), so the kernel takes them down when the
GUI ends in any way: window close, tray *Exit*, `taskkill /F`, crash, logoff.
An engine started by **another** MCP client and found on the port is detected,
never duplicated and never killed on exit — the GUI says so, and stopping a
foreign engine stays an explicit **Stop** button action.

## Workflow: Look → Act → Verify

Every interaction follows a three-step cycle:

```
1. get_window_state  → see the window (screenshot + UIA element tokens)
2. click             → act on the UI (element token or screenshot pixel)
3. verify_state      → verify the postcondition (or re-snapshot)
```

> **CRITICAL**: Always re-snapshot (`get_window_state` / `get_desktop_state`)
> after every UI change — coordinates and element tokens go stale when the
> screen changes.

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
| `get_desktop_state` | `{}` | Full desktop state: screenshot + window list + cursor |
| `get_window_state` | `{"pid": 1234, "include_screenshot": true}` | Focused window capture + UIA element tokens |
| `get_screen_size` | `{}` | Get screen resolution (width, height) and DPI |
| `get_cursor_position` | `{}` | Get current cursor (x, y) position |

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
| `list_apps` | `{}` | List applications |
| `list_windows` | `{}` | List all active windows with PID, title, position |
| `bring_to_front` | `{"pid": 1234, "window_id": 5678}` | Raise a window (needed before foreground clicks) |
| `set_window_frame` | `{"pid": 1234, "window_id": 5678, "x": 0, "y": 0, "width": 800, "height": 600}` | Set exact window geometry with readback |
| `launch_app` | `{"app": "notepad"}` | Launch an application |
| `kill_app` | `{"pid": 1234}` | Kill an application by PID |
| `zoom` | `{"pid": 1234, ...}` | Magnify a window region; zoomed coordinates can be clicked via `from_zoom` |

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
| `set_agent_cursor_theme` | `{"theme": "default"}` | Set cursor visual theme |
| `get_agent_cursor_state` | `{}` | Get current agent cursor state |

### Recording & Replay

| Tool | Arguments | Description |
|------|-----------|-------------|
| `start_recording` | `{}` | Start recording the session (screenshots + actions) |
| `stop_recording` | `{}` | Stop recording and save trajectory |
| `get_recording_state` | `{}` | Get current recording state |
| `replay_trajectory` | `{"path": "..."}` | Replay a saved trajectory |
| `install_ffmpeg` | `{}` | Install ffmpeg for video export |

### Session & Configuration

| Tool | Arguments | Description |
|------|-----------|-------------|
| `start_session` | `{"session": "my-session"}` | Start an isolated MCP session |
| `end_session` | `{"session": "my-session"}` | End an MCP session |
| `get_config` | `{}` | Read current CUA driver configuration |
| `set_config` | `{"key": "value"}` | Set configuration key-value pair |
| `health_report` | `{}` | Generate a system health report |
| `check_for_update` | `{}` | Check for CUA driver updates |

## Common Scenarios

### Open an app and interact

```json
// 1. Launch Notepad
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch_app","arguments":{"app":"notepad"}}}

// 2. Wait, then snapshot the desktop to find it (returns windows + pid)
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_desktop_state","arguments":{}}}

// 3. Click on the text area (pixel from the window screenshot, or element token)
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"click","arguments":{"pid":1234,"x":400,"y":300}}}

// 4. Type text
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"type_text","arguments":{"pid":1234,"text":"Hello from remote agent!"}}}

// 5. Verify
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_window_state","arguments":{"pid":1234,"include_screenshot":true}}}
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
// Zoom into a window region for higher-resolution coordinates
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"zoom","arguments":{"pid":1234}}}
// Click using the zoomed image's pixel coordinates
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"click","arguments":{"pid":1234,"x":112,"y":44,"from_zoom":true}}}
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

The MCP server binds **only to `127.0.0.1`** (loopback) — by itself it is not
reachable from other machines. To publish it:

- **LAN — in-process TCP relay**: the *MCP & Network* tab's **Publish on the
  network** button starts a relay inside the GUI process: it listens on
  `0.0.0.0:<port>` (or on a chosen IP) and forwards to the engine's
  `127.0.0.1:<port>`, copying bytes both ways without inspecting HTTP
  (keep-alive and SSE pass through intact). It needs **no UAC**, leaves **no rule
  on the system** and dies with the app. The tab shows a PUBLISHED ON NETWORK /
  LOCAL ONLY badge and a real connection counter (active/total). Measured on this
  platform: `0.0.0.0:8000` coexists with the engine's `127.0.0.1:8000`, so the
  same port is published without touching the engine. Verified over the LAN:
  `initialize` OK, `tools/list` with 55 tools, and `tools/call get_screen_size`
  really executing (4096x2160 @ 1.75x) through `http://<LAN-IP>:8000/mcp`.
  Removing a **legacy** `netsh portproxy` rule is still offered, and only appears
  when such a rule exists.
- **VPN**: connect remote agents via VPN, then use the Windows LAN IP (the relay
  above still has to be publishing).
- **Tunnel (public internet)**: The GUI's **Tunnel** tab exposes the local MCP over
  a public HTTPS URL via Cloudflare Tunnel, ngrok, or reverse SSH — no inbound
  firewall rule needed (outbound tunnel). Two Cloudflare flavors: the **quick
  tunnel** (no account, random `*.trycloudflare.com` URL — verified working over
  the internet) and a **named tunnel on your own domain**, for a fixed URL, driven
  end to end from the tab: **Login** (browser OAuth) → **Check login** (really
  looks for `~/.cloudflared/cert.pem`) → *Tunnel name* + *Public hostname* →
  **Create tunnel + point DNS** (runs `cloudflared tunnel create` and
  `tunnel route dns` in the background) → **Start tunnel**, which then runs
  `cloudflared tunnel run --url http://127.0.0.1:<port> <name>`. The login alone
  creates nothing — it only downloads `cert.pem`, hence the two extra steps — and
  the domain has to be in your Cloudflare account (nameservers delegated). For
  ngrok, the authtoken lives in `%LOCALAPPDATA%\ngrok\ngrok.yml` (not in the
  Windows registry); `ngrok config check` only validates the file **syntax**, not
  the token, so the tab watches the log for `ERR_NGROK_105` and shows the fix
  (`ngrok config add-authtoken <TOKEN>`).
  Authentication: engines **0.16+** are fail-closed and require a Bearer token
  (`CUA_DRIVER_RS_MCP_HTTP_TOKEN`); the Tunnel tab can generate and activate it,
  and the copied `mcpServers` snippet then includes
  `"Authorization": "Bearer <token>"`. For clients that accept **a URL only**,
  with nowhere to put a header (Claude Desktop is one), use the tab's URL password
  (`/s/<password>/mcp`): whoever proved the password in the path is authenticated
  as far as the app is concerned, so the gate **adds the `Authorization` header
  itself when talking to the engine** — if the client sends its own
  `Authorization`, the client's wins. Verified: password URL with no headers at
  all → `initialize` OK and `tools/call get_screen_size` executed (4096x2160);
  wrong password → 404; no password → 404. The engine secret never travels over
  the internet; the public credential becomes the URL password. Older engines
  (<= 0.8.x) have **no authentication of their own** — for those the URL password
  or the provider's own auth (Cloudflare Access, ngrok policy, SSH key) is the
  protection. The tab's exposure test probes the public URL in 2 phases (without
  credentials, then with the Bearer when known) to verify the tunnel is actually
  protected, and runs in the background so the UI stays responsive. The tunnel is
  torn down when the app closes.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CUA_DRIVER_RS_MCP_HTTP_PORT` | *(unset — HTTP off)* | TCP port for the HTTP MCP server. The listener is only created when this is set. |
| `CUA_DRIVER_RS_MCP_HTTP_TOKEN` | *(unset)* | Bearer token. **Required by newer engine releases** (0.16+): requests without `Authorization: Bearer <token>` get `401`. On 0.16+, unset does **not** mean open — the endpoint is fail-closed (`401` for everything). The GUI's Tunnel tab can generate one (CSPRNG, >= 32 chars) into `HKCU\Environment`. Older releases (e.g. 0.8.3) ignore it. |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

> **The listen address is NOT configurable.** The official engine binds **only to
> `127.0.0.1`** — the address is hardcoded in Cua's `mcp_http.rs`
> (`([127,0,0,1], port)`), and no bind variable exists upstream. A previous
> version of this document listed a `CUA_DRIVER_RS_MCP_HTTP_BIND` variable
> defaulting to `0.0.0.0`; **that was wrong** and has been removed (verified
> twice: the string does not exist in the installed official binary, and
> searching the `trycua/cua` repository for it returns zero hits).
> For LAN access use the GUI's in-process relay (MCP & Network tab); for internet
> access use an outbound tunnel (Tunnel tab).
>
> Engines 0.16+ also answer **403** to requests carrying a browser `Origin`
> header — verified. Call them from a server or CLI, not from a browser tab.

### CLI Access

```bash
cua-driver mcp                     # MCP server over stdio (exits when stdin closes)
cua-driver serve                   # Engine + HTTP endpoint + \\.\pipe\cua-driver
                                   # (HTTP only with CUA_DRIVER_RS_MCP_HTTP_PORT set);
                                   # this is what the GUI starts as a child process
cua-driver stop                    # Stop the running engine
cua-driver doctor                  # System health check
cua-driver call get_desktop_state  # Direct CLI tool call
cua-driver call click --x 450 --y 280
```

> The GUI no longer uses `cua-driver autostart kick` (no scheduled task) and no
> longer runs `cua-driver stop` on exit — the engine it started is a child process
> killed by the Job Object, and an engine belonging to someone else is left alone.

### Remote test script

`scripts/teste_remoto_mcp.py` checks a published endpoint from outside the
network, using the **Python 3 standard library only**: `initialize`,
`tools/list`, then it opens a **new** browser window on the remote machine (never
hijacking an open one), goes to `search.yahoo.com`, types the term, finds and
clicks the search button (Search/Pesquisar/Buscar) or presses Enter, and confirms
the result by reading the screen back.

```bash
python scripts/teste_remoto_mcp.py <URL> [--token TOKEN] [--termo TEXT]
```

If the URL already carries the password in its path (`/s/<password>/mcp`), no
token is needed.

## Sponsors

- [Webstorage Tecnologia](https://www.webstorage.com.br)
- [Imóvel Site](https://www.imovelsite.com.br)

Developed by Roger Luft <roger@webstorage.com.br>
License: MIT (see `LICENSE.md`). The `cua-driver` engine is part of the
open-source [Cua](https://github.com/trycua/cua) project — MIT,
Copyright (c) 2025 Cua AI, Inc.
