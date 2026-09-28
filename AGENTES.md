# Diretivas para Agentes de IA — FzComputerAI & CUA Driver

> **Nota:** Este documento é o espelho em português de [`AGENTS.md`](AGENTS.md). Ambos são normativos e devem ser mantidos em sincronia.

Este arquivo contém as convenções, regras de arquitetura e padrões de operação obrigatórios para qualquer Agente de IA que atue neste repositório.

---

## 🎯 Visão Geral do Projeto

**FzComputerAI** é um ecossistema nativo de Visão Computacional e Automação de Interface (UI) acessível via **Model Context Protocol (MCP)** e por uma **Interface Gráfica Nativa Compilável em Rust (`fzcomputerai`)**.

- **Motor Principal:** `cua-driver` (escrito em Rust, localizado em `cua/libs/cua-driver/rust`).
- **Interface Gráfica:** `fzcomputerai` (escrito em Rust com `egui 0.29.1` / `eframe 0.29.1`).
- **Protocolo de Comunicação:** MCP via Stdio local e HTTP TCP/IP (`CUA_DRIVER_RS_MCP_HTTP_PORT=8000`), com *bearer token* obrigatório em `CUA_DRIVER_RS_MCP_HTTP_TOKEN` — gerado e persistido pela própria GUI.
- **HTTPS & TLS (v2.2.0+):** terminação TLS **dentro da GUI** (`src/tls.rs`), listener em `<bind>:8443` encaminhando para `127.0.0.1:8000`. Certificado auto-assinado gerado no setup (`--tls-init`) ou no primeiro run; Let's Encrypt (DNS-01 Cloudflare / HTTP-01 porta 80); cert próprio customizado.
- **OAuth 2.1 & Conectores MCP (v2.3.0 - v2.4.1):** Servidor OAuth 2.1 integrado na GUI e micro-proxy Python auxiliar (`mcp-oauth-proxy`) para conectores de IA e agentes remotos.
- **Patrocinadores Oficiais:** Webstorage Tecnologia (`www.webstorage.com.br`) e Imóvel Site (`www.imovelsite.com.br`).

---

## 📌 Padrões & Regras de Desenvolvimento

### 1. Modificações no Código Rust

- Todos os componentes nativos devem ser mantidos em **Rust 2021 edition**.
- A GUI `fzcomputerai` utiliza `egui` e `eframe` de modo imediato (Immediate Mode GUI), sem dependências pesadas de Chromium, WebView ou Node.js runtime.
- Não introduza dependências desnecessárias no `Cargo.toml`.

### 1.1. Convenções obrigatórias da GUI (`src/app.rs`)

- **Spawns de processos SEMPRE via `quiet_cmd(program)`**: no Windows ele aplica `creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) para não piscar janelas de console. Nunca use `std::process::Command::new` diretamente fora do helper.
- **Versão SEMPRE via `env!("CARGO_PKG_VERSION")`**: a fonte da verdade é o `Cargo.toml`.
- **Todo handler de ação loga no Console Debug**: use `run_logged()` ou `log_debug()`.
- **Status honesto**: estados como `port_active`/`daemon_running` devem refletir verificação real (ex.: teste TCP no endpoint `/mcp`), nunca valores presumidos.
- **Processos de longa duração (túneis: cloudflared/ngrok/ssh) — ciclo de vida obrigatório**: todo túnel iniciado pela GUI DEVE ser rastreado em `HKCU\Software\FzComputerAI` (`tunnel:<provider>:<pid>`), ter um **watchdog independente disparado no start**, ser derrubado no `shutdown_cleanup` e reconciliado na abertura (`startup_reconcile_tracked_tunnels`). **Matar processo de túnel SOMENTE com identidade de 3 fatores** (imagem + `CreationDate` + marcador `run_id` único na command line) — **é PROIBIDO `taskkill /IM cloudflared.exe|ngrok.exe|ssh.exe`**.
- **Ciclo de vida do motor — a GUI é DONA do processo (NORMATIVO)**: `start_daemon` DEVE lançar `cua-driver serve` como **processo filho** da GUI, com porta e token injetados no ambiente e logs direcionados para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`.
- **Encaminhamento LAN é feito PELO PRÓPRIO APP, não por `netsh portproxy`**: uma thread do processo escuta em `<ip_lan>:porta` e copia bytes contra `127.0.0.1:porta`.
- **Limpeza ao fechar é Rust nativo, sem PowerShell e sem elevação**. **É PROIBIDO `taskkill /F /IM cua-driver.exe`**.
- **Pacote de skills é instalado PELO SETUP**: `cua-driver skills install` roda no fim da instalação.

### 1.2. HTTPS do endpoint — convenções

- **A terminação TLS é do APP, não do motor.** O `cua-driver serve` é HTTP-only em `127.0.0.1`.
- **Certificado auto-assinado é cert de SERVIDOR TLS, e só isso.** É **PROIBIDO** instalá-lo em store de confiança da máquina.
- **"Ligado" tem que significar "de pé" (NORMATIVO, v2.3.5):** o supervisor `tls_retry_if_down` cuida da resiliência a cada 30 segundos.

### 1.3. OAuth 2.1 na frente do MCP — convenções

- **Onde vive:** `oauth.rs` + `HttpRewriter` em `tls.rs`.
- **PKCE S256 obrigatório**, rotação de refresh token.
- **Micro-proxy Python (`mcp-oauth-proxy`):** Utilitário opcional incluído no instalador para pontes em ambientes que necessitam de transporte HTTP puro ou adaptação de tokens para conectores legados.

---

### 5. 🗄️ REGRA DE OURO — `archived/` antes de qualquer alteração destrutiva (NORMATIVO)

**Neste projeto nada é apagado.** Antes de **modificar de forma destrutiva, sobrescrever, mover ou remover** qualquer arquivo, faça **backup do arquivo em questão** (ou mova o próprio arquivo) para a pasta `archived/` na raiz do repositório.

- A pasta `archived/` **está no `.gitignore`** e portanto **nunca entra no branch** — ela é histórico/lixo local, não artefato do repositório. Se ela não existir, **crie-a antes** de começar (`mkdir -p archived/`).
- Use um subdiretório com data e motivo, para o histórico ser legível:
  `archived/AAAA-MM-DD-<motivo>/` (ex.: `archived/2026-09-28-atualizacao-docs/`).
- Ao **desrastrear** algo do git, o par correto é: copiar para `archived/`, depois `git rm --cached <arquivo>`, depois mover o arquivo para `archived/`. Nunca `git rm` direto (perde o conteúdo).
- Isso vale também para documentação: ao reescrever um `.md` por inteiro, arquive a versão anterior primeiro.
- **Exceção:** artefatos de build reproduzíveis (`target/`, `dist/`) não precisam de arquivamento — são gerados.

### 5.1. ⛔ NUNCA leia nem varra os exports de conversa (ARMADILHA REAL)

**Não faça `grep`/`Read`/busca recursiva em `.claude/`, `.claude-code-history/` ou `archived/`.** São **exports de conversa de centenas de KB por arquivo**. Uma busca recursiva na raiz cai dentro deles, devolve blocos enormes de JSON/markdown e **estoura o contexto do agente**.

Ao buscar no repositório, **sempre exclua esses diretórios** e prefira alvo explícito:

```bash
grep -rn "<termo>" README.md AGENTS.md CHANGELOG.md docs/ src/
```

### 6. 📚 Documentação e prints (NORMATIVO)

- **Toda alteração funcional obriga varredura de documentação.** Ao mudar comportamento, criar aba/recurso ou mexer em licença/instalação, revise e atualize **todos** os documentos relacionados: `README.md`, `README_EN.md`, `CHANGELOG.md`, `AGENTS.md`, `AGENTES.md`, `INSTALL.md`/`INSTALL_EN.md`, `SKILL.md`, `SIGNING.md`, `LICENSE.md` e a documentação em **`./docs/`**. Se a documentação necessária não existir, **crie-a**.
- **`./docs/` é o lugar da documentação técnica e de uso detalhada.** A raiz guarda o essencial (visão geral, instalação, licença, changelog); o aprofundamento vive em `./docs/`.
- **A home (`README.md` / `README_EN.md`) precisa de prints reais** da interface em `assets/img/`.

---

## 🛠️ Comandos Úteis

### Compilação da GUI Rust

```powershell
cargo build --release --manifest-path Cargo.toml
```

### Compilação do Instalador Windows (Inno Setup)

```powershell
& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" /DAppVersion=2.4.1 installer\fzcomputerai.iss
```

O arquivo gerado será `dist\fzcomputerai-setup-windows-x64.exe`.
