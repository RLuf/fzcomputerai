# Arquitetura

Para quem precisa entender como as peças se encaixam antes de mexer no código ou depurar um estado estranho na tela.

## 1. Duas peças, papéis distintos

| Peça | O que é | Quem publica | Papel |
| --- | --- | --- | --- |
| `fzcomputerai` | GUI nativa em Rust (egui/eframe 0.29.1), sem Chromium e sem WebView | este repositório (MIT, Roger Luft / Webstorage Tecnologia) | inicia, para, configura, diagnostica e **expõe** o motor |
| `cua-driver` | motor de automação de desktop (clique, teclado, tela, janelas, acessibilidade) | projeto [Cua](https://github.com/trycua/cua) — Cua AI, Inc. (MIT) | faz o trabalho de verdade |

A GUI **não implementa automação nenhuma**. Toda ação da interface termina em uma invocação de `cua-driver` como processo filho. Sem o motor instalado e no PATH, a janela abre, o console registra o erro de execução e nenhum botão produz efeito.

Há uma terceira peça, e ela é do motor: o **pacote de skills**, um conjunto de symlinks para os diretórios dos agentes (Claude Code, Codex, Antigravity, Hermes). Sem esses links, o agente conecta no MCP e **não enxerga ferramenta nenhuma** — e quem acabou de instalar não tem como adivinhar que precisa clicar num botão na aba Doctor & Skills. Por isso, desde a v2.1.1, o `cua-driver skills install` roda **no fim da instalação**. É idempotente e, pelo próprio help do motor, *"Never overwrites existing user links"* — verificado apagando os links e vendo o setup recriar os quatro. O botão da aba Doctor & Skills continua existindo para reparo.

Dependências declaradas em `fzcomputerai/Cargo.toml`: `eframe`, `egui`, `tokio`, `serde`, `serde_json`, `anyhow`, `open` (mais `winresource` como *build-dependency* apenas no Windows). **Não há cliente HTTP.** Requisições HTTP são escritas à mão sobre `std::net::TcpStream` (loopback, sem TLS) ou delegadas a `curl.exe` / PowerShell quando há TLS envolvido.

## 2. Transporte MCP

O motor expõe MCP (Model Context Protocol) por dois transportes:

| Transporte | Como se usa | Observação |
| --- | --- | --- |
| **stdio** | o cliente MCP lança `cua-driver` e conversa por entrada/saída padrão | não envolve rede; não aparece no `netstat` |
| **HTTP** | `POST /mcp`, corpo JSON-RPC 2.0 | **só sobe se `CUA_DRIVER_RS_MCP_HTTP_PORT` estiver definida** — e, no motor `0.17.0`, só sobe se `CUA_DRIVER_RS_MCP_HTTP_TOKEN` também estiver (ver abaixo) |

Detalhes que a GUI depende (verificados no motor instalado e no repositório upstream):

- Sem a variável `CUA_DRIVER_RS_MCP_HTTP_PORT`, **o listener HTTP nem é criado**. Não existe porta padrão implícita — a GUI usa 8000 apenas como valor inicial do campo.
- **O endereço de escuta não é configurável.** O motor oficial escuta somente em `127.0.0.1`; o endereço está fixo no código do Cua (`([127,0,0,1], port)`). A string `CUA_DRIVER_RS_MCP_HTTP_BIND` **não existe** no binário oficial instalado e a busca por ela no repositório `trycua/cua` retorna zero resultado.
- Uma versão anterior desta documentação afirmava haver bind `0.0.0.0`. **Era falso e foi corrigido.** Se alguém quiser reintroduzir a ideia, o comentário em `apply_env_port()` (`fzcomputerai/src/app.rs`) explica por quê não: gravar aquela variável não publica nada, o motor a ignora. A GUI hoje até **remove** a variável se encontrar sobra dela em `HKCU\Environment`, para não confundir o diagnóstico.
- **Autenticação depende da versão do motor.** O contrato abaixo foi **medido no binário `cua-driver` 0.17.0 em 2026-08-03** — antes disso esta documentação apenas repetia a si mesma, sem fonte primária. São **dois** níveis distintos, e confundi-los é a causa clássica de diagnóstico errado:

  1. **Sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do processo, o daemon nem sobe.** `cua-driver serve` sai com código 1 e imprime em `stderr`: `cua-driver serve error: CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`. O resultado não é "requisição recusada", é **porta muda**.
  2. **Com o daemon no ar, requisição sem `Authorization: Bearer <token>` recebe 401**, com o corpo `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}`. Idêntico em `POST /mcp`, `GET /mcp` e `GET /`; **não** vem header `WWW-Authenticate`. O TCP em si é aceito (`Test-NetConnection` na porta responde `True`) — a recusa é da camada de aplicação. Com o header correto: **200**, com o `result` do `initialize`.

  O motor chama esse valor de *host-generated bearer token* — o **host é este app**. Não existe comando no `cua-driver` nem no `install.ps1` oficial do Cua que gere um, então desde a **v2.1.1** a própria GUI gera: 32 bytes do RNG do Windows (64 caracteres hex) e persiste em `HKCU\Environment` na primeira vez que precisa do token. O usuário não precisa saber que a variável existe, e a GUI **nunca imprime o valor no log**; para lê-lo: `reg query HKCU\Environment /v CUA_DRIVER_RS_MCP_HTTP_TOKEN`. Versões antigas do motor (série `0.8.x`) **não têm token nenhum** — o instalador não pina versão: o passo do motor executa o instalador oficial do Cua, que resolve a última versão estável publicada. A GUI lê o token de `HKCU\Environment` na abertura (`read_mcp_token()`) e envia o header **somente quando há token configurado** — assim o mesmo teste funciona com as duas gerações.

- **Só existe um daemon, e ele herda o ambiente de quem o lançou.** Um segundo `serve` é recusado com `Cua Driver daemon is already running on \\.\pipe\cua-driver (pid N). Run 'cua-driver stop' first.`. Por isso, desde a **v2.1.1**, a GUI é **dona** do daemon: ela lança `cua-driver serve` como **processo filho**, com porta e token injetados no ambiente desse processo. A Scheduled Task `cua-driver-serve` (a que `cua-driver autostart kick` aciona) ficou como **último recurso**, porque herda o ambiente do **logon**: quem gravou o token em `HKCU\Environment` **depois** de logar sobe um daemon sem token, que morre na hora pelo item 1 e deixa a porta muda com a GUI dizendo apenas "PARADO". Quando o caminho da task é usado, o console avisa que o processo **não é da GUI** e que não haverá logs do motor.

Consequência direta: **sair do loopback nunca é questão de configuração do motor**. É encaminhamento (LAN) ou túnel (internet). Ver [acesso-remoto.md](acesso-remoto.md).

### 2.1 Encaminhamento LAN feito pelo próprio app

O caminho atual para publicar o endpoint na rede local **não é mais o `netsh portproxy`**. Desde a v2.1.1, uma **thread do próprio processo** escuta em `<IP_LAN>:<porta>` e copia bytes contra `127.0.0.1:<porta>` (`std::net::TcpListener` + `std::io::copy`, **sem nenhuma dependência nova**). É TCP puro: `curl`, `telnet` e `nc` atravessam igual.

Os motivos são medidos, não estéticos:

- o `portproxy` é regra **estática** do serviço **IP Helper**: exigia admin/UAC para criar **e** para remover;
- continuava aparecendo `LISTENING` na LAN **mesmo com o motor morto**, aceitando conexões que morriam no destino — um falso positivo de "serviço no ar";
- **sobrevivia** ao fechamento do app e ao reboot, o que obrigava uma rotina de limpeza.

Evidência da troca: com o app aberto, o `netstat` mostrou `127.0.0.1:8000` e `192.168.0.101:8000` em `LISTENING`, o MCP respondeu **HTTP 200** nos dois, e `netsh interface portproxy show v4tov4` não tinha **nenhuma** regra. Ao fechar o app, as duas portas fecharam junto e nada ficou no sistema.

O `netsh portproxy` continua no código **apenas como fallback**, para quando o bind no IP da LAN falha.

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
   │ cua-driver  (serve/stop/call/      │   │ TcpStream  POST /mcp  {initialize}   │
   │             doctor/skills/         │   │ netstat -ano -p tcp                  │
   │             check-update/update)   │   │ netsh interface portproxy show       │
   │ netsh (fallback) / reg             │   │ curl.exe  (única via com TLS)        │
   │ cloudflared | ngrok | ssh          │   └──────────────────────────────────────┘
   └───────────────┬────────────────────┘
                   │
                   v
   ┌──────────────────────────────────────────────────────────────────────────────┐
   │ MOTOR cua-driver — MCP HTTP em 127.0.0.1:<porta>  (endereço FIXO no código)  │
   └──────────┬───────────────────────────────┬───────────────────────────────────┘
              │                               │
   thread do app (TcpListener)         túnel de saída                 (nada mais
   <IP_LAN>:porta -> 127.0.0.1:porta   cloudflared/ngrok/ssh -R        alcança)
   (netsh portproxy só como fallback)
              │                               │
              v                               v
        outra máquina da LAN            URL HTTPS pública na internet
                                        (opcional: porteiro de senha local
                                         127.0.0.1:efêmera, exige /s/<senha>/)
```

## 4. Fluxo de uma ação, do clique ao console

Exemplo: o usuário clica em **Iniciar** na aba MCP & Rede.

1. `tabs/network.rs` desenha o botão e, no `clicked()`, chama `state.start_daemon()`. A camada de UI **não** executa processo nenhum — ela só chama métodos de `AppState`.
2. `AppState::start_daemon()` (`fzcomputerai/src/app.rs`) **primeiro testa o endpoint**. Se ele já responde, a função **não encosta no daemon**. Antes da v2.1.1 ela parava o motor e subia outro sem checar nada — e no Windows o socket de uma porta que já teve conexão fica retido em `TIME_WAIT` por minutos, então o `serve` novo não conseguia o bind: `MCP HTTP transport disabled — bind 127.0.0.1:8000 failed (os error 10048)`, ou seja, daemon zumbi (pipe vivo, porta muda). Na prática, clicar **Iniciar** quebrava o que estava funcionando. Para forçar troca de processo existe **Reiniciar**.
3. Não havendo endpoint de pé, a GUI lança `cua-driver serve` como **processo filho dela**, com `CUA_DRIVER_RS_MCP_HTTP_PORT` e `CUA_DRIVER_RS_MCP_HTTP_TOKEN` injetados no ambiente do filho, e com `stdout`+`stderr` redirecionados para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`. O console segue esse arquivo como `tail -f`, prefixando as linhas com `[motor]` — é assim que a atividade de clientes MCP externos (conector do Claude, Antigravity, Cursor) aparece na tela. Enquanto quem subia o motor era a Scheduled Task, o processo nascia filho do **Agendador**, o `stdout` pertencia à task e esses logs simplesmente **sumiam**.
4. `run_logged()` monta o comando com `quiet_cmd()` — que no Windows aplica `CREATE_NO_WINDOW`, para nenhuma janela preta piscar na tela — executa com `output()` e registra no log: a linha de comando, o `exit code`, o `stdout` e o `stderr`, sempre com o resultado real.
5. `log_debug()` anexa a entrada em `AppState::debug_log`, um `String` limitado a 64 KB (o excesso é cortado pelo início, em fronteira de caractere).
6. Ainda em `start_daemon()`, `check_port_status()` refaz o teste real do endpoint e recalcula os badges. `daemon_running` recebe o resultado do teste — **nunca** "eu mandei iniciar, então está ligado".
7. No próximo frame, o painel do console no rodapé desenha `debug_log`; a faixa amarela acima dele mostra a primeira linha de `status_msg` (a última mensagem relevante).

Ações longas fogem desse caminho síncrono para não travar a UI: download do instalador, `cua-driver update --apply`, download de `cloudflared`/`ngrok` e o processo do túnel são disparados com `spawn()` em processo destacado, e a GUI observa o resultado por **arquivos de flag** em `%TEMP%` ou pelo log do próprio CLI, com *throttle* de 1 s dentro de cada `poll_*`. O `update()` do eframe chama `request_repaint_after(1s)` enquanto houver algo pendente, para os polls acontecerem mesmo sem input do usuário.

A **limpeza ao fechar** deixou de ser um script: até a v2.1.0 o `on_exit` disparava um `powershell -WindowStyle Hidden` de cerca de 2 KB que esperava o processo morrer, matava processos, escrevia no registro, rodava `netsh` e ainda chamava `-Verb RunAs`. O **Microsoft Defender flagrou essa linha de comando** nesta máquina (detecção `2147941383`, 2026-08-03). Na v2.1.1 a limpeza é **Rust nativo**: chamadas diretas e curtas, sem elevação e sem PowerShell.

## 5. Onde vive o estado

Tudo em `AppState` (`fzcomputerai/src/app.rs`), uma struct única passada como `&mut` para cada aba. Não há gerenciador de estado, canal, nem `Arc<Mutex<...>>` global. Blocos principais:

| Bloco | Campos representativos |
| --- | --- |
| idioma e navegação | `language`, `active_tab` |
| endpoint MCP | `http_port`, `lan_ip`, `port_active`, `port_status`, `mcp_token` |
| encaminhamento LAN | `portproxy_active`, `portproxy_effective`, `real_listeners`, `portproxy_rules` — o encaminhamento em si é uma **thread deste processo** (ver 2.1); o `netsh` só entra como fallback |
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
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_TOKEN` | token do endpoint (motor `0.16+`): 32 bytes do RNG do Windows, 64 caracteres hex | **gerado e gravado pela própria GUI** (v2.1.1) na primeira vez que precisa dele, e injetado no ambiente do `serve` que ela lança. O valor **nunca** vai para o log; para lê-lo, `reg query HKCU\Environment /v CUA_DRIVER_RS_MCP_HTTP_TOKEN`. A Scheduled Task do daemon só enxerga a variável **no próximo logon** |
| `HKCU\Environment` -> `CUA_DRIVER_RS_MCP_HTTP_BIND` | — | **é apagado** se existir: o motor oficial a ignora |
| `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` -> `FzComputerAI` | caminho do executável entre aspas | checkbox **Iniciar com Windows** e a task `autostart` do instalador (mesmo nome e mesmo formato, de propósito) |
| `HKCU\Software\FzComputerAI` -> `portproxy:<ip>:<porta>` | porta de destino da regra; marca a regra como **propriedade deste app** | `apply_portproxy()` — **só no fallback `netsh`**; o encaminhamento normal é a thread do app e não persiste nada |
| `HKCU\Software\FzComputerAI` -> `tunnel:<provedor>:<pid>` | `imagem\|CreationDate\|porta\|run_id\|modo` — identidade forte do processo do túnel | `register_tunnel()` |
| `HKCU\Software\FzComputerAI` -> `tunnelcfg:*` | preferências da aba Túnel (provedor, **caminho** do token-file, URL pública, alvo SSH...) | botão **Salvar configuração** |

Regras de ouro da persistência, visíveis no código:

- **Segredo nunca vai para o registro nem para o log.** O token do Cloudflare é gravado em arquivo com ACL restrita (`icacls /inheritance:r /grant:r <usuário>:R`) e só o **caminho** é persistido. A senha do porteiro do túnel existe apenas em memória, por sessão, e é mascarada como `/s/***/` em qualquer texto que vá para o console.
- **Só removemos o que registramos.** A limpeza de regras `portproxy` percorre os valores `portproxy:*` desta chave (e hoje quase nunca há o que limpar: o encaminhamento normal morre junto com o processo). Nesta mesma máquina existem regras LAN->loopback de outros serviços; elas não são tocadas. Vale o mesmo para processos: `taskkill /IM` é **proibido**, porque mataria um `cloudflared`/`ngrok`/`ssh` legítimo do usuário. Na v2.1.1 o `taskkill /F /IM cua-driver.exe` foi removido também **do app e do instalador** — ele matava *todo* processo com esse nome, inclusive um motor que o usuário estivesse usando para outra coisa. O encerramento agora é `cua-driver stop` (o comando oficial) e, se ainda sobrar processo, `kill` por **PID** com o caminho do executável conferido.

## 7. O princípio de status honesto

Nenhum estado exibido é presumido a partir da intenção. Concretamente:

| Estado | Como é provado |
| --- | --- |
| MCP responde | `POST /mcp` com um `initialize` JSON-RPC real; só conta se a resposta contiver `"jsonrpc"`. **GET não serve como prova**: o endpoint MCP responde legitimamente `405 Method Not Allowed` a GET, o que provaria apenas o TCP. |
| listener existe | `netstat -ano -p tcp` — a fonte de verdade do sistema operacional. As linhas cruas vão para a tela, com as mesmas colunas do terminal. |
| endpoint alcançável na LAN | badge verde **só** com listener confirmado no `netstat` **e** POST respondendo no IP da LAN. Se um dos dois falta, o console diz qual e a cor não fica verde. |
| encaminhamento LAN | com o encaminhamento do app, o listener em `<IP_LAN>:<porta>` só existe **enquanto o processo existe**, e é provado no `netstat` mais um POST real nesse IP. Os 3 estados **REGRA FUNCIONANDO** / **REGRA SEM EFEITO** (existe na config, listener ausente) / **SEM REGRA** descrevem o fallback `netsh`, cuja regra é estática e chegava a anunciar `LISTENING` na LAN **com o motor morto**. |
| túnel ativo | `Starting` = processo vivo, URL ainda não capturada; `Running` = URL pública capturada ou informada. "Confirmado pela internet" é um estado **separado** (`tunnel_exposure`), provado por um POST `initialize` real na URL pública. |
| versão do motor | `cua-driver check-update --json` — a API oficial do próprio motor. |
| integridade do instalador baixado | `Get-FileHash -Algorithm SHA256` conferido contra o `.sha256` publicado pelo CI. Divergência apaga o arquivo. |

As cores semânticas (amarelo/vermelho) sobrevivem ao tema monocromático de propósito: elas carregam informação de segurança ("EXPOSTO SEM AUTENTICAÇÃO", "REGRA SEM EFEITO") que se perderia num tema só-verde.

## 8. Interface: o que a arquitetura impõe

- **7 seções** na barra lateral, na ordem do código: MCP & Rede, Túnel, MCP Tools, Calibração, Janelas, Gravação, Doctor & Skills.
- **Um único console global** no rodapé, visível em todas as seções. Antes cada aba tinha sua própria caixa de saída, o que duplicava a mesma informação na mesma tela. Ele se comporta como `tail -f`: acompanha o fim sozinho e **pausa** quando o usuário rola para cima, com indicador "seguindo"/"pausado" e botão **Ir ao fim**. Além dos comandos executados, ele segue o log do motor (`%TEMP%\fzcomputerai-update\cua-driver-serve.log`, linhas com prefixo `[motor]`). O **Ir ao fim** tem **precedência** sobre a detecção de rolagem: até a v2.1.0 o clique setava `console_follow = true` e, no **mesmo frame**, a detecção de posição sobrescrevia o valor com a posição **antiga** — o botão simplesmente não funcionava.
- **Tema terminal**: fundo preto, texto verde, tudo monoespaçado.
- **Bilíngue PT-BR / EN**, com troca em tempo real via `match state.language` — não há arquivo de tradução nem recarga.
- **Sem emoji e sem glifos ausentes.** A fonte padrão do egui não tem `→`, `●` nem emoji: eles renderizariam caixas vazias. Usa-se `->` em texto e um ponto **desenhado** pelo painter (`status_dot`) para os badges.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — a aba que expõe todo esse diagnóstico na prática.
- [acesso-remoto.md](acesso-remoto.md) — por que o loopback é o limite do motor e quais são as saídas reais.
- [desenvolvimento.md](desenvolvimento.md) — as convenções obrigatórias que mantêm essa arquitetura de pé.
