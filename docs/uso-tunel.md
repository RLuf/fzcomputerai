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
        |
        v
  MCP do cua-driver em 127.0.0.1:<porta>
```

Um túnel por vez: um processo rastreado, uma URL pública, um snippet.

## 2. Comparação dos provedores

| | Cloudflare (quick) | Cloudflare (nomeado) | ngrok | SSH reverso |
| --- | --- | --- | --- | --- |
| **Conta necessária** | não | sim (login OAuth ou token do túnel) | sim, com authtoken | servidor próprio: acesso a ele. Presets públicos: não |
| **Tipo de URL** | aleatória `*.trycloudflare.com`, muda a cada início | domínio fixo que você configura no painel | `*.ngrok-free.app` / `*.ngrok.app` (aleatória no plano gratuito) | `*.lhr.life` (localhost.run) ou `*.serveousercontent.com` (serveo); no seu servidor, o que você definir |
| **Autenticação possível** | senha na URL (porteiro local) | Cloudflare Access, no painel Zero Trust, **ou** senha na URL | basic-auth via *traffic policy* gerada pela GUI, **ou** senha na URL | o que o seu servidor impuser (proxy com auth, mTLS…), **ou** senha na URL |
| **Binário** | `cloudflared` (Apache-2.0) | `cloudflared` | `ngrok`, proprietário da ngrok Inc. | `ssh.exe` do Cliente OpenSSH do Windows |
| **Limites conhecidos** | serviço sem SLA para quick tunnels; a URL some ao reiniciar | conforme o seu plano Cloudflare | plano gratuito limitado — o modal de termos do app cita, por exemplo, 20.000 requisições/mês e 1 GB de tráfego; **confirme em ngrok.com/tos, pode mudar** | presets públicos não dão garantia de uptime; `BatchMode=yes` exige chave |
| **Quando escolher** | teste rápido, sessão curta | uso recorrente com domínio e autenticação de borda | você já usa ngrok e quer basic-auth pronta | você tem servidor próprio e quer controle total do caminho |

A GUI **não** classifica nenhuma dessas opções como "segura" por si. Segurança aqui vem do que você põe na frente do MCP: senha na URL, autenticação de borda, ou o token exigido pelo próprio motor (medido em 2026-08-03 no `cua-driver` 0.17.0: sem `Authorization: Bearer <token>`, a resposta é 401).

## 3. Antes de iniciar qualquer túnel

O `Iniciar túnel` executa pré-checagens honestas e **recusa** em vez de subir algo inútil:

1. **Porta MCP confirmada.** Roda `detect_confirmed_cua_port()`: nenhuma URL pública é publicada apontando para porta morta. Se o MCP local não responder, a mensagem manda iniciar o motor na aba MCP & Rede.
2. **Binário presente.** O caminho resolvido aparece na tela com ponto verde; ausente, aparece "binário: NÃO ENCONTRADO" e o botão de download.
3. **Pré-checagem do provedor** (ver cada seção abaixo).

## 4. Passo a passo por provedor

### 4.1 Cloudflare — quick tunnel (sem conta)

1. Selecione **Cloudflare**. Se o binário não for encontrado, clique **Baixar cloudflared**. O download usa `winget install --id Cloudflare.cloudflared --installer-type portable` e, se isso não produzir o executável, cai para o release oficial do GitHub; o console registra o **SHA256** e o status do Authenticode do arquivo obtido.
2. Deixe o campo de token vazio — vazio significa modo QUICK.
3. Clique **Iniciar túnel**. No modal, opcionalmente defina uma senha (seção 5).
4. O processo sobe como:

```text
cloudflared --no-autoupdate --loglevel info --logfile <%TEMP%\fzcomputerai-tunnel\cloudflare-<run_id>.log> tunnel --url http://127.0.0.1:<porta>
```

5. O status vai para **INICIANDO** e a GUI passa a ler o log do próprio `cloudflared`. Quando aparece uma URL terminada em `.trycloudflare.com`, ela é capturada, o status vira **ATIVO** e a URL completa fica copiável.

### 4.2 Cloudflare — túnel nomeado (domínio fixo)

1. Clique **Login Cloudflare (OAuth)** — isso executa `cloudflared tunnel login` e abre o navegador. Conclua a autorização e crie/roteie o túnel no painel Zero Trust (link na própria aba).
2. Cole o **token do túnel** no campo (é um campo de senha). Clique **Salvar token**: o token vai para um arquivo com ACL restrita e a GUI passa a usar **só o caminho** dele. O token não vai para argv, log nem registro. Para voltar ao quick tunnel, use **Esquecer token**.
3. O processo sobe como `cloudflared --no-autoupdate --logfile <log> tunnel run --token-file <caminho>`.
4. O túnel nomeado **não imprime URL** no log — informe o hostname no campo **URL pública** à mão. Ao ter URL, o status é promovido para **ATIVO**.
5. Autenticação recomendada nesse modo: **Cloudflare Access**, configurado no painel. Ela roda na borda, antes de chegar à sua máquina.

### 4.3 ngrok

1. Crie a conta e configure o authtoken **no seu terminal** (a GUI não pede sua credencial):

```powershell
ngrok config add-authtoken <SEU_TOKEN>
```

A aba tem um botão que copia esse comando.

2. Se o binário faltar, **Baixar ngrok** abre primeiro um modal de **termos** — o ngrok é binário proprietário da ngrok Inc., e o download (fonte oficial, via `winget Ngrok.Ngrok` com fallback para o zip oficial) só acontece após o seu aceite, que fica registrado em `HKCU\Software\FzComputerAI`.
3. Opcional: marque **Proteger com basic-auth (traffic policy gerada)**. A GUI gera uma senha, escreve um `ngrok-policy.yml` com ACL restrita e passa `--traffic-policy-file`. A credencial exibida (`fz:<senha>`) precisa ser guardada — o cliente MCP tem de enviá-la.
4. **Pré-checagem:** antes de subir, a GUI roda `ngrok config check`. Falhou ⇒ o túnel **não** inicia e a mensagem diz exatamente o que fazer (configurar o authtoken).
5. O processo sobe como `ngrok http 127.0.0.1:<porta> --log <log> --log-format logfmt --log-level info [--traffic-policy-file ...]`.
6. Se a URL não aparecer no log, use **Descobrir URL (API local 4040)**: a GUI consulta `http://127.0.0.1:4040/api/tunnels`.
7. Clientes MCP não são navegadores, então o *interstitial* do ngrok normalmente não afeta. Se afetar, envie o header `ngrok-skip-browser-warning: 1` (a sonda de exposição da GUI já envia).

### 4.4 SSH reverso

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

## 5. Nível 1 de autenticação: senha na URL

O MCP do `cua-driver` aceita `POST` em **qualquer** caminho e (nas versões sem token) não valida credencial. Nas versões com token — medido em 2026-08-03 no binário 0.17.0 — qualquer requisição sem `Authorization: Bearer <token>` leva **401** com `{"code":-32001,"message":"Authentication required"}`, o que é uma barreira do motor, não da borda. O quick tunnel do Cloudflare e os serviços SSH públicos não têm autenticação de borda. Logo, "senha na URL" só é real com um **porteiro** no meio — e é isso que a GUI sobe.

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

Comportamento verificado em teste real do porteiro: **sem senha -> 404**, **senha errada -> 404**, **senha correta -> resposta JSON-RPC 200**. Se o MCP local estiver fora, o porteiro responde **502**.

Detalhe técnico que evita um bug clássico: o porteiro descarta o `Connection` do cliente e força `Connection: close` para o MCP. Sem isso, o keep-alive do HTTP/1.1 deixaria a conexão aberta até o timeout de 30 s e o cliente perderia a requisição seguinte. Cada conexão atende uma requisição.

## 6. Sonda de exposição — "Testar pela internet"

Nunca presuma que o túnel está protegido. O botão **Testar pela internet** faz um `POST` `initialize` real na URL pública, saindo pela internet, usando `curl.exe` (o `TcpStream` da GUI não faz TLS). A URL vai num arquivo de configuração do `curl` (`--config`), não no argv — o argv de qualquer processo é legível por outros processos, e a URL contém a senha. Os arquivos temporários são apagados depois do teste.

O resultado é classificado só a partir do que a rede respondeu:

| Badge | Como é decidido | O que fazer |
| --- | --- | --- |
| **EXPOSTO SEM AUTENTICAÇÃO (verificado agora)** | a resposta contém `"jsonrpc"` sem nenhuma credencial | qualquer pessoa com a URL controla esta máquina. Pare o túnel ou coloque senha/token/Access |
| **BORDA EXIGIU AUTENTICAÇÃO (verificado: HTTP nnn)** | HTTP 401, 403, 302 ou 407 | há autenticação na frente. Confirme que é a **sua** (Access, basic-auth, porteiro) e não uma página de erro do provedor. Um 401 com corpo `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` não vem da borda: é o próprio motor exigindo o bearer token (medido na 0.17.0 em 2026-08-03) |
| **NÃO FOI POSSÍVEL VERIFICAR — trate como exposto** | timeout, 5xx, falha do `curl` | não deu para provar nada. A postura honesta é assumir exposto até verificar |

O badge só aparece **depois** que você roda a sonda. Enquanto isso, a tela não afirma nada sobre exposição. E o status **ATIVO** significa apenas "URL pública publicada" — não "verificado pela internet"; são estados separados de propósito.

Em qualquer seção do app, quando há túnel ativo, um chip laranja **TUNEL ATIVO** aparece no pé da barra lateral. A máquina estar exposta não é informação de uma aba só.

## 7. Ciclo de vida: o túnel nunca sobrevive ao app

São quatro camadas independentes, porque uma só falharia nos casos interessantes:

| Camada | Cobre |
| --- | --- |
| `Child::kill()` no `on_exit` | fechamento normal da janela |
| **watchdog** PowerShell disparado no START | `taskkill /F`, crash da GUI, queda de energia — casos em que o `on_exit` nem roda. Ele fica vigiando o PID da GUI e mata o túnel quando ela desaparece |
| bloco no `shutdown_cleanup` | processo auxiliar destacado que também encerra túneis registrados e limpa o registro |
| **reconciliação na abertura** | ao abrir, a GUI procura rastros `tunnel:*` de sessões anteriores; se o processo ainda estiver vivo, mata e registra no console que **a máquina esteve exposta até agora** |

Um processo só é morto com **identidade de 3 fatores**: imagem (`cloudflared.exe`/`ngrok.exe`/`ssh.exe`) + `CreationDate` do processo + o marcador `run_id` presente na linha de comando (ele aparece no caminho do arquivo de log passado ao CLI). Isso elimina PID reciclado e, principalmente, protege o `cloudflared`/`ngrok`/`ssh` legítimo do usuário. **`taskkill /IM` é proibido no projeto** exatamente por isso.

O **Parar túnel** também não presume: mata a árvore (`taskkill /PID <pid> /T /F` antes do `wait()`), confirma ausência por identidade, encerra o porteiro e limpa o registro. Se o processo continuar vivo, o status vai para **ERRO** com aviso — não para "PARADO".

Ao desinstalar o app, `{app}\tunnel` (binários baixados, token-file, policy do ngrok) é removido.

## 8. Solução de problemas

| Sintoma | Causa | Correção |
| --- | --- | --- |
| Cloudflare quick tunnel não sobe; a GUI recusa antes de tentar | existe `%USERPROFILE%\.cloudflared\config.yaml`; com esse arquivo presente, o quick tunnel falha | renomeie/mova o arquivo, **ou** passe a usar o túnel nomeado (token-file) |
| ngrok recusa antes de subir, mensagem sobre authtoken | `ngrok config check` falhou — sem authtoken configurado | `ngrok config add-authtoken <SEU_TOKEN>` no seu terminal (conta em ngrok.com) e tente de novo |
| SSH sai imediatamente pedindo autenticação | `BatchMode=yes` impede prompt de senha, por design | use chave (`-i`) ou um destino que aceite chave/`nokey` |
| Status fica **INICIANDO** e a URL não aparece | o CLI ainda não imprimiu a URL, ou o sufixo não é reconhecido (túnel nomeado, servidor próprio) | ngrok: **Descobrir URL (API local 4040)**. Cloudflare nomeado / SSH próprio: informe a URL à mão no campo |
| URL pública responde **404** | é o porteiro de senha: o caminho não tem `/s/<senha>/` correta | use a URL completa que a GUI mostra (**Copiar URL**). Senha errada e ausência de senha produzem o mesmo 404, de propósito |
| URL pública responde **502** | o porteiro/borda está de pé mas o MCP local não responde: motor parado, ou a porta mudou depois que o túnel subiu. Causa comum medida em 2026-08-03: sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do processo, o `cua-driver serve` sai com código 1 e a porta fica muda — e a Tarefa Agendada `cua-driver-serve` só enxerga o token a partir do logon seguinte ao `setx` | verifique o badge na aba MCP & Rede, religue o motor e reinicie o túnel |
| Status vai para **ERRO** com trecho de log | o processo do túnel saiu sozinho | leia o final do log no console; conexão de rede, credencial expirada e limite de plano aparecem ali |
| Sonda diz **EXPOSTO SEM AUTENTICAÇÃO** e você esperava proteção | senha vazia no início do túnel, ou a autenticação de borda não está aplicada à rota | pare o túnel, reinicie com senha, e/ou configure Cloudflare Access / basic-auth do ngrok |
| Sonda diz **NÃO FOI POSSÍVEL VERIFICAR** | timeout de 20 s, 5xx da borda ou `curl.exe` ausente | tente novamente; confirme `curl --version`; trate como exposto até provar o contrário |

## Ver também

- [acesso-remoto.md](acesso-remoto.md) — comparação túnel x LAN x VPN, e as implicações de segurança de cada caminho.
- [uso-mcp-rede.md](uso-mcp-rede.md) — o motor tem de estar respondendo em loopback antes de qualquer túnel.
- [atualizacao.md](atualizacao.md) — o motor recente exige token (medido na 0.17.0: sem ele o `serve` nem sobe) e muda o comportamento do endpoint.
- [solucao-de-problemas.md](solucao-de-problemas.md) — problemas que não são exclusivos do túnel.
