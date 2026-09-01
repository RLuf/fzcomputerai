# FzComputerAI — Computer Vision via Model Context Protocol (MCP)

<div align="center">

![GitHub Release](https://img.shields.io/github/v/release/RLuf/fzcomputerai)
![MIT License](https://img.shields.io/badge/Licen%C3%A7a-MIT-green.svg)
![Plataformas](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen.svg)
![MCP Enabled](https://img.shields.io/badge/MCP-Server%20Ready-orange.svg)
![HTTP TCP Transport](https://img.shields.io/badge/Transport-Stdio%20%7C%20HTTP%20TCP%20:8000-purple.svg)
[![Patrocinar](https://img.shields.io/badge/%E2%99%A5-Patrocinar-e91e63.svg)](https://github.com/sponsors/RLuf)

<p align="center">
  <strong>Servidor Nativo de Visão Computacional Multimodal & Automação Desktop para Agentes de IA</strong>
</p>

[Português (BR)](README.md) | [English (US)](README_EN.md)

</div>

---

> **FzComputerAI** é a **interface gráfica nativa** que gerencia um servidor de **Visão Computacional e Automação de Interface (UI)** acessível via **Model Context Protocol (MCP)**. Ela permite que Agentes de IA (Claude Code, Antigravity, FazAI-NG, Cursor, Windsurf, LLMs locais) enxerguem a tela e operem qualquer aplicativo de desktop — e cuida da parte chata: subir o motor, configurar a porta, provar que o endpoint responde de verdade e publicar o acesso na LAN ou na internet com segurança.
>
> O motor de automação é o **`cua-driver`**, do projeto open-source [**Cua**](https://github.com/trycua/cua) (MIT, Cua AI, Inc.). O FzComputerAI **não substitui o motor** — é o cockpit dele.

---

## 🖼️ A ferramenta

<div align="center">

**MCP & Rede** — controle do motor, porta, encaminhamento e diagnóstico com estado **real** (nada presumido)

![Aba MCP & Rede](assets/img/screenshot-mcp-rede.png)

**Túnel (Internet)** — publica o MCP local numa URL HTTPS via Cloudflare, ngrok ou SSH reverso

![Aba Túnel](assets/img/screenshot-tunel.png)

**MCP Tools** — catálogo das ferramentas do motor, com filtro e execução direto da tela

![Aba MCP Tools](assets/img/screenshot-mcp-tools.png)

</div>

---

## ✨ O que a interface entrega

| Recurso | O que faz |
| :--- | :--- |
| **Ciclo de vida do motor** | A GUI é **dona** do daemon: lança o `cua-driver serve` como **processo filho**, com porta e token injetados no ambiente, e o `stdout`/`stderr` do motor vai para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`, que o console segue como `tail -f` (prefixo `[motor]`). **Iniciar** não derruba um motor que já responde — para trocar de processo existe **Reiniciar**. Ao fechar o app, o motor é encerrado e a configuração temporária desfeita por chamadas nativas curtas, sem PowerShell e sem elevação. |
| **Status honesto** | Nenhum estado é presumido: o teste é um `POST initialize` JSON-RPC de verdade, e o verde de LAN só aparece com listener confirmado no `netstat` **e** resposta do endpoint. |
| **Acesso pela LAN** | Encaminhamento feito **pelo próprio app**: uma thread do processo escuta em `<ip_lan>:porta` e copia os bytes contra `127.0.0.1:porta`. É TCP puro — `curl`, `telnet` e `nc` atravessam igual. Não pede admin/UAC e não deixa resíduo: ao fechar o app as duas portas fecham junto. O `netsh portproxy` continua **apenas como fallback**, quando o bind no IP da LAN falha — nesse caso valem o badge de **3 estados** (funcionando / sem efeito / sem regra) e a limpeza rastreada, que só remove as regras criadas pelo app. |
| **HTTPS no endpoint** | **[v2.2.0]** Listener TLS dentro do app (`rustls`), mesma mecânica do encaminhamento LAN: `https://<ip>:8443/mcp` encaminha para o motor em `http://127.0.0.1:8000/mcp`. Certificado **auto-assinado gerado sozinho** (no setup via `--tls-init` ou no primeiro run — o que vier primeiro; renovado antes de expirar), **Let's Encrypt** com um clique (domínio público + porta 80; renovação automática) ou o seu próprio `.crt/.key`. Nada é instalado em store de confiança: o cliente confia pelo `.crt` ou pelo SHA-256 mostrado na tela. O bearer token continua obrigatório. Detalhes em [docs/https.md](docs/https.md). |
| **Acesso pela internet** | Aba **Túnel**: Cloudflare Tunnel (quick ou nomeado), ngrok e SSH reverso. Túnel de **saída** — não precisa abrir porta no roteador. |
| **Senha na URL** | Autenticação nível 1 por um porteiro local: a URL vira `https://…/s/<senha>/mcp` e sem a senha o acesso recebe 404. |
| **Sonda de exposição** | O app testa a **URL pública** com uma requisição sem credencial e mostra o resultado verificado — exposto, barrado pela borda, ou não verificável. |
| **Túnel não sobrevive ao app** | Quatro camadas de limpeza (incluindo watchdog que age em `taskkill /F` e crash), matando apenas o processo comprovadamente nosso. |
| **Central de Atualizações** | Verifica e atualiza **dois** componentes: esta interface (instalador baixado em segundo plano com SHA256 conferido — só a troca final pede confirmação) e o **motor** (atualização automática de ponta a ponta pela API oficial dele, `check-update` / `update --apply`, com fallback para o instalador oficial do Cua). |
| **Catálogo MCP Tools** | Lista, filtra e executa as ferramentas de visão e automação sem sair da interface. |
| **Console único** | Um console global no rodapé, visível em todas as seções, rolando como `tail -f`: acompanha sozinho, pausa quando você rola para ler e volta a acompanhar no botão **Ir ao fim**. |
| **Bilíngue e nativo** | PT-BR / English em tempo real. Rust + `egui`, sem Chromium, sem WebView, sem runtime Node. |

---

## 💎 Patrocinadores & Apoio

<div align="center">

| Patrocinador | Website | Foco |
| :--- | :--- | :--- |
| **Webstorage Tecnologia** | [www.webstorage.com.br](https://www.webstorage.com.br) | Soluções em Infraestrutura, Cloud & Automação Inteligente |
| **Imóvel Site** | [www.imovelsite.com.br](https://www.imovelsite.com.br) | Plataforma de Gestão e Tecnologia Imobiliária |

</div>

---

## 🚀 Recursos Principais (Computer Vision via MCP)

O servidor expõe um conjunto de ferramentas MCP (*MCP Tools*) padronizadas para análise de visão computacional multimodal e controle de desktop:

### 👁️ Visão & Inspeção Visual

| Ferramenta MCP | Descrição |
|---|---|
| `get_desktop_state` | Captura a imagem completa do Desktop (Visão Computacional), lista todas as janelas ativas, coordenadas e estado do cursor. |
| `get_window_state` | Realiza a captura focada de uma janela específica e extrai a árvore de acessibilidade (UI Automation / Accessibility Tokens). |
| `take_screenshot` | Gera uma captura de tela multimodal otimizada (PNG/JPEG base64) para consumo direto por modelos de visão (Gemini 1.5/2.0, Claude 3.5 Sonnet/Opus, GPT-4o). |

### 🖱️ Ações de Ponteiro & Automação

| Ferramenta MCP | Descrição |
|---|---|
| `mouse_click` | Executa cliques com o botão esquerdo, direito ou do meio em coordenadas específicas $(x, y)$ ou sobre elementos identificados. |
| `mouse_move` | Move o cursor para posições absolutas no desktop ou relativas dentro de uma janela. |
| `mouse_drag` | Executa movimentos de arrastar e soltar (Drag and Drop) com controle suave de trajetória. |
| `mouse_down` / `mouse_up` | Controle granular de pressionar e soltar botões do mouse. |

### ⌨️ Teclado & Atalhos

| Ferramenta MCP | Descrição |
|---|---|
| `keyboard_type` | Simula a digitação de texto com suporte a sanitização de caracteres unicode e acentuação PT-BR. |
| `keyboard_press` | Envia teclas individuais ou combinações específicas (ex: `Enter`, `Tab`, `Escape`). |
| `shortcut` | Executa atalhos de sistema complexos (ex: `Ctrl+C`, `Ctrl+V`, `Alt+Tab`, `Cmd+Space`). |

### 🛠️ Gerenciamento de Aplicações & Gravação

| Ferramenta MCP | Descrição |
|---|---|
| `launch_app` | Inicia aplicativos do sistema por nome ou caminho executável. |
| `close_app` | Encerra janelas ou processos em execução. |
| `recording_start` / `recording_stop` | Inicia e finaliza gravações de vídeo da sessão em tempo real. |

---

## 🖥️ Interface Gráfica Nativa (GUI Rust `fzcomputerai v2.2.0`)

GUI nativa em Rust (`egui`/`eframe`, sem Chromium ou WebView), bilíngue **PT-BR / English** com alternância em tempo real. Organizada em **7 abas**:

| Aba | Função |
| :--- | :--- |
| **MCP & Rede** | **[NOVO v2.2.0] HTTPS do endpoint** — terminação TLS no próprio app (`https://<ip>:8443/mcp` -> `http://127.0.0.1:8000/mcp`) com certificado **auto-assinado gerado na instalação ou no primeiro run**, **Let's Encrypt** (ACME HTTP-01, renovação automática) ou certificado próprio; badge verde só com handshake TLS + JSON-RPC reais, fingerprint SHA-256 na tela. Configuração da porta HTTP do servidor MCP (`CUA_DRIVER_RS_MCP_HTTP_PORT`), encaminhamento LAN feito pelo próprio app (`netsh portproxy` só como fallback), teste real do endpoint `/mcp` via TCP, URL de rede com IP LAN autodetectado, botão **Verificar e Atualizar** (GitHub Releases com auto-installer), **Iniciar com o Windows** (autostart) e **Console Debug** deduplicado com rolagem automática. |
| **MCP Tools** | **[NOVO v2.0.0]** Catálogo visual completo para listar, filtrar por categoria e invocar interativamente qualquer ferramenta MCP do motor CUA. |
| **Túnel (Internet)** | **[NOVO v2.1.0]** Expõe o MCP HTTP local na internet (HTTPS público -> HTTP local) por **Cloudflare Tunnel** (quick + nomeado via login OAuth/token), **ngrok** e **SSH reverso** (servidor próprio ou localhost.run/serveo). Captura a URL pública, gera o snippet `mcpServers` e testa de verdade por POST `initialize` na URL pública (sonda de exposição). Autenticação **nível 1 = senha na URL** via porteiro local (`/s/<senha>/mcp`). Ciclo de vida limpo: o túnel nunca sobrevive ao app. **O motor tem autenticação própria — medido em 2026-08-03 no `cua-driver` 0.17.0: toda requisição sem `Authorization: Bearer <token>` recebe HTTP 401 `{"code":-32001,"message":"Authentication required"}`. A senha na URL é uma camada adicional na borda, não substituta do token.** |
| **Calibração & Visão** | Calibração de tela, DPI scaling e teste de clique por coordenadas. |
| **Janelas & Processos** | Listagem de janelas ativas, inspeção UIA e lançamento de aplicativos. |
| **Gravação Trajetória** | Início e parada de gravações de sessão/trajetória. |
| **Doctor & Skills** | Diagnóstico de saúde (`doctor`) e instalação/atualização/remoção de skills. |

Destaques da v2.0.0 Stable:
- **Catálogo MCP Tools**: execute chamadas CLI das ferramentas de visão e automação diretamente na GUI.
- **Auto-Upgrade Inteligente**: checagem direta de releases no GitHub com download e instalação automática.
- **Console Debug Formatado**: logs organizados com 2 linhas em branco de espaçamento e rolagem automática ao final.
- **Instalador com Limpeza Automática**: encerramento de versões legadas e remoção de chaves antigas do Registro antes da nova instalação.

---

## 🛠️ Arquitetura do Sistema & Modos de Conexão

```
  ┌────────────────────────────────────────────────────────────────────────┐
  │                 Agente de IA / Orquestrador Remoto                     │
  │        (Antigravity / FazAI-NG / Claude Code / Cursor / Windsurf)       │
  └───────────────────────────────────┬────────────────────────────────────┘
                                      │
           ┌──────────────────────────┴──────────────────────────┐
           │ Modo Stdio (Local)       │ Modo HTTP TCP/IP (:8000) │
           ▼                          ▼                          ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │               FzComputerAI — MCP Computer Vision Server                │
  │                           (cua-driver engine)                          │
  ├────────────────────────────────────┬───────────────────────────────────┤
  │       Captura de Tela (WGC/DX)     │    Injeção de Input (SendInput)   │
  └────────────────────────────────────┴───────────────────────────────────┘
```

---

## 🌐 Conexão Remota via TCP/IP HTTP (Orquestradores como FazAI-NG)

Além do modo local `stdio`, o servidor suporta conexão remota via protocolo **HTTP TCP/IP**. Isso permite que um orquestrador rodando em um servidor separado (ex: Linux) controle desktops na rede:

### Ativando a Porta HTTP no Servidor (Windows):
```powershell
# Ativar porta TCP 8000 para o servidor MCP
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '8000', 'User')
# Token obrigatório: qualquer string aleatória gerada por você
# (o próprio motor a chama de "host-generated bearer token" — quem gera é o host;
#  usando a GUI, ela faz isso por você — veja a nota abaixo)
[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_TOKEN', '<seu-token>', 'User')
cua-driver stop
# Suba o motor com as variáveis já valendo no ambiente DESTE processo
$env:CUA_DRIVER_RS_MCP_HTTP_PORT = '8000'
$env:CUA_DRIVER_RS_MCP_HTTP_TOKEN = '<seu-token>'
cua-driver serve
```

> **Medido em 2026-08-03 no binário `cua-driver` 0.17.0** (execução real, não citação de documentação): sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do processo, o `cua-driver serve` **nem sobe** — sai com código 1 e a mensagem `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`.
>
> Atenção ao autostart: a Scheduled Task `cua-driver-serve` (usada por `autostart kick`) **herda o ambiente do logon**, então um token gravado depois de você logar só é enxergado no próximo logon — até lá o daemon sobe sem token, morre na hora e a porta fica muda. E como o processo nasce filho do **Agendador de Tarefas**, o `stdout` pertence à task: os logs do motor — inclusive a atividade de clientes MCP externos, como o conector do Claude, o Antigravity e o Cursor — simplesmente somem.
>
> Por isso a GUI **não usa mais** o `autostart kick` para subir o motor: ela lança o `cua-driver serve` como **processo filho**, com porta e token injetados no ambiente, e manda `stdout`+`stderr` para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`, que o console segue como `tail -f` (prefixo `[motor]`). A Scheduled Task fica como **último recurso**, e nesse caso o console avisa que o processo não é da GUI e que não haverá logs. Só pode existir **um** daemon por vez — e se o endpoint já responde, o botão **Iniciar** não encosta nele: no Windows a porta recém-usada fica retida em `TIME_WAIT` por minutos, um `serve` novo perderia o bind (`MCP HTTP transport disabled — bind 127.0.0.1:8000 failed (os error 10048)`) e sobraria um daemon zumbi (pipe vivo, porta muda). Para forçar a troca de processo, use **Reiniciar**.
>
> **O token você não precisa criar:** na primeira vez que precisa dele, a GUI gera 32 bytes do RNG do Windows (64 caracteres hex) e persiste em `HKCU\Environment`. O valor nunca aparece no log; para lê-lo:
> ```powershell
> reg query HKCU\Environment /v CUA_DRIVER_RS_MCP_HTTP_TOKEN
> ```

### Configurando o Cliente HTTP / Orquestrador:
- **Endpoint**: `http://<IP_DO_WINDOWS>:8000/mcp` — ou, com o HTTPS ligado na aba *MCP & Rede*, `https://<IP_DO_WINDOWS>:8443/mcp` (auto-assinado: o cliente precisa confiar no `.crt`/fingerprint; Let's Encrypt: `https://<seu-dominio>:8443/mcp`, confiado por qualquer cliente). Ver [docs/https.md](docs/https.md).
- **Método**: `POST`
- **Header**: `Content-Type: application/json`
- **Header**: `Authorization: Bearer <seu-token>` — **obrigatório**. Medido em 2026-08-03: sem ele, `POST /mcp`, `GET /mcp` e `GET /` respondem os três o mesmo HTTP 401 com `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` (e **sem** header `WWW-Authenticate`). A conexão TCP em si é aceita normalmente — a recusa acontece na camada de aplicação. Com o header correto: HTTP 200 com o `result` do `initialize`.

---

## 📦 Instalação Rápida

### 🪟 Windows — Instalador (recomendado)

Baixe o **`fzcomputerai-setup-windows-x64.exe`** na [página de releases](https://github.com/RLuf/fzcomputerai/releases/latest) e execute.

O instalador (Inno Setup, bilíngue PT-BR / English) faz:

- **Instala a GUI** em `%LOCALAPPDATA%\Programs\FzComputerAI` — no caso padrão **não pede UAC**; no diálogo é possível optar por instalar para todos os usuários (aí sim eleva).
- **Cria atalho** no Menu Iniciar e, opcionalmente, na Área de Trabalho.
- **Opção "Iniciar o FzComputerAI com o Windows"** (autostart) — grava exatamente a mesma chave `HKCU\...\Run` usada pelo checkbox da aba *MCP & Rede*, de modo que GUI e instalador nunca se contradizem.
- **Opção "Instalar o motor `cua-driver`"** (desmarcada por padrão, requer internet) — executa o instalador **oficial** do projeto cua, que instala a **última versão estável** publicada.
- **Instala o pacote de skills** ao final da instalação (`cua-driver skills install`). Sem esses symlinks o agente conecta no MCP e **não enxerga ferramenta nenhuma** — e quem acabou de instalar não teria como adivinhar que precisa clicar num botão da aba *Doctor & Skills*. É idempotente e, pelo help oficial do motor, *"Never overwrites existing user links"*; os quatro alvos são Claude Code, Codex, Antigravity e Hermes.
- **Registra um desinstalador** em *Configurações → Aplicativos → Aplicativos instalados*. Ele remove a GUI; o `cua-driver` tem ciclo de vida próprio e **não** é removido junto (o desinstalador avisa isso na tela).

> ⚠️ **Aviso do SmartScreen — leia antes de executar**
>
> Os binários deste projeto **ainda não são assinados digitalmente**. Ao abrir o instalador, o Windows exibirá *"O Windows protegeu o seu PC"*: clique em **Mais informações → Executar assim mesmo**.
>
> **O instalador não contorna esse aviso** — um instalador não assinado recebe exatamente o mesmo bloqueio que um `.exe` avulso. Antes de executar, confira o arquivo `.sha256` publicado ao lado do binário no release:
> ```powershell
> Get-FileHash .\fzcomputerai-setup-windows-x64.exe -Algorithm SHA256
> ```
> Contexto completo, opções de certificado e custos: **[SIGNING.md](SIGNING.md)**.

### 🪟 Windows — Alternativas

**a) Binário portátil** — baixe o `fzcomputerai-windows-x64.exe` do release e execute direto: sem instalação, sem atalhos, sem autostart e sem desinstalador; a atualização é manual. O mesmo aviso de SmartScreen se aplica.

**b) Build local do instalador (para quem compila do fonte)** — o antigo `install.ps1` da raiz foi removido; o instalador gráfico é o único caminho de instalação no Windows. Quem compila do código-fonte gera o mesmo instalador localmente:
```powershell
cargo build --release --manifest-path fzcomputerai/Cargo.toml
ISCC.exe /DAppVersion=<versao> installer\fzcomputerai.iss
```
> Requer o [Inno Setup](https://jrsoftware.org/isinfo.php) instalado (`ISCC.exe` no PATH ou caminho completo). O `fzcomputerai-setup-windows-x64.exe` resultante fica em `dist/`.

### 🐧 Linux & 🍎 macOS — Instalação Remota via Bash (One-liner)
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash
```

Para simular a instalação sem alterar nada no sistema (`--dry-run`):
```bash
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash -s -- --dry-run
```

> **Nota:** o instalador remoto via binário oficial instala a **GUI `fzcomputerai`**. O servidor MCP stdio continua sendo o `cua-driver` — o script imprime o snippet `.mcp.json` correspondente e orienta o uso de `npx fzcomputerai mcp` (o fallback de compilação a partir do código-fonte também compila o `cua-driver`).

### 📦 Via NPM (Global)
```bash
npm install -g fzcomputerai
```

### 🧱 Compilação a partir do Código Fonte / Pacote Tarball (.tgz)
```bash
# Baixar ou extrair o pacote de código-fonte .tgz:
tar -xzf fzcomputerai-<versão>.tgz
cd package (ou fzcomputerai)

# Compilação do motor e da GUI Rust:
cargo build --release --manifest-path fzcomputerai/Cargo.toml
```

Para instruções detalhadas de compilação e configurações avançadas, consulte o [INSTALL.md](INSTALL.md).

---

## ⚙️ Configuração nos Clientes MCP Locais

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

## 🤝 Patrocinadores Oficiais & Apoio (Patrons)

<div align="center">

| Patrocinador | Logo | Website Oficial |
| :--- | :---: | :--- |
| **Webstorage Tecnologia** | <a href="https://www.webstorage.com.br"><img src="assets/img/webstorage-logo.png" width="180" alt="Webstorage Tecnologia"></a> | [www.webstorage.com.br](https://www.webstorage.com.br) |
| **Imóvel Site** | <a href="https://www.imovelsite.com.br"><img src="assets/img/imovelsite-logo.png" width="180" alt="Imóvel Site"></a> | [www.imovelsite.com.br](https://www.imovelsite.com.br) |

</div>

---

## 📚 Documentação

A documentação detalhada vive em [`docs/`](docs/README.md):

| Documento | Para quê |
| :--- | :--- |
| [Arquitetura](docs/arquitetura.md) | Como GUI e motor se dividem, transporte MCP, onde vive o estado e o princípio de status honesto |
| [Aba MCP & Rede](docs/uso-mcp-rede.md) | Ciclo de vida do motor, porta, encaminhamento LAN e leitura do diagnóstico |
| [HTTPS no endpoint](docs/https.md) | Ligar `https://` no MCP: auto-assinado automático, Let's Encrypt, certificado próprio, como o cliente confia |
| [Aba Túnel](docs/uso-tunel.md) | Cloudflare, ngrok e SSH reverso passo a passo, senha na URL e sonda de exposição |
| [Acesso remoto](docs/acesso-remoto.md) | LAN × túnel × VPN, e **por que não existe bind `0.0.0.0`** |
| [Atualização](docs/atualizacao.md) | Central de Atualizações: interface e motor, e o que é verificado em cada um |
| [Solução de problemas](docs/solucao-de-problemas.md) | Sintoma → causa → verificação → correção |
| [Desenvolvimento](docs/desenvolvimento.md) | Compilar, convenções obrigatórias do código e build do instalador |
| [FAQ](docs/faq.md) | Perguntas diretas, com respostas honestas |

---

## 📜 Licença & Créditos

- **Motor / Projeto base:** o `cua-driver` é parte do projeto open-source [**Cua** (`trycua/cua`)](https://github.com/trycua/cua), desenvolvido e mantido por **Cua AI, Inc.** (equipe [cua.ai](https://cua.ai)) sob **MIT License** — `Copyright (c) 2025 Cua AI, Inc.` O FzComputerAI é uma **interface gráfica independente** construída sobre esse motor; não o modifica nem o redistribui. **Nosso agradecimento sincero à Cua AI, Inc. e à comunidade do Cua** — sem o trabalho deles este projeto não existiria. Comunidade: [Discord](https://discord.gg/mVnXXpdE85) · Docs: [cua.ai/docs](https://cua.ai/docs)
- **Autor & Integrações FzComputerAI:** Roger Luft (VeilWalker) — Webstorage Tecnologia (`roger@webstorage.com.br`)
- **Licença:** [MIT](LICENSE.md) — a mesma do projeto Cua, para máxima compatibilidade. O texto integral, os componentes de terceiros e a citação formal do Cua estão em [`LICENSE.md`](LICENSE.md).
- **Apoie o projeto:** [GitHub Sponsors](https://github.com/sponsors/RLuf)
- **Patrocinadores:** [Webstorage Tecnologia](https://www.webstorage.com.br) | [Imóvel Site](https://www.imovelsite.com.br)
