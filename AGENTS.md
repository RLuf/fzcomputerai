# Diretivas para Agentes de IA — FzComputerAI & CUA Driver

Este arquivo contém as convenções, regras de arquitetura e padrões de operação obrigatórios para qualquer Agente de IA que atue neste repositório.

---

## 🎯 Visão Geral do Projeto

**FzComputerAI** é um ecossistema nativo de Visão Computacional e Automação de Interface (UI) acessível via **Model Context Protocol (MCP)** e por uma **Interface Gráfica Nativa Compilável em Rust (`fzcomputerai`)**.

- **Motor Principal:** `cua-driver` (escrito em Rust, localizado em `cua/libs/cua-driver/rust`).
- **Interface Gráfica:** `fzcomputerai` (escrito em Rust com `egui 0.29.1` / `eframe 0.29.1`).
- **Protocolo de Comunicação:** MCP via Stdio local e HTTP TCP/IP (`CUA_DRIVER_RS_MCP_HTTP_PORT=8000`), com *bearer token* obrigatório em `CUA_DRIVER_RS_MCP_HTTP_TOKEN` — gerado e persistido pela própria GUI (ver 1.1).
- **HTTPS (v2.2.0):** terminação TLS **dentro da GUI** (`fzcomputerai/src/tls.rs`), listener em `<bind>:8443` encaminhando para `127.0.0.1:8000`. Certificado auto-assinado gerado no setup (`--tls-init`) ou no primeiro run; Let's Encrypt (HTTP-01) opcional; cert próprio opcional. Ver 1.2.
- **Patrocinadores Oficiais:** Webstorage Tecnologia (`www.webstorage.com.br`) e Imóvel Site (`www.imovelsite.com.br`).

---

## 📌 Padrões & Regras de Desenvolvimento

### 1. Modificações no Código Rust
- Todos os componentes nativos devem ser mantidos em **Rust 2021 edition**.
- A GUI `fzcomputerai` utiliza `egui` e `eframe` de modo imediato (Immediate Mode GUI), sem dependências pesadas de Chromium, WebView ou Node.js runtime.
- Não introduza dependências desnecessárias no `Cargo.toml`.

### 1.1. Convenções obrigatórias da GUI (`fzcomputerai/src/app.rs`)
- **Spawns de processos SEMPRE via `quiet_cmd(program)`** (helper em `app.rs`): no Windows ele aplica `creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) para não piscar janelas de console. Nunca use `std::process::Command::new` diretamente fora do helper.
- **Versão SEMPRE via `env!("CARGO_PKG_VERSION")`** (ou `concat!` com ela). Nunca hardcode o número de versão em strings da UI — a fonte da verdade é o `Cargo.toml`.
- **Todo handler de ação loga no Console Debug**: use `run_logged()` (comando, exit code, stdout, stderr, erros de spawn) ou `log_debug()` para eventos. O painel do Console Debug fica na aba MCP & Rede (limite de 64KB, mantém o final do log) e também espelha o log do motor lançado pela GUI (linhas com prefixo `[motor]`). No botão "Ir ao fim", o **salto explícito tem precedência** sobre a detecção de posição do scroll: era exatamente esse o bug — o clique setava `console_follow = true` e a detecção, no mesmo frame, sobrescrevia o valor com a posição antiga.
- **Status honesto**: estados como `port_active`/`daemon_running` devem refletir verificação real (ex.: teste TCP no endpoint `/mcp`), nunca valores presumidos. Na aba Túnel, o badge de exposição só fica verde após a **sonda real** (POST `initialize` na URL pública) — nunca por intenção.
- **Processos de longa duração (túneis: cloudflared/ngrok/ssh) — ciclo de vida obrigatório**: todo túnel iniciado pela GUI DEVE ser rastreado em `HKCU\Software\FzComputerAI` (`tunnel:<provider>:<pid>` com dado `imagem|CreationDate|porta|run_id|modo`), ter um **watchdog independente disparado no start** (mata o túnel se a GUI morrer, inclusive `kill -9`), ser derrubado no `shutdown_cleanup` e reconciliado na abertura (`startup_reconcile_tracked_tunnels`). **Matar processo de túnel SOMENTE com identidade de 3 fatores** (imagem + `CreationDate` + marcador `run_id` único na command line) — **é PROIBIDO `taskkill /IM cloudflared.exe|ngrok.exe|ssh.exe`**, que atingiria processos legítimos de outros usos do usuário. Segredos (token do Cloudflare, authtoken do ngrok) **nunca** em `argv`/log/registro — o `run_logged` loga a command line; use token-file/config nativa do CLI. Não embarque binários de túnel no instalador/release (download sob demanda com hash/Authenticode registrados).
- **Ciclo de vida do motor — a GUI é DONA do processo (NORMATIVO)**: `start_daemon` DEVE lançar `cua-driver serve` como **processo filho** da GUI, com porta e token injetados no ambiente e `stdout`+`stderr` redirecionados para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`, que o console segue em modo `tail -f` (prefixo `[motor]`). **NÃO volte a delegar para `cua-driver autostart kick`**: ali quem subia o motor era a Scheduled Task e o processo nascia filho do **Agendador** — (a) o `stdout` pertencia à task e os logs do motor sumiam, de modo que a atividade de clientes MCP externos (conector do Claude, Antigravity, Cursor) não aparecia no console; (b) a task herda o ambiente do **logon**, então token/porta gravados depois de logar não eram vistos e o daemon 0.16+ morria no ato, deixando a porta muda com a GUI dizendo apenas "PARADO"; (c) a GUI não controlava o ciclo de vida daquilo que gerencia. A Scheduled Task fica como **último recurso** e, quando for esse o caso, o console DEVE avisar que o processo não é da GUI e que não haverá logs.
- **"Iniciar" NÃO derruba um motor saudável**: antes de parar e subir outro, verifique o endpoint. No Windows, sockets da porta que já tiveram conexão ficam retidos em `TIME_WAIT` por minutos, e o `serve` novo não consegue o bind (`MCP HTTP transport disabled — bind 127.0.0.1:8000 failed (os error 10048)`), sobrando um daemon zumbi — pipe vivo, porta muda. Ou seja: clicar Iniciar **quebrava** o que estava funcionando. Com o endpoint respondendo, o botão não encosta no processo; forçar troca de processo é papel do **Reiniciar**.
- **Token do endpoint gerado pela GUI**: o motor 0.16+ chama o valor de *host-generated bearer token* — o host é o app. Sem token o `serve` sai com erro e nenhuma porta abre. A GUI gera 32 bytes do RNG do Windows (64 chars hex) e persiste em `HKCU\Environment` na primeira vez que precisa, sem exigir que o usuário saiba da variável. É a única exceção consciente à regra "segredo não vai para o registro": `HKCU\Environment` é a variável de ambiente do usuário, o mecanismo pelo qual o motor recebe o valor. Ainda assim, o valor **nunca** vai para o console/log; para inspecionar: `reg query HKCU\Environment /v CUA_DRIVER_RS_MCP_HTTP_TOKEN`.
- **Encaminhamento LAN é feito PELO PRÓPRIO APP, não por `netsh portproxy`**: uma thread do processo escuta em `<ip_lan>:porta` e copia bytes contra `127.0.0.1:porta` (`std::net::TcpListener` + `std::io::copy`, **sem dependência nova**); é TCP puro, então `curl`, `telnet` e `nc` atravessam igual. O `portproxy` é regra **estática** do serviço IP Helper: exigia admin/UAC para criar e remover, continuava aparecendo como `LISTENING` na LAN **mesmo com o motor morto** (aceitando conexões que morriam no destino — falso positivo de serviço no ar) e sobrevivia ao fechamento do app e ao reboot, obrigando rotina de limpeza. O `netsh` permanece **apenas como fallback**, quando o bind no IP da LAN falha. Não volte a tratá-lo como o caminho normal da LAN.
- **Limpeza ao fechar é Rust nativo, sem PowerShell e sem elevação**: a versão anterior disparava um `powershell -WindowStyle Hidden` de ~2 KB que esperava o processo morrer, matava processos, escrevia no registro, rodava `netsh` e chamava `-Verb RunAs` — o **Microsoft Defender flagrou essa linha de comando** (detecção 2147941383, 2026-08-03). Use chamadas diretas e curtas. E, pela mesma regra dos túneis, **é PROIBIDO `taskkill /F /IM cua-driver.exe`** (foi removido do app e do instalador): ele matava TODO processo com esse nome, inclusive um motor que o usuário estivesse rodando para outro fim. O caminho é `cua-driver stop` (comando oficial) e, se sobrar processo, kill **por PID** com o caminho do executável conferido.
- **Pacote de skills é instalado PELO SETUP**: `cua-driver skills install` roda no fim da instalação. Sem os symlinks, o agente conecta no MCP e **não enxerga ferramenta nenhuma**, e quem acabou de instalar não tem como saber que precisa clicar num botão da aba Doctor & Skills. O comando é idempotente e, pelo help oficial do motor, "Never overwrites existing user links" (verificado apagando os links e vendo o setup recriar os quatro: Claude Code, Codex, Antigravity, Hermes). O botão da aba continua existindo para reparo manual.
### 1.2. HTTPS do endpoint — convenções (NORMATIVO, v2.2.0)
- **A terminação TLS é do APP, não do motor.** O `cua-driver serve` é HTTP-only em `127.0.0.1` (fixo no código do Cua). Não tente "ligar HTTPS no motor" nem patchear o submódulo: o caminho é o listener de `tls.rs` (thread do processo, `rustls`, cópia de bytes para o loopback) — mesma doutrina do Encaminhamento LAN (sem admin, sem regra no sistema, cai com o app). O bearer token **continua obrigatório** atrás do TLS.
- **Certificado auto-assinado é cert de SERVIDOR TLS, e só isso.** Gerado por `rcgen` em `tls::ensure_self_signed` (SANs: localhost, 127.0.0.1, IP da LAN, hostname, domínio). É **PROIBIDO** instalá-lo (ou qualquer CA) em store de confiança da máquina, usá-lo para assinar binário, ou afirmar que ele "remove aviso" de navegador — isso é a seção 4.1 e ela continua valendo integralmente. O cliente confia **do lado dele**: `--cacert` com o `.crt` ou pin do SHA-256 mostrado na tela. Se encontrar código que instale o cert em store, remova e registre no CHANGELOG.
- **"Na instalação ou no primeiro run, o que vier primeiro"**: o instalador chama `fzcomputerai --tls-init` (também no upgrade silencioso) e a GUI repete `ensure_self_signed` no startup. Os dois são **idempotentes**: cert válido (> 30 dias) que já cobre os SANs **não é regenerado** — regenerar troca o fingerprint e quebra o pin de todo cliente. Regeneração explícita só pelo botão (que guarda `selfsigned.prev.*`).
- **Let's Encrypt = HTTP-01 na porta 80, e ponto.** `instant-acme`, conta persistida em `letsencrypt-account.json`, respondedor temporário em `0.0.0.0:80` só durante a emissão, renovação automática < 30 dias (checagem a cada 6 h em `poll_tls`). Não prometa emissão sem DNS público + porta 80 alcançável; não implemente DNS-01 "manual" que trave a UI esperando o usuário. Use *staging* nos testes (o limite de rate da produção é real).
- **Status honesto também aqui**: `TlsStatus::Listening` só após `tls::probe_https` (handshake real + `POST initialize` com JSON-RPC). "Listener de pé" sem JSON-RPC é `ListeningNoMcp` (amarelo) — nunca verde.
- **Segredos**: chave privada só em arquivo (`%APPDATA%\FzComputerAI\tls\`, 0600 no unix); nunca em registro, log, console ou argv. Preferências `tlscfg:*` guardam apenas caminhos/flags.
- **Crypto backend = ring** em todas as crates (`rustls`, `rcgen`, `instant-acme`). Não troque para `aws-lc-rs`: exige cmake+nasm no runner Windows do release.
- **Testes**: `cargo test --bin fzcomputerai tls::` roda o ponta a ponta com motor falso (geração, idempotência, proxy, 200/401 transparentes, stop). Mantenha-o verde ao mexer em `tls.rs`.

- **Nomenclatura dos Binários Compilados:** cópias locais para teste/distribuição manual devem incluir a versão no
  nome (ex.: `fzcomputerai-v2.0.0.exe`), mantendo a fonte de verdade em `Cargo.toml`. **EXCEÇÃO NORMATIVA — o
  instalador do release NÃO leva versão no nome:** o asset publicado é sempre `fzcomputerai-setup-windows-x64.exe`,
  porque o workflow de release (`INSTALLER_NAME` em `.github/workflows/build-release.yml`) e o
  `OutputBaseFilename` do `installer/fzcomputerai.iss` formam um contrato de nome fixo — a versão vem do tag do
  GitHub. Além disso, o fluxo de auto-upgrade da GUI (`check_for_updates`/`start_update_download` em
  `fzcomputerai/src/app.rs`) baixa o instalador pelo nome fixo; renomear o asset quebra o upgrade de TODAS as
  versões já instaladas. Quem quiser versão no nome do instalador precisa mudar os três lados juntos.

### 2. Comunicação MCP & Ferramentas de Visão
- As ferramentas de visão computacional expostas via MCP são:
  - `get_desktop_state`: Captura do desktop e lista de janelas.
  - `get_window_state`: Captura focada da janela e tokens de acessibilidade.
  - `take_screenshot`: Imagem base64 para modelos de visão multimodal.
  - `mouse_click`, `mouse_move`, `mouse_drag`, `keyboard_type`, `keyboard_press`, `shortcut`.
  - `start_recording`, `stop_recording`.

### 3. Preservação de Direitos & Atribuição
- O motor `cua-driver` é parte do projeto open-source `trycua/cua`, de **Cua AI, Inc.**, sob licença **MIT** (`Copyright (c) 2025 Cua AI, Inc.`).
- **Sempre preservar** a declaração de Copyright original, o texto integral da MIT do Cua, a citação formal (`@software{cua2025...}`) e o agradecimento no `README.md`, `README_EN.md`, `LICENSE.md` e `installer/LICENSE.txt`. A MIT **exige** que o aviso de copyright e a licença acompanhem cópias ou porções substanciais — nunca remova essas seções.
- As contribuições deste repositório e a GUI `fzcomputerai` estão sob licença **MIT** (`Copyright (c) 2026 Roger Luft (VeilWalker) — Webstorage Tecnologia`). A licença foi alterada de CC BY 4.0 para MIT na v2.1.0, para casar com a do projeto Cua e remover fricção de adoção (a própria Creative Commons não recomenda CC-BY para software). Fonte da verdade: `LICENSE.md`; os campos `license` de `fzcomputerai/Cargo.toml` e `package.json` devem acompanhar.

### 4. Assinatura de Código, SmartScreen e Segurança do Usuário Final (NORMATIVO)

> **Nota v2.2.0:** o certificado **auto-assinado de servidor TLS** do endpoint HTTPS (seção 1.2) **não** é
> exceção a nada abaixo: ele serve TLS e nunca é instalado em store de confiança nem usado para assinar
> artefato. O item 1 de 4.1 proíbe auto-assinado **para assinar artefatos distribuídos**; o item 2 proíbe
> instalar CA/cert em store. `tls.rs` não faz nenhum dos dois.

> **Fonte da verdade:** [`SIGNING.md`](SIGNING.md). Leia-o **antes** de tocar em qualquer coisa relacionada a
> assinatura, certificados, instalador ou avisos do Windows. Esta seção é o resumo vinculante; o documento traz o
> porquê, os custos e as opções reais.

**Contexto:** uma versão anterior do `install.ps1` (instalador de console que existia na raiz do repositório —
o arquivo **não existe mais**; a instalação no Windows hoje é exclusivamente pelo instalador gráfico Inno Setup)
gerava um certificado auto-assinado e o instalava na Raiz Confiável do usuário final. Isso foi **removido por
questão de segurança**. Esta seção existe para que a remoção não seja desfeita por um agente futuro agindo de
boa-fé.

#### 4.1. Proibições absolutas

Nenhuma das práticas abaixo pode ser implementada — **não em código, não no instalador, não em script auxiliar,
não em workflow de CI, não "temporariamente para testar"**:

1. **Gerar certificado auto-assinado para distribuição.** Proibido usar `New-SelfSignedCertificate`,
   `makecert`, `openssl req -x509` ou equivalente para assinar artefatos destinados ao usuário final. Não remove
   aviso nenhum e produz um binário falsamente "assinado".
2. **Instalar CA / certificado em store de confiança da máquina do usuário.** Proibido escrever em
   `Cert:\CurrentUser\Root`, `Cert:\LocalMachine\Root`, `TrustedPublisher`, no keychain System/login do macOS ou
   em `/usr/local/share/ca-certificates` (e afins). Isso altera a postura de segurança de um computador que não
   pertence a este projeto e é a técnica clássica de malware para legitimar binários arbitrários.
3. **Alterar de forma persistente a configuração de SmartScreen, Defender ou antivírus.** Proibido mexer em
   políticas/registro de SmartScreen, adicionar exclusões no Defender (`Add-MpPreference -ExclusionPath`),
   desativar proteção em tempo real ou remover a *Mark of the Web* de arquivos baixados em nome do usuário.
4. **Embutir chave privada de assinatura** em repositório, instalador, pacote npm ou artefato de release.
5. **Afirmar, em qualquer texto do projeto** (README, CHANGELOG, release notes, UI, comentários, mensagens de
   commit, resposta ao usuário), que assinatura, instalador, autoassinatura ou qualquer outro truque **"elimina",
   "remove" ou "evita" o aviso do SmartScreen/Defender**. Um instalador não assinado sofre **exatamente o mesmo
   bloqueio** que um `.exe` avulso. Mesmo com certificado OV legítimo, o aviso **pode persistir** até o
   certificado acumular reputação — ver `SIGNING.md` §5.

#### 4.2. O que é permitido

- Assinar artefatos **antes da publicação** com certificado de code signing de **CA pública**, em token USB ou
  HSM, via `scripts/sign-release.ps1` (obrigatório carimbo de tempo RFC 3161).
- Manter a assinatura **condicional** no CI, que só roda se os segredos existirem, e o `::warning` explícito
  quando não há certificado.
- Publicar e documentar os checksums `.sha256`.
- Documentar honestamente o aviso do SmartScreen e como o usuário prossegue.

#### 4.3. Obrigações ao alterar algo nessa área

- Se encontrar código que viole 4.1, **remova-o e registre a remoção no `CHANGELOG.md`** — não o deixe
  desabilitado "por precaução".
- Ao remover uma prática proibida, **deixe um comentário no arquivo** explicando o porquê, para impedir a
  reintrodução (é o que já existe em `installer/fzcomputerai.iss`,
  `.github/workflows/build-release.yml` e `scripts/sign-release.ps1`).
- **Não reescreva o histórico do `CHANGELOG.md`.** Uma promessa incorreta já publicada é corrigida com uma nota
  de correção na entrada original **mais** uma entrada nova, nunca apagando o registro.
- Datas e fatos regulatórios (token/HSM obrigatório desde **junho/2023**; validade máxima de 459 dias;
  elegibilidade do Azure Trusted Signing) vivem no `SIGNING.md`. Se precisar citá-los, **cite-os de lá**; se
  precisar atualizá-los, atualize `SIGNING.md` primeiro e cite a fonte. **Não afirme número, preço ou lista de
  países sem fonte verificável.**

---

### 5. 🗄️ REGRA DE OURO — `archived/` antes de qualquer alteração destrutiva (NORMATIVO)

**Neste projeto nada é apagado.** Antes de **modificar de forma destrutiva, sobrescrever, mover ou remover**
qualquer arquivo, faça **backup do arquivo em questão** (ou mova o próprio arquivo) para a pasta `archived/` na
raiz do repositório.

- A pasta `archived/` **está no `.gitignore`** e portanto **nunca entra no branch** — ela é histórico/lixo local,
  não artefato do repositório. Se ela não existir, **crie-a antes** de começar (`mkdir -p archived/`).
- Use um subdiretório com data e motivo, para o histórico ser legível:
  `archived/AAAA-MM-DD-<motivo>/` (ex.: `archived/2026-08-02-limpeza-repo/`).
- Ao **desrastrear** algo do git, o par correto é: copiar para `archived/`, depois
  `git rm --cached <arquivo>`, depois mover o arquivo para `archived/`. Nunca `git rm` direto (perde o conteúdo).
- Isso vale também para documentação: ao reescrever um `.md` por inteiro, arquive a versão anterior primeiro.
- **Exceção:** artefatos de build reproduzíveis (`target/`, `dist/`) não precisam de arquivamento — são gerados.

### 5.1. ⛔ NUNCA leia nem varra os exports de conversa (ARMADILHA REAL)

**Não faça `grep`/`Read`/busca recursiva em `.claude/`, `.claude-code-history/` ou
`archived/`.** São **exports de conversa de centenas de KB por arquivo**. Uma busca
recursiva na raiz cai dentro deles, devolve blocos enormes de JSON/markdown e
**estoura o contexto do agente** — na prática isso já custou **horas** para o Roger
tirar um agente de loop infinito. O histórico foi movido para
`archived/2026-08-02-limpeza-repo/claude-history/` justamente para sair do caminho.

Ao buscar no repositório, **sempre exclua esses diretórios** e prefira alvo explícito:

```bash
# BOM: lista de arquivos explícita
grep -rn "<termo>" README.md AGENTS.md CHANGELOG.md docs/ fzcomputerai/src/

# BOM: com exclusões
grep -rn "<termo>" . --exclude-dir=.claude --exclude-dir=.claude-code-history \
  --exclude-dir=archived --exclude-dir=cua --exclude-dir=target --exclude-dir=node_modules
```

O diretório `cua/` (submódulo do motor, repo inteiro de terceiro) e `target/`
também devem ser excluídos por volume — leia dentro deles apenas por caminho
direto, quando souber exatamente o arquivo.

### 6. 📚 Documentação e prints (NORMATIVO)

- **Toda alteração funcional obriga varredura de documentação.** Ao mudar comportamento, criar aba/recurso ou
  mexer em licença/instalação, revise e atualize **todos** os documentos relacionados: `README.md`,
  `README_EN.md`, `CHANGELOG.md`, `AGENTS.md`, `INSTALL.md`/`INSTALL_EN.md`, `SKILL.md`, `SIGNING.md`,
  `LICENSE.md` e a documentação em **`./docs/`**. Se a documentação necessária não existir, **crie-a**.
- **`./docs/` é o lugar da documentação técnica e de uso detalhada.** A raiz guarda o essencial (visão geral,
  instalação, licença, changelog); o aprofundamento (arquitetura, cada aba, túnel, atualização, solução de
  problemas) vive em `./docs/`.
- **A home (`README.md`) precisa vender a ferramenta**: descrição, lista de **features** e **prints reais** da
  interface. Ao alterar a UI, **capture prints novos** e atualize as imagens — print desatualizado é documentação
  errada. Os prints ficam em `assets/img/` (ex.: `assets/img/screenshot-<aba>.png`) e devem ser referenciados nos
  dois READMEs.
- **Print tem de ser real**, capturado do app compilado (o `cua-driver` pode capturar a janela). Nunca use mockup
  ou imagem de outra versão.

---

### 7. Continuous Integration (CI)
- O repositório possui um workflow de CI (`.github/workflows/ci.yml`) que executa build, testes e clippy (Windows e Linux) a cada `push` e `pull_request` para a branch `master`.

- O passo do `cargo clippy` roda com `--locked --all-targets`. No entanto, ele **não é bloqueante** (`continue-on-error: true`) no momento.

- Esta regra de clippy não-bloqueante foi estabelecida para permitir que PRs avancem até que todas as correções manuais de warnings sejam completadas em um PR dedicado. Nenhuma nova supressão genérica de alertas (`#![allow(dead_code)]`) deve ser introduzida para burlar a verificação; warnings restantes devem ser ajustados no código preservando lógica condicional (`cfg(windows)`).



---
## 🛠️ Comandos Úteis para Agentes

### Compilação da GUI Rust
```powershell
cargo build --release --manifest-path fzcomputerai/Cargo.toml
```

### Instalação e Teste
```powershell
# Windows: instalador gráfico (único caminho de instalação Windows) —
# baixar fzcomputerai-setup-windows-x64.exe em
# https://github.com/RLuf/fzcomputerai/releases/latest e executar.
# Build local do instalador (requer Inno Setup / ISCC.exe):
ISCC.exe /DAppVersion=<versao> installer\fzcomputerai.iss

# Linux/macOS: script de instalação
curl -fsSL https://github.com/RLuf/fzcomputerai/raw/master/install.sh | bash

# Via NPM Package Global
npm install -g fzcomputerai
```

### Diagnóstico de Saúde
```powershell
cua-driver doctor
# ou via npx
npx fzcomputerai doctor
```

