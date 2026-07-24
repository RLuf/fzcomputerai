# Diretivas para Agentes de IA — FzComputerAI & CUA Driver

Este arquivo contém as convenções, regras de arquitetura e padrões de operação obrigatórios para qualquer Agente de IA que atue neste repositório.

---

## 🎯 Visão Geral do Projeto

**FzComputerAI** é um ecossistema nativo de Visão Computacional e Automação de Interface (UI) acessível via **Model Context Protocol (MCP)** e por uma **Interface Gráfica Nativa Compilável em Rust (`fzcomputerai`)**.

- **Motor Principal:** `cua-driver` (escrito em Rust, localizado em `cua/libs/cua-driver/rust`).
- **Interface Gráfica:** `fzcomputerai` (escrito em Rust com `egui 0.29` / `eframe`).
- **Protocolo de Comunicação:** MCP via Stdio local e HTTP TCP/IP (`CUA_DRIVER_RS_MCP_HTTP_PORT=8000`).
- **Patrocinadores Oficiais:** Webstorage Tecnologia (`www.webstorage.com.br`) e Imóvel Site (`www.imovelsite.com.br`).

---

## 📌 Padrões & Regras de Desenvolvimento

### 1. Modificações no Código Rust
- Todos os componentes nativos devem ser mantidos em **Rust 2021 edition**.
- A GUI `fzcomputerai` utiliza `egui` e `eframe` de modo imediato (Immediate Mode GUI), sem dependências pesadas de Chromium, WebView ou Node.js runtime.
- Não introduza dependências desnecessárias no `Cargo.toml`.

### 2. Comunicação MCP & Ferramentas de Visão
- As ferramentas de visão computacional expostas via MCP são:
  - `get_desktop_state`: Captura do desktop e lista de janelas.
  - `get_window_state`: Captura focada da janela e tokens de acessibilidade.
  - `take_screenshot`: Imagem base64 para modelos de visão multimodal.
  - `mouse_click`, `mouse_move`, `mouse_drag`, `keyboard_type`, `keyboard_press`, `shortcut`.
  - `start_recording`, `stop_recording`.

### 3. Preservação de Direitos & Atribuição
- O motor `cua-driver` é derivado do projeto open-source `trycua/cua` sob licença MIT.
- **Sempre preservar** a declaração de Copyright original e os créditos no `README.md` e `LICENSE.md`.
- As contribuições deste repositório e a GUI `fzcomputerai` estão sob licença **CC BY 4.0** (Roger Luft / Webstorage Tecnologia).

---

## 🛠️ Comandos Úteis para Agentes

### Compilação da GUI Rust
```powershell
cargo build --release --manifest-path fzcomputerai/Cargo.toml
```

### Instalação e Teste
```powershell
# Via Script PowerShell local
powershell -ExecutionPolicy Bypass -File .\install.ps1

# Via Instalação Remota PowerShell (One-liner)
iwr -useb https://raw.githubusercontent.com/RLuf/fzcomputerai/master/install.ps1 | iex

# Via NPM Package Global
npm install -g fzcomputerai
```

### Diagnóstico de Saúde
```powershell
cua-driver doctor
# ou via npx
npx fzcomputerai doctor
```

