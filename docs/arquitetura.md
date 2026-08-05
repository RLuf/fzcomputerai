# Arquitetura

Para quem precisa entender como as peças se encaixam antes de mexer no código ou depurar um estado estranho na tela.

## 1. Duas peças, papéis distintos

| Peça | O que é | Quem publica | Papel |
| --- | --- | --- | --- |
| `fzcomputerai` | GUI nativa em Rust (egui/eframe 0.29.1), sem Chromium e sem WebView | este repositório (MIT, Roger Luft / Webstorage Tecnologia) | inicia, para, configura, diagnostica e **expõe** o motor |
| `cua-driver` | motor de automação de desktop (clique, teclado, tela, janelas, acessibilidade) | projeto [Cua](https://github.com/trycua/cua) — Cua AI, Inc. (MIT) | faz o trabalho de verdade |

A GUI **não implementa automação nenhuma**. Toda ação da interface termina em uma invocação de `cua-driver` como processo filho. Sem o motor instalado e no PATH, a janela abre, o console registra o erro de execução e nenhum botão produz efeito.

Desde a 2.3.0 o **daemon do motor também é processo filho da GUI** (`cua-driver serve`, dado o *spawn* pela própria janela quando nada responde na porta). Isso muda quem é dono do ciclo de vida — ver a seção 4.

Dependências declaradas em `fzcomputerai/Cargo.toml`: `eframe`, `egui`, `tokio`, `serde`, `serde_json`, `anyhow`, `open` (mais `winresource` como *build-dependency* apenas no Windows). **Não há cliente HTTP.** Requisições HTTP são escritas à mão sobre `std::net::TcpStream` (loopback, sem TLS) ou delegadas a `curl.exe` / PowerShell quando há TLS envolvido. O Job Object do Windows (`fzcomputerai/src/lifecycle.rs`) também é `extern "system"` escrito à mão, pelo mesmo motivo do `tray.rs`: três funções de `kernel32` não justificam uma dependência nova.

## 2. Transporte MCP

O motor expõe MCP (Model Context Protocol) por dois transportes:

| Transporte | Como se usa | Observação |
| --- | --- | --- |
| **stdio** | o cliente MCP lança `cua-driver` e conversa por entrada/saída padrão | não envolve rede; não aparece no `netstat` |
| **HTTP** | `POST /mcp`, corpo JSON-RPC 2.0 | **só sobe se `CUA_DRIVER_RS_MCP_HTTP_PORT` estiver definida** |

Detalhes que a GUI depende (verificados no motor instalado e no repositório upstream):

- Sem a variável `CUA_DRIVER_RS_MCP_HTTP_PORT`, **o listener HTTP nem é criado**. Não existe porta padrão implícita — a GUI usa 8000 apenas como valor inicial do campo.
- **`cua-driver mcp` não serve para o endpoint HTTP.** Medido no motor 0.17: esse subcomando é o transporte **stdio** e **morre quando o stdin fecha** — como filho de uma GUI, ele cairia sozinho. O modo que a GUI usa é `cua-driver serve`, que sobe o HTTP (com a variável de porta no ambiente) e abre também o pipe `\\.\pipe\cua-driver`, o mesmo canal que o próprio CLI usa em `call`, `status` e `stop`.
- **O endereço de escuta não é configurável.** O motor oficial escuta somente em `127.0.0.1`; o endereço está fixo no código do Cua (`([127,0,0,1], port)`). A string `CUA_DRIVER_RS_MCP_HTTP_BIND` **não existe** no binário oficial instalado (0.8.3) e a busca por ela no repositório `trycua/cua` retorna zero resultado.
- Uma versão anterior desta documentação afirmava haver bind `0.0.0.0`. **Era falso e foi corrigido.** Se alguém quiser reintroduzir a ideia, o comentário em `apply_env_port()` (`fzcomputerai/src/app.rs`) explica por quê não: gravar aquela variável não publica nada, o motor a ignora. A GUI hoje até **remove** a variável se encontrar sobra dela em `HKCU\Environment`, para não confundir o diagnóstico.
- **Autenticação depende da versão do motor.** A série `0.16+` **exige** `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32 a 4096 caracteres, sem espaço nem caractere de controle) e responde **401** a qualquer POST sem `Authorization: Bearer <token>`; ela também rejeita requisições com origem de navegador. **Sem nenhum token configurado, o endpoint dessa série é fail-closed**: 401 para tudo — não "aberto". Versões antigas (<= 0.8.x; o instalador hoje exige 0.16.0 como MÍNIMO) **não têm token nenhum**. A GUI lê o token de `HKCU\Environment` na abertura **e ao abrir a aba Túnel** (`read_mcp_token()` — o **valor** nunca vai para o log) e envia o header em todo probe **somente quando há token configurado** — assim o mesmo teste funciona com as duas gerações. O token pode ser gerado e ativado pela própria GUI, na aba Túnel (ver [uso-tunel.md](uso-tunel.md)).

Consequência direta: **sair do loopback nunca é questão de configuração do motor**. É encaminhamento (LAN) ou túnel (internet). Ver [acesso-remoto.md](acesso-remoto.md).

## 3. Diagrama

```
                        JANELA fzcomputerai.exe (Rust / egui, 1 processo)
   ┌───────────────────────────────────────────────────────────────────────────────┐
   │  BARRA LATERAL           PAINEL CENTRAL (seção ativa)                         │
   │  MCP & Rede         ┌──────────────────────────────────────────────────────┐  │
   │  Túnel              │  tabs/network.rs  tabs/tunnel.rs  tabs/mcp_tools.rs  │  │
   │  MCP Tools          │  tabs/calibration.rs  tabs/windows.rs                │  │
   │  Calibração         │  tabs/recording.rs  tabs/doctor_skills.rs            │  │
   │  Janelas            └──────────────────────┬───────────────────────────────┘  │
   │  Gravação                                  │ &mut AppState                    │
   │  Doctor & Skills                           v                                  │
   │  ─────────────      ┌──────────────────────────────────────────────────────┐  │
   │  ponto MCP          │  AppState  (app.rs) — TODO o estado, em memória      │  │
   │  chip TÚNEL         │  quiet_cmd() -> run_logged() -> log_debug()          │  │
   │  idioma / Sobre     │  spawn_bg() -> thread -> BgOutcome -> poll_bg()      │  │
   │                     │  THREADS DESTE PROCESSO: relay LAN, porteiro /s/     │  │
   │                     └──────┬───────────────────────┬───────────────────────┘  │
   │  CONSOLE GLOBAL (rodapé, visível em todas as seções, comportamento tail -f)   │
   └────────────────────────────┼───────────────────────┼──────────────────────────┘
                                │                       │
       JOB OBJECT               │                       │   sondas de rede próprias
       (KILL_ON_JOB_CLOSE)      v                       v
   ┌────────────────────────────────────┐   ┌──────────────────────────────────────┐
   │ FILHOS ADOTADOS (morrem com a GUI):│   │ TcpStream  POST /mcp  {initialize}   │
   │   cua-driver serve   (o motor)     │   │ netstat -ano -p tcp                  │
   │   cloudflared | ngrok | ssh        │   │ netsh interface portproxy show       │
   │ chamadas curtas (CREATE_NO_WINDOW):│   │   (só para achar regra LEGADA)       │
   │   cua-driver call/doctor/skills/   │   │ curl.exe  (única via com TLS)        │
   │   check-update/update, reg, netsh  │   └──────────────────────────────────────┘
   └───────────────┬────────────────────┘
                   │
                   v
   ┌──────────────────────────────────────────────────────────────────────────────┐
   │ MOTOR cua-driver — MCP HTTP em 127.0.0.1:<porta>  (endereço FIXO no código)  │
   └──────────┬───────────────────────────────┬───────────────────────────────────┘
              │                               │
   relay TCP na própria GUI            túnel de saída                 (nada mais
   0.0.0.0:porta -> 127.0.0.1:porta    cloudflared/ngrok/ssh -R        alcança)
              │                               │
              v                               v
        outra máquina da LAN            URL HTTPS pública na internet
                                        (opcional: porteiro de senha local
                                         127.0.0.1:efêmera, exige /s/<senha>/
                                         e injeta o Bearer do motor)
```

## 4. Ciclo de vida dos processos filhos (Job Object)

Desde a 2.3.0 o motor **não** é mais iniciado por tarefa agendada: todas as chamadas a `cua-driver autostart kick` saíram dos caminhos executáveis (inclusive do botão **Reiniciar**, do fluxo de token e dos scripts de instalação/atualização do motor). A GUI dá `spawn` em `cua-driver serve` e passa a ser **dona** desse processo.

Isso resolve um problema real do Windows: **um filho não morre com o pai.** `CreateProcess` não cria esse vínculo — isso é comportamento de Unix. A versão anterior compensava com um vigia em PowerShell e com `taskkill /F /IM cua-driver.exe`, o que matava motor de qualquer origem, inclusive o daemon que um cliente MCP de terceiro tivesse subido (proibição explícita do `AGENTS.md`).

`fzcomputerai/src/lifecycle.rs` faz isso pelo **kernel**:

- `main.rs` chama `lifecycle::init()` **antes de qualquer spawn**: `CreateJobObjectW` + `SetInformationJobObject` com `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. O handle do job fica aberto pela vida inteira do processo e **nunca** é fechado à mão — é o fechamento dele, na morte da GUI, que dispara a matança;
- todo filho de longa duração (o motor e o processo do túnel) é adotado com `lifecycle::adopt(&child)`. A adoção devolve `Ok` ou o código de erro do Windows, e o resultado **vai para o console**: sem adoção não há garantia de limpeza, e a GUI diz isso em vez de presumir;
- fora do Windows tudo em `lifecycle.rs` é *no-op* honesto (`adopt` devolve `Err`), e o encerramento fica com o `kill` direto no `Child` que o `shutdown_cleanup()` já faz.

O que isso passa a garantir, **medido**: com GUI + motor filho + porteiro de pé, um `taskkill /F` na GUI derrubou motor e porteiro juntos, e o `cua-driver` de **outro** cliente MCP ficou intacto; em outro teste o `cloudflared` também caiu junto com a janela. Vale para qualquer forma de sair: X, **Sair** na bandeja, `taskkill /F`, crash, logoff.

Duas consequências de projeto:

- **Ao abrir**, se nada responde na porta, o app sobe o motor sozinho como filho (`engine_child`, `engine_pid`). Se **já houver** motor de terceiro respondendo, ele é detectado (`engine_external`), **não** é duplicado nem morto, e a UI avisa que aquele motor não será encerrado ao fechar o app;
- **do fechamento foram removidos** `taskkill /F /IM cua-driver.exe` e `cua-driver stop`. Parar motor alheio virou ação **explícita** do usuário, no botão **Parar**.

O watchdog PowerShell do túnel continua no código, mas rebaixado a **fallback**: só é disparado quando a adoção no Job Object falha.

## 5. Publicar na LAN: relay no lugar do `netsh portproxy`

O caminho padrão para expor o MCP na rede local passou a ser um **relay TCP dentro do processo da GUI** (`start_lan_relay()` / `stop_lan_relay()` / `poll_lan_relay()` em `app.rs`, com `relay_handle_conn()` copiando bytes):

- escuta em `0.0.0.0:<porta>` — ou num IP escolhido, campo **Escutar em** (`lan_relay_bind`) — e encaminha para `127.0.0.1:<porta do motor confirmada>`;
- é **transparente**: copia bytes nos dois sentidos sem inspecionar nem reescrever HTTP, então keep-alive e streaming SSE passam intactos;
- **não pede UAC**, **não deixa regra no sistema** (a do `netsh` sobrevive a reboot) e **morre com o app**, porque é thread deste processo.

A medição que viabiliza o desenho: nesta plataforma `0.0.0.0:8000` **coexiste** com o `127.0.0.1:8000` do motor (o bind mais específico atende o loopback), então dá para publicar na **mesma** porta sem tocar na configuração do motor.

Na UI: badge de dois estados — **PUBLICADO NA REDE** / **SÓ LOCAL** —, contador real de conexões (ativas / total desde o início, lido dos `AtomicUsize` que as próprias threads incrementam) e os botões **Publicar na rede** / **Parar**. A remoção de regra `portproxy` **legada** continua disponível e só aparece na tela quando existe alguma.

Testado pela LAN, via `http://192.168.0.101:8000/mcp`: `initialize` OK, `tools/list` com 55 ferramentas e `tools/call get_screen_size` executando de verdade (4096x2160 @ 1.75x).

## 6. Fluxo de uma ação, do clique ao console

Exemplo: o usuário clica em **Iniciar** na aba MCP & Rede.

1. `tabs/network.rs` desenha o botão e, no `clicked()`, chama `state.start_daemon()`. A camada de UI **não** executa processo nenhum — ela só chama métodos de `AppState`.
2. `AppState::start_daemon()` (`fzcomputerai/src/app.rs`) confere se já há filho vivo, confere se há motor externo respondendo e, só então, dá `spawn` em `cua-driver serve` com `CUA_DRIVER_RS_MCP_HTTP_PORT` (e o token, quando há) no ambiente do filho — adotando-o no Job Object logo em seguida.
3. Nas ações que são chamada curta de CLI (`reg`, `netsh`, `cua-driver call`...), `run_logged()` monta o comando com `quiet_cmd()` — que no Windows aplica `CREATE_NO_WINDOW`, para nenhuma janela preta piscar na tela — executa com `output()` e registra no log: a linha de comando, o `exit code`, o `stdout` e o `stderr`, sempre com o resultado real.
4. `log_debug()` anexa a entrada em `AppState::debug_log`, um `String` limitado a 64 KB (o excesso é cortado pelo início, em fronteira de caractere).
5. A espera pelo listener do motor (o `serve` leva 1-3 s) vai para o **executor de segundo plano**; quando ela termina, `check_port_status()` refaz o teste real do endpoint e recalcula os badges. `daemon_running` recebe o resultado do teste — **nunca** "eu mandei iniciar, então está ligado".
6. No próximo frame, o painel do console no rodapé desenha `debug_log`; a faixa amarela acima dele mostra a primeira linha de `status_msg` (a última mensagem relevante).

### 6.1 O executor de segundo plano (2.4.0)

Até a 2.3.1 **toda** ação terminava em `Command::output()` **síncrono na thread da UI**. Medido: um `reg query` custa ~200 ms, um `powershell -Command` de 300 ms a 2 s, e o teste de exposição do túnel dispara **dois** `curl -m 20` — até 40 s de janela congelada, com o Windows escrevendo "(Não Respondendo)" no título. Não era travamento aleatório: era a thread do egui esperando processo externo.

A 2.4.0 acrescenta um executor mínimo, no mesmo espírito do que o projeto já fazia com downloads (spawn + poll):

- `spawn_bg(label, f)` roda `f` numa thread. A thread **não** recebe `&mut AppState`: ela devolve um `BgOutcome { log, status, effect }`, empilhado num `Mutex<Vec<_>>`;
- `poll_bg()` roda na thread da UI, drena a fila e aplica: escreve no console, atualiza a faixa de status e executa o `BgEffect` — hoje `Exposure(TunnelExposure)` (resultado da sonda do túnel) e `PortStatus { port_active, port_status, probe_401, real_listeners }` (estado da porta recalculado fora da UI). `BgEffect::None` cobre a tarefa que só tem o que registrar;
- `bg_busy` (um `AtomicUsize`) conta tarefas em voo, e a UI mostra isso em vez de fingir que terminou. Enquanto há tarefa pendente, o `update()` pede `request_repaint` a cada 200 ms.

Migrados para esse caminho na 2.4.0: o teste de exposição do túnel (o pior caso) e a espera pelo motor depois do start (que eram até 12x400 ms de `sleep` na thread da UI).

As ações **muito** longas continuam como antes, em processo destacado: download do instalador, `cua-driver update --apply`, download de `cloudflared`/`ngrok` e o processo do túnel são disparados com `spawn()`, e a GUI observa o resultado por **arquivos de flag** em `%TEMP%` ou pelo log do próprio CLI, com *throttle* de 1 s dentro de cada `poll_*`. O `update()` do eframe chama `request_repaint_after(1s)` enquanto houver algo pendente, para os polls acontecerem mesmo sem input do usuário.

## 7. Onde vive o estado

Tudo em `AppState` (`fzcomputerai/src/app.rs`), uma struct única passada como `&mut` para cada aba. Não há gerenciador de estado, canal, nem `Arc<Mutex<...>>` global. Blocos principais:

| Bloco | Campos representativos |
| --- | --- |
| idioma e navegação | `language`, `active_tab` |
| endpoint MCP | `http_port`, `lan_ip`, `port_active`, `port_status`, `mcp_token`, `mcp_probe_401` (o último probe respondeu 401) |
| motor como filho | `engine_pid`, `engine_external` (o motor que responde **não** é nosso) — e o `engine_child` privado, que é o handle de verdade |
| relay LAN | `lan_relay_bind`, `lan_relay_listen` (porta em que o relay **realmente** escuta), `lan_relay_target`, `lan_relay_conns`, `lan_relay_total` |
| regra `portproxy` legada | `portproxy_active`, `portproxy_effective`, `real_listeners`, `portproxy_rules` |
| segundo plano | `bg_busy` (tarefas em voo) — e a fila privada `bg_out` de `BgOutcome` |
| saída unificada | `status_msg` (última mensagem), `debug_log` (histórico), `console_follow` |
| atualização | `update_available`, `update_downloading`, `update_ready`, `driver_version`, `driver_latest`, `driver_update_available` |
| túnel | `tunnel_provider`, `tunnel_run_provider` (provedor **congelado** no start), `tunnel_status`, `tunnel_pid`, `tunnel_public_url`, `tunnel_exposure`, `tunnel_gate_password`, `tunnel_gate_port`, `tunnel_run_id` |
| Cloudflare nomeado | `tunnel_cf_bin`, `tunnel_cf_name`, `tunnel_cf_hostname`, `tunnel_cf_logged` (existe `~/.cloudflared/cert.pem`?), `tunnel_cf_token_file` (**caminho**), `tunnel_cf_token_input` (campo mascarado, nunca persistido nem logado) |
| privado (só `app.rs` mexe) | `tunnel_child`, `engine_child`, `tunnel_gate_stop`, `lan_relay_stop`, os `Instant` de *throttle* |

Nota sobre `tunnel_run_provider`: ele existe porque trocar o rádio de provedor com um túnel vivo fazia parada, identidade (`tunnel:<provedor>:<pid>`), limpeza de `HKCU`, log e extração de URL usarem o provedor **selecionado** em vez do **em execução** — parar um Cloudflare com ngrok marcado tentava apagar `tunnel:ngrok:<pid>`. Desde a 2.2.0 o provedor é congelado no start e usado em todo o ciclo de vida do túnel.

O estado de UI é **efêmero por definição**: fechar o app zera tudo o que não estiver no registro.

## 8. Persistência real

Não existe arquivo de configuração do FzComputerAI, e o *storage* do eframe não é usado. O que persiste está no registro do Windows, gravado por `reg.exe` / PowerShell e sempre **relido para confirmar**:

| Chave / valor | Conteúdo | Quem escreve |
| --- | --- | --- |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_PORT` | porta do endpoint HTTP do motor | botão **Aplicar Porta** (`set_user_env_confirmed`) |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_TOKEN` | token do endpoint (motor `0.16+`) | botão **Gerar e ativar token do motor** (aba Túnel), via `set_user_env_secret` — gravação com releitura de confirmação e **sem log do valor**; o token vem de `gen_secure_token` (CSPRNG via PowerShell/`RNGCryptoServiceProvider`, >= 32 caracteres) |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_BIND` | — | **é apagado** se existir: o motor oficial a ignora |
| `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` -> `FzComputerAI` | caminho do executável entre aspas | checkbox **Iniciar com Windows** e a task `autostart` do instalador (mesmo nome e mesmo formato, de propósito) |
| `HKCU\Software\FzComputerAI` -> `portproxy:<ip>:<porta>` | porta de destino da regra; marca a regra como **propriedade deste app**. **Legado**: o caminho padrão da LAN é o relay, que não persiste nada — isto só existe para máquinas que usaram versões anteriores | `apply_portproxy()` |
| `HKCU\Software\FzComputerAI` -> `tunnel:<provedor>:<pid>` | `imagem\|CreationDate\|porta\|run_id\|modo` — identidade forte do processo do túnel | `register_tunnel()` |
| `HKCU\Software\FzComputerAI` -> `tunnelcfg:*` | preferências da aba Túnel (provedor, **caminho** do token-file, nome do túnel e hostname do Cloudflare nomeado, URL pública, alvo SSH...) | botão **Salvar configuração** |

Regras de ouro da persistência, visíveis no código:

- **Segredo nunca vai para o log.** O token do Cloudflare é gravado em arquivo com ACL restrita (`icacls /inheritance:r /grant:r <usuário>:R`) e só o **caminho** é persistido no registro. O token do motor é a exceção de registro **por exigência do próprio motor** (ele só lê `HKCU\Environment`) — e mesmo assim o **valor** nunca aparece no console: leitura (`read_mcp_token`) e gravação (`set_user_env_secret`) suprimem o stdout do `reg`. A senha do porteiro do túnel existe apenas em memória, por sessão, e é mascarada como `/s/***/` em qualquer texto que vá para o console.
- **Só removemos o que registramos.** A limpeza de regras `portproxy` percorre os valores `portproxy:*` desta chave. Nesta mesma máquina existem regras LAN->loopback de outros serviços; elas não são tocadas. Vale o mesmo para túneis: `taskkill /IM` é **proibido**, porque mataria um `cloudflared`/`ngrok`/`ssh` legítimo do usuário.

## 9. O princípio de status honesto

Nenhum estado exibido é presumido a partir da intenção. Concretamente:

| Estado | Como é provado |
| --- | --- |
| MCP responde | `POST /mcp` com um `initialize` JSON-RPC real, com `Authorization: Bearer` quando há token configurado; um **401** é registrado em `mcp_probe_401` (motor vivo exigindo token — não "parado"). **GET não serve como prova**: o endpoint MCP responde legitimamente `405 Method Not Allowed` a GET, o que provaria apenas o TCP. |
| listener existe | `netstat -ano -p tcp` — a fonte de verdade do sistema operacional. As linhas cruas vão para a tela, com as mesmas colunas do terminal. |
| endpoint alcançável na LAN | badge verde **só** com listener confirmado no `netstat` **e** POST respondendo no IP da LAN. Se um dos dois falta, o console diz qual e a cor não fica verde. |
| relay publicado | dois estados, ambos verificáveis pelo próprio processo: **PUBLICADO NA REDE** (o socket está escutando, e `lan_relay_listen` guarda a porta real) ou **SÓ LOCAL**. O uso é mostrado com o contador de conexões que as threads do relay incrementam — não com "a regra existe, deve estar funcionando". O relay **não** sobe apontando para porta morta: sem motor confirmado em `127.0.0.1`, nada é publicado. |
| regra de encaminhamento (legado `netsh`) | 3 estados: **REGRA FUNCIONANDO** (existe no `netsh` e o listener está de pé), **REGRA SEM EFEITO** (existe na config, listener ausente), **SEM REGRA**. Aquele limbo do meio dependia do serviço IP Helper — é justamente o que o relay não tem. |
| motor de quem? | `engine_child` vivo = o motor é **nosso filho** e cai junto com a GUI; `engine_external` = alguém já respondia na porta antes de nós, e a UI diz que **não** vamos encerrá-lo ao fechar. A adoção no Job Object também é registrada com o resultado real (`Ok` ou o código de erro do Windows). |
| túnel ativo | `Starting` = processo vivo, URL ainda não capturada; `Running` = URL pública capturada ou informada. "Confirmado pela internet" é um estado **separado** (`tunnel_exposure`, enum `Exposed` / `EngineAuth` / `EdgeAuth` / `AuthOk` / `Unknown`), provado por uma **sonda em 2 fases** (`tunnel_probe_once`) na URL pública: fase 1 **sem** credencial — 200 + `"result"` ⇒ `Exposed`; 401 com corpo JSON-RPC ⇒ `EngineAuth`; 401/403/302/407 sem JSON-RPC ⇒ `EdgeAuth`; resto ⇒ `Unknown` — e fase 2 **com** `Authorization: Bearer`, só quando a GUI conhece o token e a fase 1 não expôs — 200 + `"result"` ⇒ `AuthOk`, a prova ponta a ponta. Antes da 2.2.0, qualquer resposta com `"jsonrpc"` era marcada como exposta — o 401 do motor `0.16+` também contém `"jsonrpc"`, o que gerava alarme falso em túnel protegido. URL (que pode conter a senha do gate) e Bearer viajam no arquivo `--config` do `curl`, nunca no argv. |
| versão do motor | `cua-driver check-update --json` — a API oficial do próprio motor. |
| integridade do instalador baixado | `Get-FileHash -Algorithm SHA256` conferido contra o `.sha256` publicado pelo CI. Divergência apaga o arquivo. |

As cores semânticas (amarelo/vermelho) sobrevivem ao tema monocromático de propósito: elas carregam informação de segurança ("EXPOSTO SEM AUTENTICAÇÃO", "REGRA SEM EFEITO") que se perderia num tema só-verde.

## 10. Interface: o que a arquitetura impõe

- **7 seções** na barra lateral, na ordem do código: MCP & Rede, Túnel, MCP Tools, Calibração, Janelas, Gravação, Doctor & Skills.
- **Um único console global** no rodapé, visível em todas as seções. Antes cada aba tinha sua própria caixa de saída, o que duplicava a mesma informação na mesma tela. Ele se comporta como `tail -f`: acompanha o fim sozinho e **pausa** quando o usuário rola para cima, com indicador "seguindo"/"pausado" e botão **Ir ao fim**.
- **A janela não espera processo externo.** Ação que chame CLI de duração imprevisível (sonda de túnel, espera pelo motor, criação de túnel nomeado) vai para `spawn_bg` e volta pelo `poll_bg`. É o que impede o "(Não Respondendo)" no título — ver 6.1.
- **Tema terminal**: fundo preto, texto verde, tudo monoespaçado.
- **Bilíngue PT-BR / EN**, com troca em tempo real via `match state.language` — não há arquivo de tradução nem recarga.
- **Sem emoji e sem glifos ausentes.** A fonte padrão do egui não tem `→`, `●` nem emoji: eles renderizariam caixas vazias. Usa-se `->` em texto e um ponto **desenhado** pelo painter (`status_dot`) para os badges.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — a aba que expõe todo esse diagnóstico na prática.
- [acesso-remoto.md](acesso-remoto.md) — por que o loopback é o limite do motor e quais são as saídas reais.
- [desenvolvimento.md](desenvolvimento.md) — as convenções obrigatórias que mantêm essa arquitetura de pé.
