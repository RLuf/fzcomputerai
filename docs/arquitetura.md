# Arquitetura

Para quem precisa entender como as peças se encaixam antes de mexer no código ou depurar um estado estranho na tela.

## 1. Duas peças, papéis distintos

| Peça | O que é | Quem publica | Papel |
| --- | --- | --- | --- |
| `fzcomputerai` | GUI nativa em Rust (egui/eframe 0.29.1), sem Chromium e sem WebView | este repositório (MIT, Roger Luft / Webstorage Tecnologia) | inicia, para, configura, diagnostica e **expõe** o motor |
| `cua-driver` | motor de automação de desktop (clique, teclado, tela, janelas, acessibilidade) | projeto [Cua](https://github.com/trycua/cua) — Cua AI, Inc. (MIT) | faz o trabalho de verdade |

A GUI **não implementa automação nenhuma**. Toda ação da interface termina em uma invocação de `cua-driver` como processo filho. Sem o motor instalado e no PATH, a janela abre, o console registra o erro de execução e nenhum botão produz efeito.

Dependências declaradas em `fzcomputerai/Cargo.toml`: `eframe`, `egui`, `tokio`, `serde`, `serde_json`, `anyhow`, `open` (mais `winresource` como *build-dependency* apenas no Windows). **Não há cliente HTTP.** Requisições HTTP são escritas à mão sobre `std::net::TcpStream` (loopback, sem TLS) ou delegadas a `curl.exe` / PowerShell quando há TLS envolvido.

## 2. Transporte MCP

O motor expõe MCP (Model Context Protocol) por dois transportes:

| Transporte | Como se usa | Observação |
| --- | --- | --- |
| **stdio** | o cliente MCP lança `cua-driver` e conversa por entrada/saída padrão | não envolve rede; não aparece no `netstat` |
| **HTTP** | `POST /mcp`, corpo JSON-RPC 2.0 | **só sobe se `CUA_DRIVER_RS_MCP_HTTP_PORT` estiver definida** |

Detalhes que a GUI depende (verificados no motor instalado e no repositório upstream):

- Sem a variável `CUA_DRIVER_RS_MCP_HTTP_PORT`, **o listener HTTP nem é criado**. Não existe porta padrão implícita — a GUI usa 8000 apenas como valor inicial do campo.
- **O endereço de escuta não é configurável.** O motor oficial escuta somente em `127.0.0.1`; o endereço está fixo no código do Cua (`([127,0,0,1], port)`). A string `CUA_DRIVER_RS_MCP_HTTP_BIND` **não existe** no binário oficial instalado e a busca por ela no repositório `trycua/cua` retorna zero resultado.
- Uma versão anterior desta documentação afirmava haver bind `0.0.0.0`. **Era falso e foi corrigido.** Se alguém quiser reintroduzir a ideia, o comentário em `apply_env_port()` (`fzcomputerai/src/app.rs`) explica por quê não: gravar aquela variável não publica nada, o motor a ignora. A GUI hoje até **remove** a variável se encontrar sobra dela em `HKCU\Environment`, para não confundir o diagnóstico.
- **Autenticação depende da versão do motor.** A série `0.16+` **exige** `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32 a 4096 caracteres, sem espaço nem caractere de controle) e responde **401** a qualquer POST sem `Authorization: Bearer <token>`; ela também rejeita requisições com origem de navegador. Versões antigas (série `0.8.x`) **não têm token nenhum** — o instalador não pina versão: o passo do motor executa o instalador oficial do Cua, que resolve a última versão estável publicada. A GUI lê o token de `HKCU\Environment` na abertura (`read_mcp_token()`) e envia o header **somente quando há token configurado** — assim o mesmo teste funciona com as duas gerações.

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
   │  idioma / Sobre     └──────┬───────────────────────┬───────────────────────┘  │
   │                            │                       │                          │
   │  CONSOLE GLOBAL (rodapé, visível em todas as seções, comportamento tail -f)   │
   └────────────────────────────┼───────────────────────┼──────────────────────────┘
                                │                       │
       processos filhos         │                       │   sondas de rede próprias
       (CREATE_NO_WINDOW)       v                       v
   ┌────────────────────────────────────┐   ┌──────────────────────────────────────┐
   │ cua-driver  (autostart kick/stop/  │   │ TcpStream  POST /mcp  {initialize}   │
   │             call/doctor/skills/    │   │ netstat -ano -p tcp                  │
   │             check-update/update)   │   │ netsh interface portproxy show       │
   │ netsh / reg / powershell / taskkill│   │ curl.exe  (única via com TLS)        │
   │ cloudflared | ngrok | ssh          │   └──────────────────────────────────────┘
   └───────────────┬────────────────────┘
                   │
                   v
   ┌──────────────────────────────────────────────────────────────────────────────┐
   │ MOTOR cua-driver — MCP HTTP em 127.0.0.1:<porta>  (endereço FIXO no código)  │
   └──────────┬───────────────────────────────┬───────────────────────────────────┘
              │                               │
   netsh portproxy                     túnel de saída                 (nada mais
   <IP_LAN>:porta -> 127.0.0.1:porta   cloudflared/ngrok/ssh -R        alcança)
              │                               │
              v                               v
        outra máquina da LAN            URL HTTPS pública na internet
                                        (opcional: porteiro de senha local
                                         127.0.0.1:efêmera, exige /s/<senha>/)
```

## 4. Fluxo de uma ação, do clique ao console

Exemplo: o usuário clica em **Iniciar** na aba MCP & Rede.

1. `tabs/network.rs` desenha o botão e, no `clicked()`, chama `state.start_daemon()`. A camada de UI **não** executa processo nenhum — ela só chama métodos de `AppState`.
2. `AppState::start_daemon()` (`fzcomputerai/src/app.rs`) chama `run_logged("cua-driver", &["autostart", "kick"])`.
3. `run_logged()` monta o comando com `quiet_cmd()` — que no Windows aplica `CREATE_NO_WINDOW`, para nenhuma janela preta piscar na tela — executa com `output()` e registra no log: a linha de comando, o `exit code`, o `stdout` e o `stderr`, sempre com o resultado real.
4. `log_debug()` anexa a entrada em `AppState::debug_log`, um `String` limitado a 64 KB (o excesso é cortado pelo início, em fronteira de caractere).
5. Ainda em `start_daemon()`, `check_port_status()` refaz o teste real do endpoint e recalcula os badges. `daemon_running` recebe o resultado do teste — **nunca** "eu mandei iniciar, então está ligado".
6. No próximo frame, o painel do console no rodapé desenha `debug_log`; a faixa amarela acima dele mostra a primeira linha de `status_msg` (a última mensagem relevante).

Ações longas fogem desse caminho síncrono para não travar a UI: download do instalador, `cua-driver update --apply`, download de `cloudflared`/`ngrok` e o processo do túnel são disparados com `spawn()` em processo destacado, e a GUI observa o resultado por **arquivos de flag** em `%TEMP%` ou pelo log do próprio CLI, com *throttle* de 1 s dentro de cada `poll_*`. O `update()` do eframe chama `request_repaint_after(1s)` enquanto houver algo pendente, para os polls acontecerem mesmo sem input do usuário.

## 5. Onde vive o estado

Tudo em `AppState` (`fzcomputerai/src/app.rs`), uma struct única passada como `&mut` para cada aba. Não há gerenciador de estado, canal, nem `Arc<Mutex<...>>` global. Blocos principais:

| Bloco | Campos representativos |
| --- | --- |
| idioma e navegação | `language`, `active_tab` |
| endpoint MCP | `http_port`, `lan_ip`, `port_active`, `port_status`, `mcp_token` |
| encaminhamento LAN | `portproxy_active`, `portproxy_effective`, `real_listeners`, `portproxy_rules` |
| saída unificada | `status_msg` (última mensagem), `debug_log` (histórico), `console_follow` |
| atualização | `update_available`, `update_downloading`, `update_ready`, `driver_version`, `driver_latest`, `driver_update_available` |
| túnel | `tunnel_provider`, `tunnel_status`, `tunnel_pid`, `tunnel_public_url`, `tunnel_exposure`, `tunnel_gate_password`, `tunnel_gate_port`, `tunnel_run_id` |
| privado (só `app.rs` mexe) | `tunnel_child`, `tunnel_gate_stop`, os `Instant` de *throttle* |

O estado de UI é **efêmero por definição**: fechar o app zera tudo o que não estiver no registro.

## 6. Persistência real

Não existe arquivo de configuração do FzComputerAI, e o *storage* do eframe não é usado. O que persiste está no registro do Windows, gravado por `reg.exe` / PowerShell e sempre **relido para confirmar**:

| Chave / valor | Conteúdo | Quem escreve |
| --- | --- | --- |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_PORT` | porta do endpoint HTTP do motor | botão **Aplicar Porta** (`set_user_env_confirmed`) |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_TOKEN` | token do endpoint (motor `0.16+`) | **não é escrito pela GUI** — apenas lido |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_BIND` | — | **é apagado** se existir: o motor oficial a ignora |
| `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` -> `FzComputerAI` | caminho do executável entre aspas | checkbox **Iniciar com Windows** e a task `autostart` do instalador (mesmo nome e mesmo formato, de propósito) |
| `HKCU\Software\FzComputerAI` -> `portproxy:<ip>:<porta>` | porta de destino da regra; marca a regra como **propriedade deste app** | `apply_portproxy()` |
| `HKCU\Software\FzComputerAI` -> `tunnel:<provedor>:<pid>` | `imagem\|CreationDate\|porta\|run_id\|modo` — identidade forte do processo do túnel | `register_tunnel()` |
| `HKCU\Software\FzComputerAI` -> `tunnelcfg:*` | preferências da aba Túnel (provedor, **caminho** do token-file, URL pública, alvo SSH...) | botão **Salvar configuração** |

Regras de ouro da persistência, visíveis no código:

- **Segredo nunca vai para o registro nem para o log.** O token do Cloudflare é gravado em arquivo com ACL restrita (`icacls /inheritance:r /grant:r <usuário>:R`) e só o **caminho** é persistido. A senha do porteiro do túnel existe apenas em memória, por sessão, e é mascarada como `/s/***/` em qualquer texto que vá para o console.
- **Só removemos o que registramos.** A limpeza de regras `portproxy` percorre os valores `portproxy:*` desta chave. Nesta mesma máquina existem regras LAN->loopback de outros serviços; elas não são tocadas. Vale o mesmo para túneis: `taskkill /IM` é **proibido**, porque mataria um `cloudflared`/`ngrok`/`ssh` legítimo do usuário.

## 7. O princípio de status honesto

Nenhum estado exibido é presumido a partir da intenção. Concretamente:

| Estado | Como é provado |
| --- | --- |
| MCP responde | `POST /mcp` com um `initialize` JSON-RPC real; só conta se a resposta contiver `"jsonrpc"`. **GET não serve como prova**: o endpoint MCP responde legitimamente `405 Method Not Allowed` a GET, o que provaria apenas o TCP. |
| listener existe | `netstat -ano -p tcp` — a fonte de verdade do sistema operacional. As linhas cruas vão para a tela, com as mesmas colunas do terminal. |
| endpoint alcançável na LAN | badge verde **só** com listener confirmado no `netstat` **e** POST respondendo no IP da LAN. Se um dos dois falta, o console diz qual e a cor não fica verde. |
| regra de encaminhamento | 3 estados: **REGRA FUNCIONANDO** (existe no `netsh` e o listener está de pé), **REGRA SEM EFEITO** (existe na config, listener ausente), **SEM REGRA**. |
| túnel ativo | `Starting` = processo vivo, URL ainda não capturada; `Running` = URL pública capturada ou informada. "Confirmado pela internet" é um estado **separado** (`tunnel_exposure`), provado por um POST `initialize` real na URL pública. |
| versão do motor | `cua-driver check-update --json` — a API oficial do próprio motor. |
| integridade do instalador baixado | `Get-FileHash -Algorithm SHA256` conferido contra o `.sha256` publicado pelo CI. Divergência apaga o arquivo. |

As cores semânticas (amarelo/vermelho) sobrevivem ao tema monocromático de propósito: elas carregam informação de segurança ("EXPOSTO SEM AUTENTICAÇÃO", "REGRA SEM EFEITO") que se perderia num tema só-verde.

## 8. Interface: o que a arquitetura impõe

- **7 seções** na barra lateral, na ordem do código: MCP & Rede, Túnel, MCP Tools, Calibração, Janelas, Gravação, Doctor & Skills.
- **Um único console global** no rodapé, visível em todas as seções. Antes cada aba tinha sua própria caixa de saída, o que duplicava a mesma informação na mesma tela. Ele se comporta como `tail -f`: acompanha o fim sozinho e **pausa** quando o usuário rola para cima, com indicador "seguindo"/"pausado" e botão **Ir ao fim**.
- **Tema terminal**: fundo preto, texto verde, tudo monoespaçado.
- **Bilíngue PT-BR / EN**, com troca em tempo real via `match state.language` — não há arquivo de tradução nem recarga.
- **Sem emoji e sem glifos ausentes.** A fonte padrão do egui não tem `→`, `●` nem emoji: eles renderizariam caixas vazias. Usa-se `->` em texto e um ponto **desenhado** pelo painter (`status_dot`) para os badges.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — a aba que expõe todo esse diagnóstico na prática.
- [acesso-remoto.md](acesso-remoto.md) — por que o loopback é o limite do motor e quais são as saídas reais.
- [desenvolvimento.md](desenvolvimento.md) — as convenções obrigatórias que mantêm essa arquitetura de pé.
