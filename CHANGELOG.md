# Changelog

Todas as alterações notáveis do projeto **FzComputerAI / CUA Driver Computer Vision MCP** serão documentadas neste arquivo.

O formato baseia-se no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/spec/v2.0.0.html).

---

## [2.4.2] - 2026-08-05

### Corrigido — o travamento do arranque (a causa mais antiga) e uma regressão da 2.4.1

- **A janela demorava a aparecer, ou nem aparecia.** O construtor do estado roda **dentro** do `eframe::run_native`, antes do primeiro frame: tudo que demorasse ali segurava a janela em branco com "(Não Respondendo)". Ele fazia reconciliação de sobras (**um PowerShell por túnel rastreado**), sondagem de porta, subida do motor e leitura de tela pelo `cua-driver`. Medição desta máquina, que explica a gravidade: `reg` 74–79 ms, `netstat` 85 ms, `netsh` 137 ms, `where` ~180 ms — e **`powershell -NoProfile -Command 1` entre 72 e 161 segundos**. Agora o construtor faz só leitura barata de configuração, e o trabalho pesado acontece em `run_startup_tasks()`, **depois do primeiro frame**.
- **Regressão introduzida e corrigida na mesma sessão:** a primeira versão dessa mudança rodava `run_startup_tasks()` na thread da UI no segundo frame — e um frame que não termina é uma janela que nunca aparece. Foi exatamente o que o teste mostrou (processo vivo, 55 MB, sem janela). A limpeza de sobras e a leitura de tela passaram para o executor de segundo plano; sobrou na thread da UI apenas o que é rápido (`where`) e o `spawn` do motor.
- **Status falso após "Iniciar":** `start_daemon` sondava a porta logo depois do `spawn` e pintava "PARADO" (o motor leva 1–3 s para abrir o listener), ainda por cima sobrescrevendo o veredito que a tarefa assíncrona traria. A sondagem imediata saiu.

### Corrigido — documentação que contradizia o código (achado por revisão adversarial)

- `README.md` / `README_EN.md` ainda **ensinavam `cua-driver autostart kick`** no bloco de configuração da porta — o caminho removido na 2.3.0 e proibido pelo `AGENTS.md`. O mesmo em `docs/atualizacao.md` (três trechos) e `docs/uso-tunel.md` (fluxo do token).
- "com autostart no Windows" nos READMEs era lido como autostart **do motor**; agora diz explicitamente que o autostart é o **da GUI**.
- Cinco documentos afirmavam que o instalador fixa o motor **0.8.3** e que, portanto, a instalação padrão vem **sem token** — o oposto do que ele faz depois da correção da 2.4.0 (mínimo **0.16.0**, justamente a série que exige token).
- `docs/desenvolvimento.md` descrevia a task `cuadriver` e `postinstall skipifsilent` ("instalação silenciosa não instala o motor"), contradizendo o próprio `.iss` e outros quatro documentos: hoje é o **componente `engine`**, que roda **também** em `/VERYSILENT` e só é pulado com `/SKIPENGINE`.

---

## [2.4.0] - 2026-08-05

### Corrigido — a interface não trava mais (defeito antigo, causa medida)

- **Toda ação congelava a janela por segundos**, às vezes com "(Não Respondendo)" no título. A causa não era aleatória: cada handler chamava `Command::output()` **síncrono na thread da UI** — `reg query` ~200ms, `powershell` 300ms–2s, e o teste de exposição do túnel dispara `curl -m 20` **duas vezes** (até 40s parado). Agora existe um executor de segundo plano (`spawn_bg` / `poll_bg`, com `BgOutcome`/`BgEffect`): a tarefa roda em thread e o resultado é aplicado na thread da UI. Migrados os dois piores casos — o **teste pela internet** e a **espera pelo motor após o start** (eram até 12×400ms de `sleep`). Enquanto há tarefa em voo, a UI pede repaint a cada 200ms e diz que está trabalhando.
- **Correções encontradas por revisão adversarial do próprio código novo desta série:**
  - `stop_lan_relay()` **não parava o relay**: a conexão que destrava o `accept()` ia para `127.0.0.1:<porta>` e era atendida pelo **motor** (bind mais específico), não pelo relay — o socket seguia publicado na rede com o badge dizendo "parado". Agora a batida vai para o endereço em que o relay realmente escuta.
  - **Laço infinito do relay contra si mesmo**: com relay em `0.0.0.0:<porta>` e motor morto, conexões ao loopback passam a cair no próprio relay, que reconectaria em si mesmo até esgotar threads e handles. Agora a recursão é detectada e a conexão recusada.
  - **Token do motor ia no `argv` do PowerShell** ao ser gravado (violação direta do AGENTS.md: enquanto o processo vive, qualquer processo do mesmo usuário lê a linha de comando). Passa a ir pelo **stdin** (`powershell -Command -`).
  - O módulo do Job Object prometia status honesto que **não estava implementado** (`is_active()` sem chamador, retorno de `init()` descartado). Agora o arranque registra se a garantia de limpeza existe — ou avisa que **não** existe.
- **Instalador: parou de rebaixar o motor.** A comparação de versão era **textual** contra o pin `0.8.3`, e `'0.17.0'` é menor que `'0.8.3'` em ordem alfabética — numa máquina com o motor 0.17 o instalador concluía "versão errada" e reinstalava a antiga, derrubando junto o suporte a token do endpoint HTTP (que só existe de 0.16 em diante). Agora a comparação é **numérica**, o critério é "igual ou mais novo" e o mínimo declarado passou a ser **0.16.0**.

### Adicionado

- **Cloudflare com domínio próprio (URL fixa), fluxo completo pela GUI.** `cloudflared tunnel login` sozinho **não cria nada** — só baixa o `cert.pem`, e quem parava aí ficava sem URL achando que o login falhou. A aba Túnel agora faz o caminho inteiro: **Login** (OAuth no navegador) → **Verificar login** (confere de verdade se o `cert.pem` existe) → campos **Nome do túnel** e **Hostname público** → **Criar túnel + apontar DNS** (`tunnel create` + `tunnel route dns`, em segundo plano) → **Iniciar túnel** passa a rodar `cloudflared tunnel run --url http://127.0.0.1:<porta> <nome>`. O domínio precisa já estar na sua conta Cloudflare (nameservers delegados).
- **O porteiro (gate) passa a injetar o `Bearer` do motor.** Clientes como o Claude Desktop só aceitam **uma URL** — não há onde colar o header `Authorization` — e o motor 0.16+ é *fail-closed*, então esses clientes tomariam 401 e o túnel seria inútil para eles. Quem provou a senha no caminho (`/s/<senha>/mcp`) já está autenticado perante o app, então o porteiro acrescenta o header ao falar com o motor; se o cliente mandar o próprio `Authorization`, o dele vence. O segredo do motor **não viaja pela internet** — a credencial pública passa a ser a senha da URL. **Testado:** URL com senha e **sem nenhum header** → `initialize` OK e `tools/call get_screen_size` executou; senha errada → 404; sem senha → 404.
- **`scripts/remote-teste.py`** — teste de fora da rede, só com a biblioteca padrão do Python 3: `initialize`, `tools/list`, abre uma janela **nova** de navegador na máquina remota (nunca sequestra a que estiver em uso), vai ao Yahoo, digita o termo, descobre o rótulo real do botão de pesquisa (Search/Pesquisar/Buscar) ou envia Enter, e confere o resultado lendo a tela de volta. Uso: `python remote-teste.py <URL> [--token TOKEN] [--termo TEXTO]`. Com URL protegida por senha, o token é dispensável. Ele reconfigura a saída para UTF-8 porque a resposta do motor traz emoji e o console cp1252 do Windows quebraria o teste sem ter falhado — defeito encontrado ao rodar o script contra o túnel ao vivo.

### Notas honestas

- O **ngrok** desta máquina continua sem subir por **credencial**, não por código: o authtoken em `%LOCALAPPDATA%\ngrok\ngrok.yml` é inválido (`ERR_NGROK_105`), e `ngrok config check` passa porque valida só a sintaxe do arquivo. A chave do ngrok **não** fica no registro do Windows (varredura de `HKCU` e `HKLM` não encontrou authtoken algum).
- O instalador **não foi compilado nesta máquina**: o `ISCC.exe` do Inno Setup não está instalado aqui. As correções acima estão no `.iss` e serão compiladas pelo CI (ou localmente após instalar o Inno Setup).

---

## [2.3.0] - 2026-08-05

### Alterado (ciclo de vida — tudo passa a depender do processo principal)

- **O motor virou PROCESSO FILHO da GUI.** "Iniciar" deixou de chamar `cua-driver autostart kick` (tarefa agendada do Windows, processo independente que sobrevivia ao app) e passa a dar `spawn` em `cua-driver serve` com a porta/token no ambiente. Verificado no help do motor: `mcp` é stdio e **morre no EOF do stdin** (medido), então `serve` é o único modo que sustenta o endpoint HTTP — e o pipe `\\.\pipe\cua-driver` que ele abre é o canal que o próprio CLI usa (`call`, `status`, `stop`), não uma escolha nossa.
- **Morte-junto garantida pelo KERNEL (Job Object).** Novo `fzcomputerai/src/lifecycle.rs`: no arranque a GUI cria um Job Object com `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` e **adota** todo filho (motor e túnel). No Windows um filho **não** morre com o pai — `CreateProcess` não cria esse vínculo (isso é comportamento de Unix), e era por isso que existia um vigia em PowerShell. Agora, quando o processo da GUI termina **por qualquer via** (X, Sair na bandeja, `taskkill /F`, crash, logoff), o Windows destrói o job e mata os filhos. **Testado:** com GUI + motor filho + relay de pé, um `taskkill /F` na GUI derrubou motor e relay juntos, e o `cua-driver` de outro cliente MCP ficou intacto.
- **Watchdog PowerShell do túnel virou fallback.** Só é disparado quando a adoção pelo job falha — antes era um processo por túnel fazendo o que o kernel faz melhor.
- **REMOVIDO do fechamento: `taskkill /F /IM cua-driver.exe` e `cua-driver stop`.** Os dois encerravam o motor de **qualquer** origem, inclusive o daemon que um cliente MCP de terceiro tivesse subido — a proibição explícita do AGENTS.md (matar por imagem). Parar motor alheio agora é ação consciente no botão "Parar", nunca automática. A UI passa a distinguir e dizer quando o motor **não** é filho dela ("não será encerrado ao fechar o app").
- **"Aplicar Porta" não ressuscita mais o motor pela tarefa agendada**: reinicia o filho, se houver.

### Adicionado (rede)

- **Relay TCP interno substitui o `netsh portproxy` como caminho padrão de publicar na LAN.** Escuta em `0.0.0.0:<porta>` (ou num IP escolhido) e encaminha para `127.0.0.1:<porta do motor>`, copiando bytes nos dois sentidos sem inspecionar HTTP — keep-alive e streaming (SSE) passam intactos. Três ganhos, todos medidos: **não pede UAC**, **não deixa regra no sistema** (a do netsh sobrevive a reboot) e **morre com o app**. Medição que viabiliza tudo: nesta plataforma `0.0.0.0:8000` **coexiste** com o `127.0.0.1:8000` do motor, então publica-se na MESMA porta sem tocar na configuração do motor.
- Badge honesto de dois estados (PUBLICADO NA REDE / SÓ LOCAL) e **contador real de conexões** (ativas/total) — em vez do antigo limbo "regra existe mas pode não estar funcionando", que dependia do serviço IP Helper.
- A remoção de regras `portproxy` **legadas** continua disponível na UI, e só aparece quando existe alguma.
- **Testado de ponta a ponta pela LAN**: `initialize` OK, `tools/list` com 55 ferramentas e `tools/call get_screen_size` executando de verdade (`4096x2160 @ 1.75x`) via `http://192.168.0.101:8000/mcp` com `Authorization: Bearer`.

---

## [2.2.0] - 2026-08-05

### Corrigido (aba Túnel — todos os itens verificados em teste real na GUI)
- **Falso "EXPOSTO SEM AUTENTICAÇÃO" com motor 0.16+.** A sonda de exposição classificava como exposto qualquer resposta contendo `jsonrpc` — mas o **401 dos motores novos também contém `jsonrpc`** (é um erro JSON-RPC `Authentication required`). Um túnel corretamente protegido pelo token do motor era anunciado como aberto. Agora o **código HTTP decide primeiro**: 200 + `"result"` = exposto; 401 com corpo JSON-RPC = **MOTOR EXIGIU TOKEN**; 401/403/302/407 sem JSON-RPC = borda do provedor; resto = não verificável.
- **ngrok antigo morria no spawn com "unknown flag".** A GUI passava `--traffic-policy-file`, que só existe no agente 3.9+; no 3.3.x (instalado via winget em muitas máquinas) o processo saía na hora e a aba mostrava ERRO. Agora a GUI **pergunta ao próprio binário** (`ngrok http --help`) se a flag existe: agente novo usa a traffic policy em arquivo (como antes); agente antigo cai para `ngrok start` com **config v2 gerada** (`basic_auth` em arquivo com ACL restrita, mesclada ao config padrão que guarda o authtoken) — o segredo continua fora do argv.
- **Provedor errado no stop/limpeza.** Trocar o rádio de provedor com um túnel vivo fazia parada, confirmação de identidade, leitura de log e limpeza do HKCU usarem a imagem/slug do provedor **selecionado** em vez do provedor **em execução** (ex.: parar um Cloudflare com ngrok marcado tentava apagar `tunnel:ngrok:<pid>`). O provedor agora é **congelado no start** (`tunnel_run_provider`) e usado em todo o ciclo de vida.
- **Token do motor vazava no Console Debug.** `read_mcp_token` usava `run_logged`, que loga o stdout — e o stdout do `reg query` contém o valor do token. A leitura agora é direta e o log registra apenas o desfecho, mascarado (AGENTS.md §1.1: segredo nunca em argv/log).

### Adicionado (aba Túnel)
- **Sonda de exposição em DUAS FASES.** Fase 1 sem credencial (como antes); fase 2, quando a GUI conhece o token do motor, repete o `initialize` **com** `Authorization: Bearer` — se responder resultado, o badge vira **"PROTEGIDO E FUNCIONAL (sem credencial: barrado; com Bearer: initialize OK)"**: prova de ponta a ponta de que o túnel está protegido **e** utilizável. URL e Bearer viajam no arquivo `--config` do curl, nunca no argv.
- **Aviso de segurança em 4 estados reais** (combinando token em `HKCU\Environment` + o último probe local): token configurado e aceito; token **recusado** (401 com Bearer); motor **exige token e não há nenhum** (fail-closed — o túnel sobe mas nenhum cliente entra); motor antigo aberto. Antes o aviso dizia "sem token = endpoint aceita qualquer requisição", o que é **falso** nos motores 0.16+ (fail-closed).
- **Botão "Gerar e ativar token do motor"** (Windows), exibido nos dois estados 401: gera token com o **CSPRNG do sistema** (RNGCryptoServiceProvider via PowerShell, ≥32 chars), grava com confirmação em `HKCU\Environment` (sem passar por log), exporta para o ambiente do próprio processo (filhos herdam) e **reinicia o daemon** (`cua-driver stop` + `autostart kick`). O snippet `mcpServers` passa a incluir o header `Authorization: Bearer` automaticamente.
- **Diagnóstico dirigido `ERR_NGROK_105`**: authtoken inválido **passa** no `ngrok config check` (que valida só a sintaxe do arquivo) e o processo morre depois, ao autenticar. Quando o log do túnel contém `ERR_NGROK_105`, a mensagem de erro agora explica isso e dá o comando de correção (`ngrok config add-authtoken`).
- **Nota de conflito ngrok × token do motor**: basic-auth de borda e Bearer do motor usam o **mesmo header** `Authorization` — o cliente MCP só envia um. Com token do motor ativo, a borda é ignorada no start (logado no console) e a UI explica o porquê.
- A aba **relê o token do motor** na primeira abertura (antes só no startup da GUI — um token criado depois ficava invisível).

### Documentação
- Prints reais novos da aba Túnel: `assets/img/screenshot-tunel.png` (badge "PROTEGIDO E FUNCIONAL" após sonda em 2 fases) e `assets/img/screenshot-tunel-token.png` (estado fail-closed com o botão de geração de token).
- Varredura completa: READMEs (PT/EN), `docs/uso-tunel.md`, `docs/acesso-remoto.md`, `docs/solucao-de-problemas.md`, `docs/faq.md`, `docs/arquitetura.md` e `docs/uso-mcp-rede.md` atualizados para o fluxo de token e a sonda em 2 fases.

---

## [2.1.0] - 2026-08-02

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
