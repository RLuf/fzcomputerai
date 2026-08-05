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

**Túnel (Internet)** — publica o MCP local numa URL HTTPS via Cloudflare, ngrok ou SSH reverso, com sonda de exposição em **2 fases** (sem e com Bearer) e o veredito **PROTEGIDO E FUNCIONAL** verificado de ponta a ponta

![Aba Túnel](assets/img/screenshot-tunel.png)

**Token do motor** — motores `cua-driver` 0.16+ exigem token Bearer e são *fail-closed* sem ele (401 para tudo); a aba avisa em vermelho e o botão **"Gerar e ativar token do motor"** gera, grava e reinicia o daemon em um clique

![Aba Túnel — token do motor](assets/img/screenshot-tunel-token.png)

**MCP Tools** — catálogo das ferramentas do motor, com filtro e execução direto da tela

![Aba MCP Tools](assets/img/screenshot-mcp-tools.png)

</div>

---

## ✨ O que a interface entrega

| Recurso | O que faz |
| :--- | :--- |
| **Ciclo de vida do motor** | Iniciar, parar e reiniciar o `cua-driver` em um clique. O autostart do Windows e o **da GUI** (abrir o app junto com o sistema) - o motor nao usa tarefa agendada. Ao abrir, o app sobe o motor sozinho **como processo filho** (se nada estiver respondendo na porta); ao fechar, o motor que ele mesmo subiu é encerrado e a configuração temporária desfeita. |
| **Status honesto** | Nenhum estado é presumido: o teste é um `POST initialize` JSON-RPC de verdade, e o verde de LAN só aparece com listener confirmado no `netstat` **e** resposta do endpoint. |
| **Acesso pela LAN** | Relay TCP dentro do próprio app: escuta em `0.0.0.0:<porta>` (ou num IP escolhido no campo *Escutar em*) e encaminha para o `127.0.0.1:<porta>` do motor, copiando bytes nos dois sentidos sem inspecionar HTTP — keep-alive e SSE passam intactos. **Não pede UAC**, não deixa regra no sistema e morre junto com o app. Badge **PUBLICADO NA REDE / SÓ LOCAL** com contador real de conexões (ativas/total). Regras `netsh portproxy` **legadas** continuam removíveis — o botão só aparece quando existe alguma. |
| **Acesso pela internet** | Aba **Túnel**: Cloudflare Tunnel (quick, sem conta, ou nomeado com domínio próprio), ngrok e SSH reverso. Túnel de **saída** — não precisa abrir porta no roteador. O teste da URL pública roda em segundo plano: a interface não congela enquanto ele acontece. |
| **Domínio próprio (URL fixa)** | Fluxo completo pela GUI para o Cloudflare nomeado: **Login** (OAuth no navegador) → **Verificar login** (confere de verdade se o `cert.pem` existe) → **Criar túnel + apontar DNS** → **Iniciar túnel**. O login sozinho não cria nada — ele só baixa o certificado; por isso os dois passos seguintes existem. O domínio precisa já estar na sua conta Cloudflare (nameservers delegados). |
| **Senha na URL** | Autenticação nível 1 por um porteiro local: a URL vira `https://…/s/<senha>/mcp` e sem a senha o acesso recebe 404. |
| **URL sozinha basta** | Clientes que só aceitam **uma URL** (Claude Desktop, por exemplo) não têm onde colar o header `Authorization`. Quem provou a senha no caminho já está autenticado perante o app, então o porteiro acrescenta o `Bearer` ao falar com o motor — e se o cliente mandar o próprio `Authorization`, o dele vence. O segredo do motor não viaja pela internet; a credencial pública passa a ser a senha da URL. |
| **Sonda de exposição** | Teste da **URL pública** em **2 fases**: primeiro sem credencial (exposto, barrado pelo motor, barrado pela borda, ou não verificável); depois, se a GUI conhece o token do motor, repete **com** `Authorization: Bearer` — sem credencial barrado **e** com Bearer `initialize` OK rende o veredito **PROTEGIDO E FUNCIONAL**. |
| **Token do motor** | Motores `cua-driver` 0.16+ exigem token Bearer no endpoint `/mcp` (sem token configurado: *fail-closed*, 401 para tudo). A GUI gera o token (CSPRNG), grava em `HKCU\Environment` sem logar o valor, reinicia o daemon e o snippet `mcpServers` já sai com o header `Authorization`. |
| **Tudo morre com o app** | Motor e túnel são **processos filhos** adotados num **Job Object** do Windows: quem os encerra é o kernel, junto com a GUI — no X, no *Sair* da bandeja, num `taskkill /F`, num crash ou no logoff (verificado). Um motor de terceiro que já esteja respondendo na porta é detectado, **não** é duplicado nem encerrado, e a interface avisa que ele não será fechado junto. O watchdog do túnel ficou como *fallback*, para o caso de a adoção no Job falhar. |
| **Central de Atualizações** | Verifica e atualiza **dois** componentes: esta interface (instalador com SHA256 conferido) e o **motor** (pela API oficial dele, `check-update` / `update --apply`). |
| **Catálogo MCP Tools** | Lista, filtra e executa as ferramentas de visão e automação sem sair da interface. |
| **Console único** | Um console global no rodapé, visível em todas as seções, rolando como `tail -f`: acompanha sozinho e pausa quando você rola para ler. |
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
| `get_desktop_state` | Captura a imagem do Desktop (Visão Computacional), lista as janelas ativas e o estado do cursor. |
| `get_window_state` | Captura focada de uma janela específica + árvore de acessibilidade (UIA) com tokens de elemento clicáveis. |
| `zoom` | Amplia uma região da janela para inspeção fina; as coordenadas do zoom podem ser usadas nos cliques (`from_zoom`). |
| `get_accessibility_tree` | Extrai a árvore de acessibilidade completa para navegação semântica. |

### 🖱️ Ações de Ponteiro & Automação

| Ferramenta MCP | Descrição |
|---|---|
| `click` / `double_click` / `right_click` | Cliques por token de elemento (UIA, funciona em segundo plano) ou por pixel do screenshot. |
| `move_cursor` | Move o cursor para posições na janela ou no desktop. |
| `drag` | Arrastar e soltar (Drag and Drop) com trajetória controlada. |
| `scroll` | Rolagem vertical/horizontal na janela alvo. |

### ⌨️ Teclado & Atalhos

| Ferramenta MCP | Descrição |
|---|---|
| `type_text` | Digita texto (com suporte a Unicode/acentuação PT-BR), inclusive em segundo plano via UIA. |
| `press_key` | Envia teclas individuais (ex.: `Enter`, `Tab`, `Escape`). |
| `hotkey` | Executa combinações (ex.: `Ctrl+C`, `Ctrl+V`, `Alt+Tab`). |

### 🛠️ Gerenciamento de Aplicações & Gravação

| Ferramenta MCP | Descrição |
|---|---|
| `launch_app` / `kill_app` / `list_apps` | Inicia, encerra e lista aplicativos. |
| `list_windows` / `bring_to_front` / `set_window_frame` | Enumera janelas, traz para frente e posiciona/redimensiona com verificação real. |
| `start_recording` / `stop_recording` / `replay_trajectory` | Grava a sessão e reexecuta trajetórias gravadas. |
| `verify_state` | Verifica pós-condições de forma determinística (nunca presume sucesso). |

---

## 🖥️ Interface Gráfica Nativa (GUI Rust `fzcomputerai v2.4.2`)

GUI nativa em Rust (`egui`/`eframe`, sem Chromium ou WebView), bilíngue **PT-BR / English** com alternância em tempo real. Organizada em **7 abas**:

| Aba | Função |
| :--- | :--- |
| **MCP & Rede** | Configuração da porta HTTP do servidor MCP (`CUA_DRIVER_RS_MCP_HTTP_PORT`), **publicação na rede local por relay TCP interno** (botões *Publicar na rede* / *Parar*, sem UAC e sem regra deixada no sistema), teste real do endpoint `/mcp` via TCP, URL de rede com IP LAN autodetectado, botão **Verificar Atualizações** (GitHub Releases com auto-installer), **Iniciar com o Windows** (autostart) e **Console Debug** deduplicado com rolagem automática. |
| **MCP Tools** | **[NOVO v2.0.0]** Catálogo visual completo para listar, filtrar por categoria e invocar interativamente qualquer ferramenta MCP do motor CUA. |
| **Túnel (Internet)** | **[NOVO v2.1.0]** Expõe o MCP HTTP local na internet (HTTPS público -> HTTP local) por **Cloudflare Tunnel** (quick + nomeado via login OAuth/token), **ngrok** e **SSH reverso** (servidor próprio ou localhost.run/serveo). Captura a URL pública, gera o snippet `mcpServers` e testa de verdade a URL pública com **sonda de exposição em 2 fases** (POST `initialize` sem e com Bearer; veredito **PROTEGIDO E FUNCIONAL** quando sem credencial é barrado e com Bearer responde). **[NOVO v2.2.0]** Motores `cua-driver` 0.16+ exigem **token Bearer** no `/mcp` (*fail-closed* sem ele): a aba avisa e o botão **"Gerar e ativar token do motor"** gera, grava e reinicia em um clique — o snippet já sai com `Authorization`. A **senha na URL** via porteiro local (`/s/<senha>/mcp`) segue como camada opcional, compatível com o Bearer (a senha vai no path, não no header). Ciclo de vida limpo: o túnel nunca sobrevive ao app. **[NOVO v2.4.0]** Cloudflare com **domínio próprio** (URL fixa) em passos guiados pela GUI — *Login*, *Verificar login*, *Criar túnel + apontar DNS* e *Iniciar túnel* —, e o porteiro passa a **injetar o `Authorization: Bearer`** ao falar com o motor, de modo que clientes que só aceitam uma URL funcionem sem header (se o cliente mandar o próprio `Authorization`, o dele vence). O teste de exposição roda em segundo plano: a interface não trava enquanto ele acontece. **Motores antigos (≤0.8.x) não têm autenticação própria — leia o aviso da aba.** |
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
# Depois disso, basta ABRIR o FzComputerAI: ele sobe o motor como processo
# filho (e o encerra ao fechar). Nao use `cua-driver autostart kick` - a
# tarefa agendada foi removida do fluxo do aplicativo na v2.3.0.
```

> **Atenção (verificado):** o motor escuta **somente em `127.0.0.1`** — setar a porta NÃO publica nada na LAN.
> Para outra máquina alcançar o endpoint, use **Publicar na rede** (o relay TCP do próprio app, sem UAC e sem regra no sistema) na aba *MCP & Rede*, ou a aba **Túnel**.
> Motores **0.16+** exigem ainda o header `Authorization: Bearer <token>` (token de `CUA_DRIVER_RS_MCP_HTTP_TOKEN`;
> a aba Túnel gera e grava por você) e **rejeitam requisições com `Origin` de navegador** (HTTP 403).

### Configurando o Cliente HTTP / Orquestrador:
- **Endpoint**: `http://<IP_DO_WINDOWS>:8000/mcp` (exige o **Publicar na rede** ativo na aba *MCP & Rede*)
- **Método**: `POST`
- **Headers**: `Content-Type: application/json` e, nos motores 0.16+, `Authorization: Bearer <token>`

---

## 🧪 Teste de fora da rede

Para provar que a URL pública funciona **de outra máquina, fora da sua rede**, o repositório traz o `scripts/remote-teste.py` — só biblioteca padrão do Python 3, nada para instalar:

```bash
python scripts/remote-teste.py <URL> [--token TOKEN] [--termo TEXTO]
```

Ele faz `initialize` e `tools/list`, abre uma **janela nova** de navegador na máquina remota (nunca sequestra uma janela já aberta), navega até `search.yahoo.com`, digita o termo (padrão: `Roger Luft`), descobre e clica no botão de pesquisa (*Search* / *Pesquisar* / *Buscar*) — ou envia `Enter` — e confere o resultado lendo a tela de volta.

Se a URL já leva a senha no caminho (`/s/<senha>/mcp`), o `--token` não é necessário: o porteiro injeta o `Authorization` ao falar com o motor.

---

## 📦 Instalação Rápida

### 🪟 Windows — Instalador (recomendado)

Baixe o **`fzcomputerai-setup-windows-x64.exe`** na [página de releases](https://github.com/RLuf/fzcomputerai/releases/latest) e execute.

O instalador (Inno Setup, bilíngue PT-BR / English) faz:

- **Instala a GUI** em `%LOCALAPPDATA%\Programs\FzComputerAI` — no caso padrão **não pede UAC**; no diálogo é possível optar por instalar para todos os usuários (aí sim eleva).
- **Cria atalho** no Menu Iniciar e, opcionalmente, na Área de Trabalho.
- **Opção "Iniciar o FzComputerAI com o Windows"** (autostart) — grava exatamente a mesma chave `HKCU\...\Run` usada pelo checkbox da aba *MCP & Rede*, de modo que GUI e instalador nunca se contradizem.
- **Componente "Instalar o motor `cua-driver`"** (**marcado por padrão**; requer internet) — executa o instalador **oficial** do projeto Cua como passo real da instalação, inclusive no modo silencioso (`/VERYSILENT`); só é pulado com `/SKIPENGINE` ou quando a versão fixada já está instalada.
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
| [Aba MCP & Rede](docs/uso-mcp-rede.md) | Ciclo de vida do motor, porta, publicação na rede local e leitura do diagnóstico |
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
