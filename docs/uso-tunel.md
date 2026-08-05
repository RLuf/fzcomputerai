# Aba Túnel — expor o MCP na internet

Para quem precisa que um agente fora da sua rede alcance este computador, e quer saber exatamente o que está abrindo.

Leia o aviso primeiro: o endpoint MCP dá controle de **mouse, teclado e tela** desta máquina. Quem alcança a URL e passa pela autenticação existente controla o computador. Uma URL aleatória **não é** autenticação — é só uma URL difícil de adivinhar, e ela vaza em log, histórico de navegador, print de tela e arquivo de configuração de cliente MCP.

Código: `fzcomputerai/src/tabs/tunnel.rs` (interface) e a seção "ABA TÚNEL" de `fzcomputerai/src/app.rs` (comportamento).

## 1. Como funciona no geral

O túnel é de **saída**: o binário do provedor roda nesta máquina, abre uma conexão para fora e recebe as requisições por ela. **Nenhuma porta de entrada é aberta** no roteador ou no firewall.

```text
cliente MCP na internet
        |  HTTPS
        v
  borda do provedor (Cloudflare / ngrok / servidor SSH)
        |  conexão de saída iniciada por esta máquina
        v
  cloudflared | ngrok | ssh -R   (processo filho desta GUI)
        |  HTTP em loopback
        v
  [porteiro de senha em 127.0.0.1:efêmera]   <- só se você definir senha
   (valida /s/<senha>/ e injeta o Bearer do motor)
        |
        v
  MCP do cua-driver em 127.0.0.1:<porta>
```

Um túnel por vez: um processo rastreado, uma URL pública, um snippet.

## 2. Comparação dos provedores

| | Cloudflare (quick) | Cloudflare (nomeado) | ngrok | SSH reverso |
| --- | --- | --- | --- | --- |
| **Conta necessária** | não | sim: login OAuth + criar túnel + DNS, tudo pela aba (ou token do túnel do painel) | sim, com authtoken **válido** | servidor próprio: acesso a ele. Presets públicos: não |
| **Tipo de URL** | aleatória `*.trycloudflare.com`, muda a cada início | hostname fixo do **seu** domínio (ex.: `mcphome.seudominio.com.br`), sobrevive a reinícios | `*.ngrok-free.app` / `*.ngrok.app` (aleatória no plano gratuito) | `*.lhr.life` (localhost.run) ou `*.serveousercontent.com` (serveo); no seu servidor, o que você definir |
| **Autenticação possível** | senha na URL (porteiro local) | Cloudflare Access, no painel Zero Trust, **ou** senha na URL | basic-auth via *traffic policy* gerada pela GUI, **ou** senha na URL | o que o seu servidor impuser (proxy com auth, mTLS…), **ou** senha na URL |
| **Binário** | `cloudflared` (Apache-2.0) | `cloudflared` | `ngrok`, proprietário da ngrok Inc. | `ssh.exe` do Cliente OpenSSH do Windows |
| **Limites conhecidos** | serviço sem SLA para quick tunnels; a URL some ao reiniciar | conforme o seu plano Cloudflare | plano gratuito limitado — o modal de termos do app cita, por exemplo, 20.000 requisições/mês e 1 GB de tráfego; **confirme em ngrok.com/tos, pode mudar** | presets públicos não dão garantia de uptime; `BatchMode=yes` exige chave |
| **Quando escolher** | teste rápido, sessão curta | uso recorrente com domínio e autenticação de borda | você já usa ngrok e quer basic-auth pronta | você tem servidor próprio e quer controle total do caminho |

A GUI **não** classifica nenhuma dessas opções como "segura" por si. Segurança aqui vem do que você põe na frente do MCP: senha na URL, autenticação de borda, ou token do motor `0.16+` — a seção seguinte explica este último.

## 3. Token do motor (cua-driver 0.16+)

A camada de autenticação mais forte não fica na borda nem no porteiro: fica no próprio motor.

- **Motores `0.16+`** exigem um token na variável de ambiente `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32–4096 caracteres) para o endpoint HTTP `/mcp`. Sem o header `Authorization: Bearer` correto, a resposta é **HTTP 401** com corpo JSON-RPC (`{"error":{"code":-32001,"message":"Authentication required"}}`).
- **Sem nenhum token configurado, o endpoint é fail-closed**: 401 para **tudo** — não "aberto". O túnel sobe, a URL pública existe, mas nenhum cliente entra.
- **Motores antigos (`<=0.8.x`)** não têm autenticação: endpoint aberto.

A GUI lê o token de `HKCU\Environment` no início do app **e ao abrir a aba Túnel** (novidade da 2.2.0), envia-o como Bearer em toda sonda, e o valor **nunca aparece em log** — a 2.2.0 corrigiu uma falha em que a leitura do registro logava o valor junto com a saída do `reg query`.

### O aviso em 4 estados

No topo da aba, um quadro vermelho descreve a situação real do motor:

| Estado | O que o quadro diz |
| --- | --- |
| **(a)** token configurado e aceito | quem tiver a URL **e** o token controla esta máquina — trate os dois como segredo |
| **(b)** token conhecido **recusado** (401 mesmo com Bearer) | o token do registro não é o que o motor usa; gere um novo |
| **(c)** motor exige token e **não há nenhum** em `HKCU` (fail-closed) | o túnel sobe mas **nenhum cliente entra**; o snippet sairia sem `Authorization`. Gere o token **antes** de iniciar |
| **(d)** motor antigo, endpoint aberto | quem tiver a URL controla mouse, teclado e tela |

![Estado fail-closed: aviso vermelho e o botão "Gerar e ativar token do motor"](../assets/img/screenshot-tunel-token.png)

### Botão "Gerar e ativar token do motor"

Nos estados (b) e (c), no Windows, o quadro traz o botão **Gerar e ativar token do motor**. Ele:

1. gera um token CSPRNG (`RNGCryptoServiceProvider` via PowerShell, mínimo de 32 caracteres);
2. grava em `HKCU\Environment` com confirmação de escrita — sem logar o valor;
3. exporta o token ao ambiente do próprio processo e reinicia o daemon pelo ciclo de vida da propria GUI (Parar + Iniciar o processo filho; nunca `autostart kick`).

Depois disso, o **Copiar snippet** passa a incluir o header `"Authorization": "Bearer <token>"` no bloco `mcpServers` — o cliente MCP já sai configurado para autenticar.

## 4. Antes de iniciar qualquer túnel

O `Iniciar túnel` executa pré-checagens honestas e **recusa** em vez de subir algo inútil:

1. **Porta MCP confirmada.** Roda `detect_confirmed_cua_port()`: nenhuma URL pública é publicada apontando para porta morta. Se o MCP local não responder, a mensagem manda iniciar o motor na aba MCP & Rede.
2. **Binário presente.** O caminho resolvido aparece na tela com ponto verde; ausente, aparece "binário: NÃO ENCONTRADO" e o botão de download.
3. **Pré-checagem do provedor** (ver cada seção abaixo).

## 5. Passo a passo por provedor

### 5.1 Cloudflare — quick tunnel (sem conta)

1. Selecione **Cloudflare**. Se o binário não for encontrado, clique **Baixar cloudflared**. O download usa `winget install --id Cloudflare.cloudflared --installer-type portable` e, se isso não produzir o executável, cai para o release oficial do GitHub; o console registra o **SHA256** e o status do Authenticode do arquivo obtido.
2. Deixe o campo de token vazio — vazio significa modo QUICK.
3. Clique **Iniciar túnel**. No modal, opcionalmente defina uma senha (seção 6).
4. O processo sobe como:

```text
cloudflared --no-autoupdate --loglevel info --logfile <%TEMP%\fzcomputerai-tunnel\cloudflare-<run_id>.log> tunnel --url http://127.0.0.1:<porta>
```

5. O status vai para **INICIANDO** e a GUI passa a ler o log do próprio `cloudflared`. Quando aparece uma URL terminada em `.trycloudflare.com`, ela é capturada, o status vira **ATIVO** e a URL completa fica copiável.

### 5.2 Cloudflare — túnel nomeado com domínio próprio (URL fixa)

Novidade da 2.4.0: o fluxo inteiro cabe na aba. Não é preciso ir ao painel criar o túnel nem escrever o registro DNS à mão.

**Pré-requisito honesto:** o domínio precisa estar **na sua conta Cloudflare**, com os nameservers delegados a ela. Sem isso, o passo de DNS falha — e a GUI diz exatamente isso, em vez de deixar você achando que subiu.

1. **Login Cloudflare (OAuth)** — executa `cloudflared tunnel login` e abre o navegador; escolha o domínio e autorize. **O login sozinho não cria nada:** ele só grava `~/.cloudflared/cert.pem`, que autoriza a conta *nesta máquina*. Quem parava aqui ficava sem URL e achava que o login tinha falhado — por isso os dois passos seguintes existem.
2. **Verificar login** — a GUI não presume: ela procura o arquivo `cert.pem` em `%USERPROFILE%\.cloudflared\` e responde "conta autorizada" ou "sem autorização".
3. Preencha **Nome do túnel** (padrão `fzcomputerai`) e **Hostname público** (ex.: `mcphome.seudominio.com.br`). A URL final aparece ao lado, já montada: `https://<hostname>/mcp`.
4. **Criar túnel + apontar DNS** — roda, em segundo plano, `cloudflared tunnel create <nome>` e depois `cloudflared tunnel route dns <nome> <hostname>`. Se o túnel já existir, o `create` diz isso e o fluxo segue para a rota — que é o passo que de fato publica o hostname.
5. **Iniciar túnel**. O processo sobe como:

```text
cloudflared --no-autoupdate --loglevel info --logfile <log> tunnel run --url http://127.0.0.1:<porta> <nome>
```

Como o hostname é fixo, a URL pública é conhecida **antes** de subir: a GUI a preenche sozinha, sem depender de capturar nada no log. A credencial é o `cert.pem` mais o JSON do túnel em `~/.cloudflared` — nenhum segredo vai para o argv.

#### Alternativa: token do túnel criado no painel

Se você prefere criar o túnel no painel Zero Trust, cole o **token do túnel** no campo (é campo de senha) e clique **Salvar token**: ele vai para um arquivo com ACL restrita e a GUI passa a usar **só o caminho** dele — o token não vai para argv, log nem registro. O processo sobe como `cloudflared --no-autoupdate --logfile <log> tunnel run --token-file <caminho>`. Nesse modo o log **não imprime URL**: informe o hostname no campo **URL pública** à mão; ao ter URL, o status é promovido para **ATIVO**.

Com um token salvo, ele **tem precedência** sobre o par nome + hostname. Para voltar ao fluxo por nome de túnel — ou ao quick tunnel — use **Esquecer token**.

Autenticação nesse modo: **Cloudflare Access**, configurado no painel — ele roda na borda, antes de chegar à sua máquina —, o token do motor (seção 3) e/ou a senha na URL (seção 6).

### 5.3 ngrok

1. Crie a conta e configure o authtoken **no seu terminal** (a GUI não pede sua credencial):

```powershell
ngrok config add-authtoken <SEU_TOKEN>
```

A aba tem um botão que copia esse comando.

2. Se o binário faltar, **Baixar ngrok** abre primeiro um modal de **termos** — o ngrok é binário proprietário da ngrok Inc., e o download (fonte oficial, via `winget Ngrok.Ngrok` com fallback para o zip oficial) só acontece após o seu aceite, que fica registrado em `HKCU\Software\FzComputerAI`.
3. Opcional: marque **Proteger com basic-auth (traffic policy gerada)**. A GUI gera uma senha e adapta o comando à versão do seu agente — ela pergunta ao próprio binário (`ngrok http --help`) se a flag `--traffic-policy-file` existe:
   - **agente 3.9+**: escreve um `ngrok-policy.yml` com ACL restrita e passa `--traffic-policy-file` (como antes);
   - **agente antigo (ex. 3.3.x)**: fallback automático para `ngrok start fz-mcp` com uma config v2 gerada (basic_auth em arquivo com ACL restrita), mesclada à config padrão para preservar o authtoken. Antes da 2.2.0, o spawn morria com "unknown flag" e a aba mostrava ERRO.

   A credencial exibida (`fz:<senha>`) precisa ser guardada — o cliente MCP tem de enviá-la.

   **Conflito com o token do motor:** a basic-auth de borda e o Bearer do motor usam o **mesmo** header `Authorization`, e o cliente MCP só envia um. Com token do motor ativo, a GUI **ignora** a basic-auth de borda no start (com nota na UI e no log) — a proteção do túnel passa a ser o Bearer do motor.
4. **Pré-checagem:** antes de subir, a GUI roda `ngrok config check`. Falhou ⇒ o túnel **não** inicia e a mensagem diz exatamente o que fazer (configurar o authtoken). Atenção: essa checagem só valida a **sintaxe** da config — um authtoken inválido passa nela e o processo morre depois, ao autenticar, com **ERR_NGROK_105** no log. A GUI detecta esse código e explica a correção: rode `ngrok config add-authtoken <TOKEN>` com o token real de dashboard.ngrok.com.

   **Onde a chave mora, verificado:** o authtoken **não** fica no registro do Windows — varredura de `HKCU` e `HKLM` não encontrou nada. Ele vive em `%LOCALAPPDATA%\ngrok\ngrok.yml`, que é o arquivo que o `add-authtoken` escreve.

   **Estado medido na máquina de teste desta versão:** o authtoken presente nesse `ngrok.yml` era **inválido** — o `ngrok config check` passou (a sintaxe estava certa) e o processo morreu ao autenticar, com `ERR_NGROK_105`. Ou seja: o caminho do ngrok na GUI não está quebrado; o que faltava era **credencial válida**. Com um token válido, o fluxo é o descrito acima. Diferente do Cloudflare quick tunnel, este provedor não foi exercitado ponta a ponta nesta versão.
5. O processo sobe como `ngrok http 127.0.0.1:<porta> --log <log> --log-format logfmt --log-level info [--traffic-policy-file ...]` — ou, no fallback de agente antigo com basic-auth, como `ngrok start fz-mcp` com as configs geradas.
6. Se a URL não aparecer no log, use **Descobrir URL (API local 4040)**: a GUI consulta `http://127.0.0.1:4040/api/tunnels`.
7. Clientes MCP não são navegadores, então o *interstitial* do ngrok normalmente não afeta. Se afetar, envie o header `ngrok-skip-browser-warning: 1` (a sonda de exposição da GUI já envia).

### 5.4 SSH reverso

1. Selecione **SSH reverso**. O `ssh.exe` vem do Cliente OpenSSH do Windows.
2. Escolha o destino:
   - **servidor próprio** — `usuario@seu.servidor`, com a porta remota que você quer publicar. É a opção com mais controle: você decide o proxy, a autenticação e os logs;
   - **presets públicos** — botões `localhost.run` (preenche `nokey@localhost.run`, porta 80) e `serveo.net` (porta 80). Sem garantia de uptime.
3. Preencha **Chave (-i)** se precisar de uma chave específica.
4. O processo sobe com as opções fixas abaixo (mais o `-R <porta_remota>:127.0.0.1:<porta_local>` e seus args extras):

```text
ssh -N -T -E <log> -o BatchMode=yes -o StrictHostKeyChecking=accept-new
    -o ExitOnForwardFailure=yes -o ConnectTimeout=10
    -o ServerAliveInterval=30 -o ServerAliveCountMax=3
    -R <porta_remota>:127.0.0.1:<porta_local> <destino>
```

`BatchMode=yes` é deliberado: sem ele, um `ssh` esperando senha ficaria pendurado invisível, sem janela onde digitar. A consequência é que **autenticação por senha não funciona** aqui — use chave.

5. A URL pública é extraída do log do `ssh` quando termina em `.lhr.life`, `.serveousercontent.com` ou `.serveo.net`. Com servidor próprio, informe a URL à mão.

## 6. Nível 1 de autenticação: senha na URL

O MCP do `cua-driver` aceita `POST` em **qualquer** caminho e (nas versões sem token) não valida credencial. O quick tunnel do Cloudflare e os serviços SSH públicos não têm autenticação de borda. Logo, "senha na URL" só é real com um **porteiro** no meio — e é isso que a GUI sobe.

Ao definir senha no modal de início:

- um mini reverse-proxy sobe em `127.0.0.1`, numa **porta efêmera**, e o túnel aponta para **ele**, não para o MCP;
- o porteiro exige `/s/<senha>/` no início do caminho. Sem isso, responde **404** e nunca toca o MCP;
- a senha é reduzida a caracteres URL-safe (`A-Z a-z 0-9 - . _ ~`) antes do uso. Se tivesse espaço, `#`, `%` ou acento, o cliente percent-encodaria o caminho e o porteiro responderia 404 a **tudo**, com uma URL de aparência válida na tela. O ajuste é avisado no console;
- **Gerar** cria uma senha de 16 caracteres. Ela vem de um gerador pseudoaleatório simples semeado pelo relógio (`xorshift`, sem a crate `rand`) — bom o bastante para senha de URL, **não** é material criptográfico. Se o seu modelo de ameaça exige mais, gere a senha em outro lugar e cole;
- a senha **não é persistida** e **não aparece no console**: qualquer texto que vá para log ou UI tem o segmento substituído por `/s/***/`. Ela vive só em memória, por sessão de túnel.

A URL final fica assim:

```text
https://exemplo-aleatorio.trycloudflare.com/s/Kx7fQ2mB9tLpZ4vR/mcp
```

E o snippet que a GUI monta para o cliente MCP (botão **Copiar snippet**):

```json
{
  "mcpServers": {
    "fzcomputerai": {
      "type": "http",
      "url": "https://exemplo-aleatorio.trycloudflare.com/s/Kx7fQ2mB9tLpZ4vR/mcp"
    }
  }
}
```

Com o token do motor ativo (seção 3), o snippet traz também o header `Authorization: Bearer`. As duas camadas coexistem sem conflito: a senha viaja no **path**, o token no **header**.

Comportamento verificado em teste real do porteiro: **sem senha -> 404**, **senha errada -> 404**, **senha correta -> resposta JSON-RPC 200**. Se o MCP local estiver fora, o porteiro responde **502**.

Detalhe técnico que evita um bug clássico: o porteiro descarta o `Connection` do cliente e força `Connection: close` para o MCP. Sem isso, o keep-alive do HTTP/1.1 deixaria a conexão aberta até o timeout de 30 s e o cliente perderia a requisição seguinte. Cada conexão atende uma requisição.

### A URL com senha dispensa o header (o porteiro injeta o Bearer)

Novidade da 2.4.0, e ela existe por um problema concreto: clientes como o **Claude Desktop** aceitam **só uma URL** — não há campo onde colar `Authorization`. Com motor `0.16+`, que é fail-closed, esses clientes tomariam 401 em tudo e o túnel seria inútil para eles.

Como o porteiro resolve: quem provou a senha no caminho (`/s/<senha>/mcp`) **já está autenticado perante o app**, então o porteiro acrescenta o `Authorization: Bearer <token do motor>` ao falar com o motor, ali em `127.0.0.1`. As consequências, ditas sem maquiagem:

- o segredo do motor **não viaja pela internet**. A credencial pública passa a ser a **senha da URL** — que é sua, não é persistida e muda quando você reinicia o túnel com outra;
- se o cliente mandar o **próprio** `Authorization`, o dele vence: a injeção não sobrescreve nada;
- em troca, **essa URL completa passa a ser suficiente para controlar a máquina**, sem mais nenhum segredo. Trate-a como senha: ela vaza em log, histórico de navegador, print de tela e arquivo de configuração de cliente MCP.

Verificado pela internet, com a URL de senha e **nenhum header**: `initialize` OK e `tools/call get_screen_size` executando de verdade (4096x2160). Senha errada e senha ausente continuam devolvendo **404** — a injeção não afrouxa o porteiro, ela só age depois que a senha passou.

Nesse arranjo o snippet do cliente é o do bloco acima, sem bloco `headers` — e é o que basta.

## 7. Sonda de exposição — "Testar pela internet"

Nunca presuma que o túnel está protegido. O botão **Testar pela internet** faz um `POST` `initialize` real na URL pública, saindo pela internet, usando `curl.exe` (o `TcpStream` da GUI não faz TLS). A URL e o header `Authorization: Bearer` vão num arquivo de configuração do `curl` (`--config`), não no argv — o argv de qualquer processo é legível por outros processos, e a URL contém a senha. Os arquivos temporários são apagados depois do teste.

Desde a 2.4.0 a sonda roda em **segundo plano**. Antes eram dois `curl -m 20` executados na thread da interface: até 40 segundos com a janela congelada e o Windows escrevendo "(Não Respondendo)" no título — este era o pior caso do app inteiro. Agora a janela continua respondendo e o resultado é aplicado quando chega.

Desde a 2.2.0 a sonda tem **duas fases**:

1. **Fase 1 — sem nenhuma credencial.** Prova o que um desconhecido com a URL consegue fazer.
2. **Fase 2 — com `Authorization: Bearer`.** Só roda quando a GUI conhece o token do motor e a fase 1 **não** acusou exposição. Repete o mesmo `POST` `initialize`, agora autenticado. Se responder 200 com `"result"`, a prova é ponta a ponta: sem credencial barra, com credencial funciona.

O resultado é classificado só a partir do que a rede respondeu:

| Badge | Como é decidido | O que fazer |
| --- | --- | --- |
| **EXPOSTO SEM AUTENTICAÇÃO (verificado agora)** | fase 1 respondeu HTTP 200 com `"result"` sem nenhuma credencial | qualquer pessoa com a URL controla esta máquina. Pare o túnel ou coloque senha/token/Access |
| **MOTOR EXIGIU TOKEN (verificado: HTTP 401 sem Bearer)** | fase 1 respondeu 401 **com** corpo JSON-RPC — é o motor `0.16+` barrando | a proteção do próprio motor está confirmada; se a fase 2 não rodou, confira se a GUI conhece o token (seção 3) |
| **PROTEGIDO E FUNCIONAL (sem credencial: barrado; com Bearer: initialize OK)** | fase 1 barrou **e** a fase 2, com Bearer, obteve `initialize` 200 | prova ponta a ponta — o melhor resultado. Copie o snippet e use |
| **BORDA EXIGIU AUTENTICAÇÃO (verificado: HTTP nnn)** | HTTP 401, 403, 302 ou 407 **sem** corpo JSON-RPC | há autenticação na frente. Confirme que é a **sua** (Access, basic-auth, porteiro) e não uma página de erro do provedor |
| **NÃO FOI POSSÍVEL VERIFICAR — trate como exposto** | timeout, 5xx, falha do `curl` | não deu para provar nada. A postura honesta é assumir exposto até verificar |

Correção importante da 2.2.0: a sonda antiga marcava **EXPOSTO** para qualquer resposta contendo `"jsonrpc"`. Só que o 401 do motor `0.16+` **também** tem corpo JSON-RPC — o resultado era alarme falso num túnel protegido. Agora exposição exige 200 com `"result"`, e o 401 JSON-RPC vira o badge verde **MOTOR EXIGIU TOKEN**.

![Aba Túnel com túnel Cloudflare ativo e badge PROTEGIDO E FUNCIONAL](../assets/img/screenshot-tunel.png)

O badge só aparece **depois** que você roda a sonda. Enquanto isso, a tela não afirma nada sobre exposição. E o status **ATIVO** significa apenas "URL pública publicada" — não "verificado pela internet"; são estados separados de propósito.

Em qualquer seção do app, quando há túnel ativo, um chip laranja **TUNEL ATIVO** aparece no pé da barra lateral. A máquina estar exposta não é informação de uma aba só.

### Teste de fora da rede: `scripts/remote-teste.py`

A sonda prova que a URL responde. Para provar que ela **controla a máquina**, a 2.4.0 traz um script feito para rodar em **outro computador, pela internet**, só com a biblioteca padrão do Python 3 — nada para instalar:

```bash
python remote-teste.py https://mcphome.seudominio.com.br/s/<senha>/mcp
python remote-teste.py https://exemplo.trycloudflare.com/mcp --token <TOKEN> --termo "Roger Luft"
```

O que ele faz, nesta ordem: `initialize`, `tools/list`, abre uma janela **nova** de navegador na máquina remota (nunca sequestra uma janela já aberta), navega para `search.yahoo.com`, digita o termo, descobre e clica no botão de pesquisa (Search / Pesquisar / Buscar) — ou envia Enter, se não achar o botão — e **confere o resultado lendo a tela de volta**.

Se a URL já tem senha (`/s/<senha>/mcp`), o `--token` é dispensável: é o porteiro que injeta o Bearer (seção 6). O script reconfigura a própria saída para UTF-8 — a resposta do motor tem emoji e o console `cp1252` do Windows quebraria antes de mostrar o resultado.

## 8. Ciclo de vida: o túnel nunca sobrevive ao app

São camadas independentes, porque uma só falharia nos casos interessantes:

| Camada | Cobre |
| --- | --- |
| **Job Object do Windows** (desde a 2.3.0) | tudo de uma vez: X na janela, **Sair** na bandeja, `taskkill /F`, crash, logoff. É a garantia do **kernel** — no Windows um filho *não* morre com o pai (isso é comportamento de Unix), então o processo do túnel é **adotado** por um job criado antes de qualquer spawn, e o sistema o derruba quando a GUI termina de qualquer forma |
| `Child::kill()` no `on_exit` | fechamento normal da janela |
| **watchdog** PowerShell — hoje um **fallback** | disparado **só quando a adoção no job falha**. Com o job ativo, ele seria um PowerShell por túnel fazendo o que o kernel já faz melhor. Sem job, volta a ser a única rede contra túnel órfão: fica vigiando o PID da GUI e mata o túnel quando ela desaparece |
| bloco no `shutdown_cleanup` | processo auxiliar destacado que também encerra túneis registrados e limpa o registro |
| **reconciliação na abertura** | ao abrir, a GUI procura rastros `tunnel:*` de sessões anteriores; se o processo ainda estiver vivo, mata e registra no console que **a máquina esteve exposta até agora** |

Medido nesta versão: com GUI, motor e relay da LAN de pé, um `taskkill /F` na GUI derrubou motor e relay junto — e o `cua-driver` de **outro** cliente MCP, que não era filho desta GUI, ficou intacto. Em outro teste, o `cloudflared` também morreu junto com a GUI.

Um processo só é morto com **identidade de 3 fatores**: imagem (`cloudflared.exe`/`ngrok.exe`/`ssh.exe`) + `CreationDate` do processo + o marcador `run_id` presente na linha de comando (ele aparece no caminho do arquivo de log passado ao CLI). Isso elimina PID reciclado e, principalmente, protege o `cloudflared`/`ngrok`/`ssh` legítimo do usuário. **`taskkill /IM` é proibido no projeto** exatamente por isso.

O **Parar túnel** também não presume: mata a árvore (`taskkill /PID <pid> /T /F` antes do `wait()`), confirma ausência por identidade, encerra o porteiro e limpa o registro. Se o processo continuar vivo, o status vai para **ERRO** com aviso — não para "PARADO".

Ao desinstalar o app, `{app}\tunnel` (binários baixados, token-file, policy do ngrok) é removido.

## 9. Solução de problemas

| Sintoma | Causa | Correção |
| --- | --- | --- |
| Cloudflare quick tunnel não sobe; a GUI recusa antes de tentar | existe `%USERPROFILE%\.cloudflared\config.yaml`; com esse arquivo presente, o quick tunnel falha | renomeie/mova o arquivo, **ou** passe a usar o túnel nomeado (token-file) |
| Fez o Login do Cloudflare e não apareceu túnel nem hostname nenhum | o `tunnel login` **só** grava o `cert.pem` — ele não cria túnel nem DNS | **Verificar login** e depois **Criar túnel + apontar DNS**: são esses dois passos que criam de fato (seção 5.2) |
| **Criar túnel + apontar DNS** falha no passo do DNS | o domínio do hostname não está na sua conta Cloudflare (nameservers não delegados a ela) | leve o domínio para a conta usada no login e repita; a saída crua do `cloudflared` fica no console |
| ngrok recusa antes de subir, mensagem sobre authtoken | `ngrok config check` falhou — sem authtoken configurado | `ngrok config add-authtoken <SEU_TOKEN>` no seu terminal (conta em ngrok.com) e tente de novo |
| ngrok passa na pré-checagem mas morre logo depois, com **ERR_NGROK_105** no log | authtoken inválido — o `ngrok config check` só valida sintaxe, então deixa passar, e o processo morre ao autenticar | `ngrok config add-authtoken <TOKEN>` com o token real de dashboard.ngrok.com (a GUI detecta o código e mostra essa instrução) |
| Túnel **ATIVO**, mas todo cliente recebe **401** | motor `0.16+` fail-closed: não há token em `HKCU\Environment`, ou o snippet do cliente está sem o header `Authorization` | clique **Gerar e ativar token do motor** (seção 3) e copie o snippet de novo — ele passa a incluir o Bearer |
| O cliente MCP só aceita uma URL (Claude Desktop e afins) e toma **401** | não há onde colar `Authorization` na interface dele, e o motor `0.16+` exige Bearer | inicie o túnel **com senha** e entregue a URL `/s/<senha>/mcp`: o porteiro injeta o Bearer por você (seção 6) |
| SSH sai imediatamente pedindo autenticação | `BatchMode=yes` impede prompt de senha, por design | use chave (`-i`) ou um destino que aceite chave/`nokey` |
| Status fica **INICIANDO** e a URL não aparece | o CLI ainda não imprimiu a URL, ou o sufixo não é reconhecido (túnel nomeado, servidor próprio) | ngrok: **Descobrir URL (API local 4040)**. Cloudflare nomeado / SSH próprio: informe a URL à mão no campo |
| URL pública responde **404** | é o porteiro de senha: o caminho não tem `/s/<senha>/` correta | use a URL completa que a GUI mostra (**Copiar URL**). Senha errada e ausência de senha produzem o mesmo 404, de propósito |
| URL pública responde **502** | o porteiro/borda está de pé mas o MCP local não responde: motor parado, ou a porta mudou depois que o túnel subiu | verifique o badge na aba MCP & Rede, religue o motor e reinicie o túnel |
| Status vai para **ERRO** com trecho de log | o processo do túnel saiu sozinho | leia o final do log no console; conexão de rede, credencial expirada e limite de plano aparecem ali |
| Sonda diz **EXPOSTO SEM AUTENTICAÇÃO** e você esperava proteção | senha vazia no início do túnel, ou a autenticação de borda não está aplicada à rota | pare o túnel, reinicie com senha, e/ou configure Cloudflare Access / basic-auth do ngrok / token do motor (seção 3) |
| Sonda diz **NÃO FOI POSSÍVEL VERIFICAR** | timeout de 20 s, 5xx da borda ou `curl.exe` ausente | tente novamente; confirme `curl --version`; trate como exposto até provar o contrário |

## Ver também

- [acesso-remoto.md](acesso-remoto.md) — comparação túnel x LAN x VPN, e as implicações de segurança de cada caminho.
- [uso-mcp-rede.md](uso-mcp-rede.md) — o motor tem de estar respondendo em loopback antes de qualquer túnel.
- [atualizacao.md](atualizacao.md) — motor `0.16+` exige token e muda o comportamento do endpoint.
- [solucao-de-problemas.md](solucao-de-problemas.md) — problemas que não são exclusivos do túnel.
