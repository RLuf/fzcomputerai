# Changelog

All notable changes to the **FzComputerAI / CUA Driver Computer Vision MCP** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] - 2026-07-23

### Added
- **MCP Computer Vision Integration**: Native support for Model Context Protocol (MCP) enabling AI agents (Claude Code, Antigravity, Cursor, Windsurf) to perform real-time desktop visual analysis and UI control.
- **Multimodal Visual Inspection Tools**:
  - `get_desktop_state`: Full screen capture, window listings, mouse cursor position tracking.
  - `get_window_state`: Window-level visual capture and UI Automation tree element extraction.
  - `take_screenshot`: Fast PNG/JPEG base64 image capture for multimodal LLM processing.
- **Pointer & Keyboard Control Tools**:
  - `mouse_click`, `mouse_move`, `mouse_drag`, `mouse_down`, `mouse_up`.
  - `keyboard_type`, `keyboard_press`, `shortcut`.
  - `launch_app`, `close_app`, `recording_start`, `recording_stop`.
- **Automated Installation Scripts**:
  - `install.ps1`: Native PowerShell installer for Windows with automatic dependency checking, Cargo release build, PATH configuration, and `.mcp.json` generation.
  - `install.sh`: Native Bash installer for Linux (X11/Wayland) and macOS.
- **Multilingual Documentation**:
  - Portuguese (PT-BR): `README.md` and `INSTALL.md`.
  - English (EN): `README_EN.md` and `INSTALL_EN.md`.
- **Project Governance & Config**:
  - Configured workspace `.gitignore` for Rust build artifacts, temporary files, and IDE workspace directories.
  - Added default `.mcp.json` configuration file for automatic client discovery.

---

## [0.8.3] - 2026-07-15

### Added
- Initial workspace integration with `cua-driver` core engine (Rust workspace).
- Windows Graphics Capture (WGC) and Win32 UI Automation backend crates.
