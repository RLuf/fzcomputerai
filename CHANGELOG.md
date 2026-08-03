# Changelog

Todas as alterações notáveis do projeto **FzComputerAI / CUA Driver Computer Vision MCP** serão documentadas neste arquivo.

O formato baseia-se no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/spec/v2.0.0.html).

---

## [2.1.0] - 2026-08-03

### Alterado (instalador — motor sempre na última versão)
- **Fim da versão fixada do motor no instalador.** O `installer/fzcomputerai.iss` não tem mais `/DCuaDriverVersion` (era `0.8.3` cravado — um instalador que "atualizava" para um motor 9 versões atrás do publicado): o passo do motor executa o instalador oficial do projeto Cua com **alvo explícito** — `-Release <latest_version>` vindo de `cua-driver check-update --json` na hora da instalação. **Descoberta do teste real**: sem `-Release`, o `install.ps1` oficial **não consulta o GitHub** — instala o `BAKED_VERSION` congelado dentro do próprio script (precedência documentada nele: env > `-Release` > baked > API; o script embarcado estava com baked `0.8.3` contra `0.17.0` publicado). Por isso: alvo conhecido → script **embarcado** (auditável) com `-Release` exato; alvo desconhecido (sem motor / consulta muda) → script do **endpoint oficial cua.ai** (baked atualizado pelo CD do Cua a cada release; conferido: `0.17.0` == latest), com o embarcado como **fallback offline**. "Nada a fazer" = **instalado == latest** confirmado pelo check-update; `/FORCEENGINE` continua forçando a reinstalação. Sem rede, tudo falha rápido e **não-fatal** (o motor atual permanece intacto).
- **Caminho resolvido do motor em todo o instalador.** `check-update`, `stop` e `autostart kick/enable/disable` passam pelo caminho real do exe (PATH ou o canônico `%LOCALAPPDATA%\Programs\Cua\cua-driver\bin\cua-driver.exe`) — o nome puro `cua-driver` dependia de um PATH que o processo do setup não herda logo após uma instalação do motor, e um `stop` que falha em silêncio deixava o daemon antigo vivo durante a troca de versão.
- **Removido o botão "Verificar atualizações (internet)"** da página de pré-requisitos: redundante agora que o passo final sempre resolve a latest — e ficava sem função quando não havia motor instalado.
- **Destino default nunca herda `%TEMP%`.** Se o diretório lembrado da instalação anterior (ou passado via `/DIR=`) estiver dentro de `%TEMP%`/`%TMP%`, o setup corrige para `%LOCALAPPDATA%\Programs\FzComputerAI` — instalar em pasta temporária significa produto apagado pela limpeza automática do Windows, além de contaminar a detecção de instalação anterior/órfã. A comparação tolera formas 8.3 x longas do mesmo caminho (`RUNNER~1` x `runneradmin`), onde a checagem textual pura viraria no-op.
- **Consulta de atualização com limite de 20 s.** O `check-update` que decide "nada a fazer" roda sob timeout (via wrapper PowerShell) — o `Exec` do Inno não tem timeout e essa chamada já foi medida travando >120 s nesta máquina; sem o limite, uma rede pendurada suspenderia a instalação inteira, inclusive o auto-upgrade silencioso da GUI (que fecha o app antes de rodar o setup). Estouro do limite = estado desconhecido: o instalador oficial roda mesmo assim, de forma não-fatal.
- **Aviso de compilação quando o submódulo `cua` está ausente**: antes, o `skipifsourcedoesntexist` escondia que o setup tinha sido gerado SEM o `install.ps1` embarcado (o fallback offline auditável virava código morto em silêncio).

### Testado (evidências reais de 2026-08-03, máquina Windows 11)
- **Instalador E2E**: 3 execuções `/VERYSILENT` com `exit 0`; passo do motor terminou `ok=sim` com `cua-driver --version = 0.17.0` verificado por execução; 17 arquivos em `%LOCALAPPDATA%\Programs\FzComputerAI`.
- **Desinstalador E2E**: `exit 0`; 17 → 0 arquivos; chave `Run\FzComputerAI` ausente após remover; **motor preservado por design** (0.17.0 respondendo e daemon vivo durante e após a desinstalação da GUI).
- **Relatório final da cópia instalada**: 100% `[OK]` — GUI instalada, motor FUNCIONAL (versão executada, não caminho), autostart ativo, listener real na 8000 e `POST /mcp initialize` = **HTTP 200**.
- **Contrato de auth do motor 0.17.0 PROVADO no binário** (fim da disputa de documentação). Antes desta versão a afirmação "0.16+ exige token" aparecia em 8 arquivos deste repositório **sem nenhuma fonte primária** — era documentação citando documentação. O que foi medido no binário 0.17.0 desta máquina, em 2026-08-03:
  - `cua-driver serve` **sem** `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente: sai com **exit 1** e `stderr` = `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`;
  - com o daemon no ar, **toda** requisição sem `Authorization` recebe **401** `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` — testado em `POST /mcp`, `GET /mcp` e `GET /` (o TCP conecta normalmente; a recusa é da aplicação, e **não** há header `WWW-Authenticate`);
  - com `Authorization: Bearer <token>`: **HTTP 200** com o `result` do `initialize`.
  O token vive em `HKCU\Environment` e é lido pela GUI e pelo `verify-install.ps1`.

### Alterado (tudo vira filho do app — fim do netsh, do PowerShell oculto e do alarme do antivírus)
- **Encaminhamento LAN feito pelo próprio app, não mais por `netsh interface portproxy`.** Uma thread do processo escuta em `<ip_lan>:porta` e copia bytes contra `127.0.0.1:porta` (`std::net::TcpListener` + `std::io::copy` — **zero dependência nova**). É TCP puro: `curl`, `telnet` e `nc` atravessam igual. O portproxy era regra **estática do IP Helper**: exigia admin/UAC, continuava `LISTENING` na LAN **mesmo com o motor morto** (aceitando conexões que morriam no destino) e **sobrevivia ao fechamento do app e ao reboot**. O `netsh` ficou só como fallback, quando o bind no IP da LAN falha.
- **`shutdown_cleanup` reescrito em Rust nativo.** A versão anterior disparava um `powershell -WindowStyle Hidden` de ~2 KB que esperava o processo morrer, matava processos, escrevia no registro, rodava `netsh` e chamava `-Verb RunAs`. **O Microsoft Defender flagrou exatamente essa linha de comando** (detecção `2147941383`, 2026-08-03 19:19) — e com razão: é o retrato de script oculto + kill + persistência + elevação. Agora são chamadas diretas e curtas, sem elevação e sem PowerShell.
- **`taskkill /F /IM cua-driver.exe` removido do app e do instalador.** Matava **todo** processo com esse nome na máquina, inclusive um motor de outro uso — a mesma prática que o `AGENTS.md` já proibia para os binários de túnel. Agora: `cua-driver stop` (o comando oficial da CLI) e, se sobrar, kill **por PID com o caminho do executável conferido**.
- **Token do endpoint gerado pela própria GUI.** O motor 0.16+ chama o valor de *host-generated bearer token* — o host é o app. Sem isso, o `serve` sai com erro e nenhuma porta abre, deixando o produto "PARADO" para sempre até alguém descobrir que precisa criar uma variável de ambiente à mão. A GUI gera 32 bytes do RNG do Windows (64 chars hex) e persiste em `HKCU\Environment`.
- **Pacote de skills instalado pelo setup.** `cua-driver skills install` passou a rodar no fim da instalação: sem os symlinks, o agente conecta no MCP e **não enxerga ferramenta nenhuma**. O botão na aba Doctor & Skills já existia, mas quem acabou de instalar não tem como saber que precisa clicar nele. Verificado apagando os links e vendo o setup recriar os quatro (Claude Code, Codex, Antigravity, Hermes).
- **Aviso de segurança da aba Túnel deixou de mentir.** O texto de "sem token" afirmava que *"o endpoint aceita qualquer requisição"* — **falso** em motores 0.16+, onde sem token não existe endpoint algum (medido: `serve` falha, `netstat` vazio). Agora o texto depende da versão real do motor e não inventa risco.
- **Bug meu, corrigido no mesmo dia:** a verificação "a porta está livre?" que eu havia adicionado usava um `TcpListener::bind` de teste — e era **ela** que fazia o motor seguinte falhar com `bind 127.0.0.1:8000 failed (os error 10048)`, virando daemon zumbi (pipe vivo, porta muda). Trocada por espera do processo anterior encerrar, sem tocar na porta.

### Corrigido (a GUI passou a ser DONA do daemon — e por isso enxerga os logs)
- **`start_daemon` não delega mais para o Agendador de Tarefas.** A função chamava `cua-driver autostart kick`, que manda a Scheduled Task `cua-driver-serve` subir o motor — o processo nascia filho do **Agendador**, não da GUI. Três consequências, todas observadas na máquina:
  1. **os logs do motor sumiam**: o stdout/stderr pertencia à task e se perdia, então o console da GUI só mostrava os comandos que a *própria* GUI executava (`run_logged`). Um cliente MCP **externo** — o conector do Claude, Antigravity, Cursor — conversando com o motor não aparecia em lugar nenhum;
  2. **ambiente errado**: a task herda o ambiente do **logon**, então porta e token gravados depois de logar não eram vistos e o daemon morria no ato (motor 0.16+), deixando a porta muda com a GUI dizendo apenas "PARADO";
  3. a GUI não era dona do ciclo de vida daquilo que ela gerencia.
  Agora ela lança `cua-driver serve` como **processo filho**, com `CUA_DRIVER_RS_MCP_HTTP_PORT` e `CUA_DRIVER_RS_MCP_HTTP_TOKEN` lidos do registro e **injetados no processo**, derrubando antes qualquer daemon de outro dono (só pode existir um). A Scheduled Task permanece como **último recurso**, e nesse caso o console avisa explicitamente que o processo não é da GUI e que não haverá logs.
- **Logs REAIS do motor no console, em tempo real.** O daemon lançado pela GUI escreve stdout+stderr em `%TEMP%\fzcomputerai-update\cua-driver-serve.log`, e a GUI segue o arquivo como `tail -f` (`poll_engine_log`, ~0,7 s), prefixando cada linha com `[motor]`. É o que faz a atividade de clientes MCP externos finalmente aparecer na tela.
- **Botão "Ir ao fim" do console não funcionava.** O clique setava `console_follow = true`, mas a detecção de posição do scroll, no **mesmo frame**, sobrescrevia o valor usando a posição **antiga** — o botão era anulado antes de surtir efeito. Agora o salto (`console_jump`) tem precedência e só o frame seguinte volta a decidir pela posição real.

### Corrigido (regressão do daemon com motor 0.16+)
- **O botão "Iniciar" não subia o daemon quando havia token configurado.** `start_daemon` chamava só `cua-driver autostart kick`, e a Scheduled Task do motor herda o ambiente do **logon** — quem gravou `CUA_DRIVER_RS_MCP_HTTP_TOKEN` depois de logar (o caso normal, inclusive logo após instalar) subia um daemon sem token, que morre no ato e deixa a porta muda, com a GUI mostrando **PARADO** sem explicar por quê. Agora, quando o `kick` não abre a porta, a GUI lê porta e token de `HKCU\Environment`, derruba o daemon anterior (só pode existir **um** — `serve` recusa com `already running`) e lança `serve` com essas variáveis **injetadas no processo filho**, sondando a porta por até 4 s antes de reportar estado. Sem token no registro, o log diz exatamente isso em vez de falhar em silêncio.
- **Limitação conhecida** (não-fatal, registrada no `.iss`): dentro do processo do setup a consulta `check-update` pode retornar vazia mesmo com rede — o passo então executa o instalador oficial mesmo assim, que resolve a latest e pula download já em disco ("release already on disk"). Comportamento observado nos logs `archived/2026-08-03-release/setup-e2e*.log`.

### Adicionado (empacotamento e release)
- **Pacote portátil Windows** (`fzcomputerai-portable-v<versão>-windows-x64.zip`): exe + marcador `fzcomputerai.portable` (preferências em `.ini` ao lado, sem registro, sem autostart) + LEIA-ME + licença; gerado por `scripts/make-portable.ps1` e agora também no CI, com `.sha256`.
- **Pacotes Linux no CI**: `.deb` (cargo-deb), `.rpm` (cargo-generate-rpm) e **AppImage** (appimagetool oficial) — sem snap, sem dmg, por decisão do projeto. Metadados em `fzcomputerai/Cargo.toml` (`[package.metadata.deb]`/`[package.metadata.generate-rpm]`).
- **Corpo do release** lista também `Source code (zip / tar.gz)` (anexados automaticamente pelo GitHub) e descreve cada artefato; continua listando **somente o que existe de fato**.
- **`verify-install.ps1` honesto de ponta a ponta**: a checagem do motor **executa** `--version` (PATH → caminho canônico) e imprime a versão real — junction pendurada agora sai `[FALHA]` com causa, não `[OK]` mentiroso; o teste de MCP envia `Authorization: Bearer` quando o token existe em `HKCU\Environment` e traduz `401` em diagnóstico (token ausente × token divergente).
- **Proteção contra lock órfão** do instalador oficial do motor (`~/.cua-driver/install.lock` de instalação morta, >30 min): removido antes de invocar o `install.ps1` — no `.iss` e nos dois fluxos da GUI. Uma instalação travada às 11:33 segurou o lock por 4h e pendurou todas as instalações seguintes; o próprio `install.ps1` oficial espera o lock para sempre.

### Alterado (GUI — botão atualizador age de ponta a ponta)
- **"Verificar Atualizações" virou "Verificar e Atualizar"** (aba MCP & Rede) e passou a **agir** em vez de só relatar: motor desatualizado começa a atualizar **sozinho** — para o daemon antigo, aplica a última versão estável (`cua-driver update --apply`; se o subcomando não existir ou falhar, como no 0.8.3, **fallback automático para o instalador oficial** do projeto Cua, que instala a latest do GitHub) e religa o autostart (**daemon novo no ar**), sem mais cliques. GUI desatualizada: o instalador baixa em segundo plano com **SHA256 conferido**; apenas a troca final pede confirmação, porque exige fechar o aplicativo.
- **Caminho resolvido do motor também na GUI** (`engine_exe()`): consulta e atualização funcionam imediatamente após instalar o motor, mesmo com o PATH da sessão desatualizado — um processo já aberto não herda o PATH que o `install.ps1` acabou de gravar. O banner "motor NÃO encontrado" e o `check_driver_present` usam o mesmo critério (PATH **ou** caminho canônico), eliminando o estado contraditório "Motor atualizado. Versão: X" + banner vermelho pedindo reinstalação.
- **Blindagem do fluxo automático** (19 defeitos confirmados por revisão adversarial e corrigidos antes do release):
  - veredito de sucesso/falha do motor vem do **exit code real** do `update --apply`/instalador — nunca "o script terminou, logo deu certo" (falso sucesso que anunciava "Motor atualizado" com o binário antigo);
  - o daemon é religado (`autostart kick`) em **todos** os caminhos, inclusive exceção — falha de atualização não deixa mais o motor parado até o próximo logon;
  - `powershell` sempre com `-ExecutionPolicy Bypass`: em máquina com política `Restricted` (default de Windows client), instalar/atualizar o motor pela GUI falhava sempre — o `.iss` já fazia isso, a GUI não;
  - fallback de rede baixa o script oficial **para arquivo** e o invoca (`& $s`), nunca `irm | iex`: no `iex`, um `exit 1` do script matava o processo antes de gravar qualquer flag (spinner eterno, retry bloqueado até reiniciar o app) e o `$ErrorActionPreference='Stop'` dele vazava para o wrapper;
  - flags de progresso pré-limpas no lado Rust antes do spawn: fim da corrida em que o retry consumia `error.flag`/`ready.flag` **da tentativa anterior** (download íntegro órfão, diálogo de instalar abrindo sobre exe sendo re-baixado);
  - a troca da GUI **espera a atualização do motor terminar**: confirmar "Fechar e instalar agora" no meio do update matava o `cua-driver update --apply` na troca de junction e punha dois instaladores do motor em corrida;
  - falha de download/SHA256 do instalador **aparece na UI** (antes: só no console de debug, com a Central de Atualizações afirmando "atualizada" em verde logo após a falha);
  - caminhos com apóstrofo (usuário `O'Neil`) não quebram mais o parse dos scripts PowerShell (escape `''` em toda interpolação);
  - consulta do motor com `--no-cache` (com fallback para o cache): o cache de 20 h do `check-update` fazia o botão declarar "atualizado" contra release publicada há 1 h;
  - estado de atualização é **zerado quando a consulta falha**: o auto-disparo nunca mais para o daemon com base em `update_available` obsoleto de uma consulta antiga;
  - "Depois" é respeitado: não reabre a Central afirmando "atualizada" com o instalador ainda estacionado no `%TEMP%`.

### Alterado (licenciamento)
- **Licença alterada de CC BY 4.0 para MIT.** Motivo: a própria Creative Commons **não recomenda** licenças CC para software (não tratam código-fonte nem patentes), e a CC-BY cria fricção de adoção para quem quer depender do projeto. A MIT é também a licença do projeto **Cua** (`trycua/cua`), o que torna o ecossistema coerente. Copyright do FzComputerAI: `(c) 2026 Roger Luft (VeilWalker) — Webstorage Tecnologia`. Atualizados `fzcomputerai/Cargo.toml`, `package.json`, `installer/LICENSE.txt`, READMEs e `AGENTS.md` §3.
- **Criado `LICENSE.md` na raiz** — o arquivo **não existia** (havia apenas `installer/LICENSE.txt`), embora o `AGENTS.md` já exigisse preservar copyright nele. Agora contém: a MIT do FzComputerAI, o **texto integral da MIT do projeto Cua** (`Copyright (c) 2025 Cua AI, Inc.`, conforme exigido pela licença), a **citação formal** pedida pelos autores (`@software{cua2025...}`), a lista de componentes de terceiros (egui/eframe, cloudflared, ngrok, OpenSSH) e uma seção de **agradecimento** à Cua AI, Inc. e à comunidade do Cua — o `cua-driver` é a base sobre a qual esta GUI foi construída.

### Adicionado (interface)
- **Ícone próprio do aplicativo.** O repositório **não tinha nenhum `.ico`** — por isso o Windows exibia ícone genérico na busca, na barra de tarefas e no título. Agora há `installer/fzcomputerai.ico` (multi-tamanho: 16/32/48/64/128/256), gerado de forma reproduzível por `scripts/make-icon.ps1`, e o ícone da **janela em execução** (`ViewportBuilder::with_icon`), que é um caminho separado do recurso do `.exe`: usa `fzcomputerai/assets/icon64.rgba` (RGBA cru embutido por `include_bytes!`, para não precisar da feature `image` do eframe — zero dependência nova).
- **Seção DONATE no diálogo Sobre**, com botão para **GitHub Sponsors** (`github.com/sponsors/RLuf`), link copiável e coração desenhado no painter (a fonte padrão não tem glifo de emoji). Adicionados `.github/FUNDING.yml` e badge de patrocínio nos READMEs.
- **Crédito ao projeto Cua dentro do app**: o diálogo Sobre agora exibe a licença MIT, o copyright da Cua AI, Inc., o agradecimento e o link para o repositório oficial.

### Alterado (interface)
- **Console unificado.** Antes cada aba tinha a sua própria caixa de saída (`calibration_log`, `windows_log`, `recording_log`, `doctor_output`, `skills_output`, `mcp_tools_output`, `tunnel_output`) **e** a aba MCP & Rede tinha o Console Debug — dois consoles com a mesma informação na mesma tela. Agora há **um único console global**, em faixa redimensionável acima do rodapé, **visível em todas as abas**, com comportamento de `tail -f`: acompanha o fim do log automaticamente, **pausa quando o usuário rola para cima** para poder ler e volta a acompanhar ao retornar ao fim (indicador honesto "seguindo"/"pausado" + botão "Ir ao fim"). Os 7 campos por aba foram consolidados em `status_msg` (faixa da última mensagem) + `debug_log` (histórico).
- **Rodapé**: removido o número de telefone; passa a exibir apenas `Roger Luft (VeilWalker) | Webstorage Tecnologia`. O apoio ao projeto migrou para a seção DONATE do Sobre.

### Adicionado
- **Nova aba "Túnel (Internet)" na GUI (`tabs/tunnel.rs`)**: expõe o MCP HTTP local (`127.0.0.1:<porta>`) na internet — HTTPS público -> HTTP local — por três vias:
  - **Cloudflare Tunnel**: quick tunnel (sem conta, URL `*.trycloudflare.com`) e túnel nomeado (login OAuth via `cloudflared tunnel login`, ou token colado gravado em token-file com ACL restrita — o token nunca vai para argv/log/registro).
  - **ngrok**: download só após aceite dos Termos de Serviço (modal + link para `ngrok.com/tos`); URL descoberta pelo log `logfmt` ou pela API local `127.0.0.1:4040`; opção de basic-auth via traffic policy gerada.
  - **SSH reverso**: servidor próprio (`-R ... user@host`, chave, `BatchMode=yes`) ou presets públicos (localhost.run / serveo).
  - Captura automática da URL pública (scanner por sufixo, sem dependência nova), botão **Copiar**, snippet `mcpServers` pronto e **sonda de exposição** que testa de verdade a URL pública com `POST initialize` (badge honesto: exposto / borda exigiu auth / não verificável).
- **Autenticação nível 1 — senha na URL (porteiro local)**: ao iniciar com senha, um mini reverse-proxy em `127.0.0.1` exige `/s/<senha>/` no caminho antes de encaminhar ao MCP (a URL vira `https://<host>/s/<senha>/mcp`). Sem a senha, o porteiro responde 404. (Nível 2 — OAuth ou aprovação manual na tela — fica para etapa futura; a arquitetura do porteiro já reserva o ponto de extensão.)
- **Download verificado de binários** (padrão do auto-upgrade): cloudflared (Apache-2.0) e ngrok baixados em segundo plano, com hash SHA256 e status Authenticode registrados no log; nunca embarcados no instalador/release.
- **Ciclo de vida limpo — o túnel nunca sobrevive ao app** (4 camadas): `Child::kill()` no `on_exit`; **watchdog independente disparado no início** (mata o túnel se a GUI morrer, inclusive por `kill -9`/crash); bloco de túneis no `shutdown_cleanup`; e reconciliação na abertura (`startup_reconcile_tracked_tunnels`). O processo alvo é morto **apenas** com identidade de 3 fatores (imagem + `CreationDate` + marcador único na command line) — nunca `taskkill /IM`, que atingiria cloudflared/ngrok/ssh legítimos de outros usos. Rastreamento em `HKCU\Software\FzComputerAI` (`tunnel:<provider>:<pid>`).
  - Limite honesto: se a GUI **e** o watchdog forem mortos no mesmo instante, o túnel sobrevive até a próxima abertura da GUI (removido então pela reconciliação).
- **Chip global "TÚNEL ATIVO -> internet"** no cabeçalho, visível de qualquer aba enquanto um túnel estiver de pé.
- Config da aba persistida em `HKCU\Software\FzComputerAI` (`tunnelcfg:*`); a proteção efetiva depende da modalidade e da configuração do provedor.

### Corrigido
- **Nota de correção à entrada [2.0.0] — regras de Firewall**: o item "Aplicação de regras do Windows Firewall com checagem deduplicada de nomes" **não corresponde ao código** — a GUI nunca cria regras de Windows Firewall (`New-NetFirewallRule`/`netsh advfirewall`); a exposição LAN é feita apenas por `netsh interface portproxy`. A afirmação foi registrada por engano e é corrigida aqui (histórico preservado conforme AGENTS.md §4.3).
- **Nota de correção à entrada [2.0.0] — o "bind 0.0.0.0" NUNCA funcionou**: o item "Suporte a bind em `0.0.0.0` com fallback gracioso para `127.0.0.1`" era **falso**. O motor oficial do projeto Cua escuta **somente em `127.0.0.1`**: o endereço é fixo no código deles (`([127,0,0,1], port)` em `mcp_http.rs`) e **não existe variável de bind no upstream**. Verificado de duas formas independentes: (1) a string `CUA_DRIVER_RS_MCP_HTTP_BIND` **não aparece** no binário oficial instalado (0.8.3); (2) a busca por essa variável no repositório `trycua/cua` retorna **zero** ocorrência. A variável só tinha efeito num motor com patch local, que não é o que o usuário executa. Consequência prática: **todo o acesso pela LAN que funcionava vinha do `portproxy`**, não do bind.
  - Corrigido nesta versão: o botão passou a se chamar **"Aplicar Porta"** (não promete mais o bind); a GUI **deixou de gravar** a variável morta e **remove** a que tenha sobrado no ambiente do usuário; o instalador **deixou de semeá-la** (com aviso no `.iss` para não reintroduzir); a tabela de variáveis do `SKILL.md` foi corrigida; e as mensagens de diagnóstico passaram a apontar o caminho verdadeiro (Encaminhamento para LAN, aba Túnel para internet).

### Segurança (motor)
- **O endpoint HTTP do motor passou a exigir token nas versões novas.** A série **0.16+** do `cua-driver` exige `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32–4096 caracteres) e responde **401** a qualquer POST sem `Authorization: Bearer <token>`, além de rejeitar requisições com origem de navegador. A versão pinada no instalador (**0.8.3**) **não tem** autenticação alguma — confirmado no binário. A GUI agora **envia o header quando há token configurado** (lido de `HKCU\Environment`), funcionando com as duas gerações do motor; sem isso, atualizar o motor faria o app reportar "MCP parado" com o motor perfeitamente vivo. O aviso da aba Túnel passou a refletir o **estado real** (com ou sem token) em vez de afirmar sempre que não há autenticação, e o snippet `mcpServers` inclui o header quando aplicável.

---

## [2.0.0] - 2026-07-28

### Adicionado & Aprimorado
- **Catálogo MCP Tools na GUI (`mcp_tools.rs`)**: Nova aba visual para listar, filtrar por categoria e invocar interativamente chamadas de visão e automação do motor CUA Driver.
- **Mecanismo de Auto-Upgrade de Versão**: Botão *Verificar Atualizações* na GUI que consulta o GitHub Releases API, baixa o instalador em **background** para `%TEMP%` com **verificação SHA256 obrigatória** (o `.sha256` publicado pelo CI), pede confirmação para **fechar** o aplicativo, garante os processos encerrados, instala silenciosamente e reabre a GUI e o motor. Só atualiza se a release for **estritamente mais nova** (comparação semver) — rollback de release no GitHub nunca vira downgrade silencioso.
- **Regras Inteligentes de Firewall & PortProxy**:
  - `netsh interface portproxy` direciona obrigatoriamente para a porta CUA confirmada (`127.0.0.1:$CUA_PORT_CONFIRMADA`).
  - Aplicação de regras do Windows Firewall com checagem deduplicada de nomes para evitar regras duplicadas.
  - Suporte a bind em `0.0.0.0` com fallback gracioso para `127.0.0.1`.
- **Limpeza Automática no Instalador (Inno Setup)**: o `InitializeSetup()` do `installer/fzcomputerai.iss` encerra instâncias em execução (`fzcomputerai`, `cua-driver`), remove a scheduled task do daemon (`cua-driver-serve`), limpa registros de autostart legados em `HKCU\...\Run` e executa silenciosamente o desinstalador de versões anteriores antes de instalar a nova.
- **Console Debug Formatado**: Logs organizados com autoscroll e 2 linhas em branco de espaçamento após cada lote de execução. Console **sempre visível** como faixa fixa no rodapé da aba MCP & Rede; controles do daemon e de porta/IP fixos no topo; só o diagnóstico rola.
- **Diagnóstico honesto na aba MCP & Rede**: host e URL exibidos derivam do **estado real** (netstat + teste TCP), listeners reais impressos na tela (incluindo portas órfãs no IP da LAN) e regras portproxy existentes listadas cruas do `netsh show v4tov4`. Badge do encaminhamento com **3 estados**: REGRA FUNCIONANDO (config + listener confirmados), REGRA SEM EFEITO (config sem listener — dica do IP Helper) e SEM REGRA.
- **Encerramento limpo ao fechar a GUI**: o `on_exit` para o daemon `cua-driver` e remove as regras portproxy **criadas pelo próprio app** (registradas em `HKCU\Software\FzComputerAI`) — nunca regras de outros serviços com padrão parecido. Elevação única (UAC) apenas se necessária.
- **Workflow CI/Release Atualizado**: Compilação multiplataforma e publicação automatizada de instaladores e binários pré-compilados em releases com checksum `.sha256`.

### Removido
- **`install.ps1` da raiz do repositório** — o Windows agora instala **exclusivamente** pelo instalador gráfico
  Inno Setup (`fzcomputerai-setup-windows-x64.exe`, baixado dos releases ou gerado localmente com
  `ISCC.exe /DAppVersion=<versao> installer\fzcomputerai.iss`). Manter dois instaladores Windows duplicava
  lógica e abria margem para comportamentos divergentes. O `install.sh` permanece para Linux/macOS.
  *Não confundir com o `install.ps1` oficial do motor cua-driver (`cua/libs/cua-driver/scripts/install.ps1`),
  que continua existindo e é o que o instalador gráfico executa na task opcional do motor.*

### Corrigido
- **Nome de saída do instalador no `.iss`**: `OutputBaseFilename` fixado em `fzcomputerai-setup-windows-x64`
  (sem a versão no nome), para casar com o `INSTALLER_NAME` esperado pelo workflow de release — antes o job
  Windows falharia por não encontrar o arquivo na hora de publicar o asset.

---

### Segurança
- **Removida a autoassinatura de código do `install.ps1`** (função `Set-FzCodeSigning`, presente da v1.0.0 à v1.0.2).
  A função gerava um certificado auto-assinado `CN=FzComputerAI (Webstorage Tecnologia)` em `Cert:\CurrentUser\My`,
  **instalava esse certificado no store de Raiz Confiável do usuário** (`Cert:\CurrentUser\Root`) e assinava
  `fzcomputerai.exe` e `cua-driver.exe` com ele. Instalar uma raiz confiável na máquina de quem instala altera a
  configuração de segurança do usuário final e é a técnica clássica usada por malware para legitimar binários
  arbitrários; além disso, a assinatura resultante **não removia o aviso do SmartScreen**, porque o certificado não
  encadeia em nenhuma CA pública. **Quem executou a versão antiga do script deve seguir o procedimento de remediação
  descrito em `SIGNING.md` (seção 10, "Perguntas frequentes")** para localizar e remover o certificado dos stores
  `Root` e `My`.
- **Removido do workflow o step "Auto-Sign"**, que aplicava a mesma autoassinatura no runner do GitHub Actions.
  O comentário em `.github/workflows/build-release.yml` registra a remoção para que o step não seja reintroduzido.

### Adicionado
- **Instalador Windows (Inno Setup)**: `installer/fzcomputerai.iss` gera o `fzcomputerai-setup-windows-x64.exe`, com
  atalhos no Menu Iniciar, opção de iniciar com o Windows, instalação opcional do motor `cua-driver`, desinstalador
  registrado em *Aplicativos instalados* e upgrade in-place. **O instalador não é assinado e, portanto, exibe o mesmo
  aviso do SmartScreen que o `.exe` avulso** — contornar o SmartScreen nunca foi objetivo dele.
- **Script de assinatura local `scripts/sign-release.ps1`**: assina e verifica os artefatos com `signtool`, exigindo um
  certificado de code signing real (token USB / HSM) e carimbo de tempo RFC 3161. **Recusa-se a executar** se
  encontrar apenas certificados auto-assinados no store.
- **`SIGNING.md`**: documento de referência sobre assinatura de código, SmartScreen e distribuição — estado atual,
  custos e requisitos das opções reais de certificado, e as práticas que este projeto se recusa a adotar.
- **Console Debug na GUI**, execução de comandos sem janela de console, autostart e versão dinâmica a partir do
  `Cargo.toml`; CI com stamp de versão e `install.sh` via `curl | bash`.

---

## [1.0.2] - 2026-07-25

### Adicionado
- **Interface Gráfica (GUI) Nativa em Rust (`fzcomputerai`)**: Painel de controle completo com abas para configuração do servidor MCP, testes de calibração de tela e visão, gerenciador de janelas e processos, gravação de trajetória e diagnóstico (Doctor).
- **Autoassinatura de código (Authenticode) no `install.ps1`** — *recurso removido depois por questão de segurança; ver [2.0.0], seção "Segurança"*. O script passou a gerar um certificado auto-assinado `CN=FzComputerAI (Webstorage Tecnologia)`, a instalá-lo na Raiz Confiável do usuário (`Cert:\CurrentUser\Root`) e a assinar os binários com ele.
  > **Correção de registro:** esta entrada, como publicada originalmente, afirmava que o recurso eliminava os avisos do Windows Defender. **Isso era falso.** Um certificado auto-assinado não encadeia em nenhuma CA pública: o SmartScreen e o Defender continuam avisando exatamente igual. O texto original é mantido aqui apenas como registro histórico da promessa incorreta.
- **Workflow CI/CD Multiplataforma**: Configuração completa do GitHub Actions para compilação nativa em Windows, macOS e Linux a cada release de tag `v*`. A versão 1.0.2 incluía também um step "Auto-Sign" que aplicava a mesma autoassinatura no runner — *step removido depois; ver [2.0.0], seção "Segurança"*.
- **Pacote NPM**: Publicação global via `npm install -g fzcomputerai`.

### Corrigido
- Correção no workflow do GitHub Actions (`fail-fast: false`, remoção de `submodules: recursive` desnecessário e adição de dependências Linux).

---

## [1.0.1] - 2026-07-24

### Corrigido
- Ajustes de pipeline no GitHub Actions.

---

## [1.0.0] - 2026-07-24

### Adicionado
- **Integração de Visão Computacional MCP**: Suporte nativo ao Model Context Protocol (MCP) permitindo que agentes de IA inspecionem visualmente o desktop e controlem a UI em tempo real.
- **Ferramentas de Inspeção Visual Multimodal**: `get_desktop_state`, `get_window_state`, `take_screenshot`.
- **Ferramentas de Controle de Ponteiro & Teclado**: `mouse_click`, `mouse_move`, `keyboard_type`, `shortcut`, etc.
- **Suporte a Transporte HTTP TCP/IP Nativo**: Configuração da porta `8000` via `CUA_DRIVER_RS_MCP_HTTP_PORT` para orquestradores remotos como FazAI-NG.
- **Documentação Multilíngue**: Guias de instalação e uso completos em Português e Inglês.
- **Scripts de Instalação Automatizados**: `install.ps1` e `install.sh`.
- **Seção de Patrocinadores Oficiais**: Inclusão de Webstorage Tecnologia e Imóvel Site.
