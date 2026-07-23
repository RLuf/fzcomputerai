# FzComputerAI — Interface Gráfica Compilável para o CUA Driver

## Descrição

Criar uma **interface gráfica nativa compilável para Windows** (`fzcomputerai`) que atua como frontend completo do `cua-driver`. O app embute o `cua-driver` como motor, oferece instalação assistida tanto no cliente quanto no servidor, e permite controle interativo do Windows via GUI — cliques, digitação, screenshots ao vivo, automação visual — tudo sem precisar de terminal.

O binário final será um `.exe` Windows standalone, compilado em Rust com `egui` (framework GUI imediato, sem dependências de runtime pesadas), linkando diretamente contra as crates `cua-driver-core` e `platform-windows` já existentes.

---

## Arquitetura

```
fzcomputerai/
├── Cargo.toml           ← novo workspace root (ou member do workspace cua)
├── src/
│   ├── main.rs          ← entry point + loop egui
│   ├── app.rs           ← estado global da aplicação (AppState)
│   ├── tabs/
│   │   ├── install.rs   ← aba: Instalação & Configuração (cliente/servidor)
│   │   ├── control.rs   ← aba: Controle Interativo Windows
│   │   ├── skills.rs    ← aba: Gerenciar Skills
│   │   └── logs.rs      ← aba: Log / Output
│   ├── driver.rs        ← bridge para cua-driver (spawn MCP, call tools)
│   └── mcp_client.rs    ← cliente MCP stdio para comunicar com driver em proc
```

### Stack Técnica

| Componente      | Tecnologia                                                 |
| --------------- | ---------------------------------------------------------- |
| GUI framework   | `egui 0.29` + `eframe` (imediato, compilado nativamente)   |
| Backend motor   | `cua-driver-core` + `platform-windows` (crates existentes) |
| Runtime async   | `tokio` (já no workspace)                                  |
| Captura de tela | `platform-windows::capture` (já existente)                 |
| Comunicação     | MCP stdio em processo, canal `tokio::mpsc`                 |
| Compilação      | `cargo build --release` → exe standalone                   |

---

## User Review Required

> [!IMPORTANT]
> O `fzcomputerai` deve ser um **crate novo** dentro do workspace Rust existente em `g:\fzcomputerai\cua\libs\cua-driver\rust\` **ou** um workspace independente em `g:\fzcomputerai\`. Qual você prefere? Conecta junto na mesma tela que estou utilizando com opcao de interacao ou nao.
>
> - **Opção A**: Membro do workspace `cua` — reutiliza deps do workspace, build conjunto com o driver
> - **Opção B**: Workspace standalone em `g:\fzcomputerai\` — independente, referencia o driver como path dependency

> [!WARNING]
> A GUI usará **egui** (puro Rust, sem WebView, sem Electron). Isso garante binário único ~5-10 MB. Alternativa seria `tauri` (WebView), mas requer Node.js no build. Egui é recomendado para simplicidade de compilação.

---

## Open Questions

> [!IMPORTANT]
>
> 1. **Modo cliente vs servidor**: O app roda no **mesmo Windows** que será controlado (host mode) **ou** controla um **servidor remoto** (outra máquina via TCP/named pipe)?
> 2. **Autenticação no servidor**: Se precisar controlar máquina remota, quer autenticação básica (token) ou integração com algo específico? Sim ele deve ter integracao com meu software orchestrador FAZAI-NG http://github.com/RLuf/fazai-ng-pv (privado), esta rodando no servidor ssh -P31052 rluft@walker:/home/rluft/fazai-ng/ (192.168.0.22) SENHA 7878, SOMENTE LEITURA, NAO PODE ALTERAR.Por padrao ele usa anthropic mas pode fazer fallback. podmemos considerar que ele usa opus-4.8...
> 3. **Skills**: Quer instalar as skills do `cua-driver` para os agentes (Claude Code, Antigravity etc.) via GUI, ou as "skills" aqui são os recursos da GUI em si? as opcoes na interface, vc nao procisa incluir o cua na interface pode apenas colocar um downloadre bonito com sttus bar....
>    Explore a documentacao em https://cua.ai/docs/how-to-guides e traga para a interface sa funcionalidade use licenca CC4.0 Roger Luft roger@webstorage.com.br coloca um botao de help ensinando o que faz cada coisa, sobre/about com botao donate. adicione o log da webstorgae em algum lugar, o github ja esta pronto, se tiver actions faca o workflow para compilar la tambem sempre que vier a tag v sair a verso stable e ao abrir o app ele verificar no repo ou no site e o botao o update acisar o botao do donate +55 51 99242539

## aprimore com opcao de auto install do windows (desktop a ser usado) para o cliente (linux/windows), onde o usuario fornece num campo os daos apra acesso ai automaticamente ele entrra verifica se ja tem algum dos cli e mana ver senao manda baixar...

## Proposed Changes

### [NEW] Crate `fzcomputerai` — GUI App

#### [NEW] `Cargo.toml` (g:\fzcomputerai\Cargo.toml)

Workspace standalone com dependency path para `cua-driver-core` e `platform-windows`.

#### [NEW] `src/main.rs`

Entry point Windows com `#![windows_subsystem = "windows"]` (sem console). Inicializa eframe + AppState.

#### [NEW] `src/app.rs`

`AppState` — estado global compartilhado via `Arc<Mutex<>>`. Contém:

- Tab ativa
- Status do driver (rodando / parado)
- Buffer de screenshot (imagem atual)
- Log ring buffer
- Config de conexão (host, porta, token)

#### [NEW] `src/driver.rs`

Bridge async que:

1. Spawna `cua-driver serve` (ou usa embedded mode)
2. Comunica via named pipe `\\.\pipe\cua-driver`
3. Expõe `call_tool(name, args)` → `Result<serde_json::Value>`
4. Mantém screenshot loop periódico (100 ms)

#### [NEW] `src/tabs/install.rs`

**Aba Instalação & Configuração**:

- Detecta se `cua-driver.exe` está instalado (`which cua-driver`)
- Botão "Instalar Driver" → roda o PowerShell install script
- Botão "Instalar Skills" → `cua-driver skills install`
- Seção "Configurar como Servidor" → `cua-driver autostart enable` + mostra status
- Seção "Conectar a Servidor Remoto" → campos host/porta/token
- Mostra saúde: `cua-driver doctor`

#### [NEW] `src/tabs/control.rs`

**Aba Controle Interativo Windows**:

- Painel esquerdo: screenshot ao vivo (atualiza a cada ~200ms)
- Click no screenshot → envia `mouse_click` com coordenadas proporcionais
- Painel direito: controles manuais
  - Campo texto → botão "Digitar"
  - Atalho de teclado (hotkey)
  - Scroll (cima/baixo)
  - Botão "Screenshot Manual"
  - Zoom em janela (lista janelas abertas)

#### [NEW] `src/tabs/skills.rs`

**Aba Skills**:

- Lista skills instaladas por agente (Claude Code, Antigravity, Codex…)
- Botões: Instalar / Atualizar / Desinstalar
- Mostra path e status de cada symlink

#### [NEW] `src/tabs/logs.rs`

**Aba Logs**:

- Ring buffer dos últimos 500 logs do driver
- Auto-scroll
- Filtro por nível (INFO/WARN/ERROR)
- Botão "Limpar"

---

## Verification Plan

### Build & Compilação

```powershell
cargo build --release --manifest-path g:\fzcomputerai\Cargo.toml
# Deve gerar: g:\fzcomputerai\target\release\fzcomputerai.exe
```

### Testes Manuais

1. Abrir `fzcomputerai.exe` — janela aparece sem console
2. Aba Instalação: detecta `cua-driver` no PATH
3. Aba Controle: screenshot ao vivo aparece em <1s
4. Click no screenshot → cursor move na tela real
5. Campo de texto → "Digitar" → texto aparece na janela focada
6. Aba Skills → instala skill para Antigravity (`~/.gemini/skills/`)

Crie um instalador BuildIn que registra auto assina seleciona pra qual coder... instalador padrao instalacao normal baixa e instala todos, avancado somente baixa e instalaum o que o cliente pedir
