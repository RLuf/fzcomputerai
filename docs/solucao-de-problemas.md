# Solução de problemas

Para quem tem um sintoma na tela e quer chegar à causa sem chutar.

**Antes de qualquer coisa: leia o console.** Ele fica no rodapé, é o mesmo em todas as seções e registra cada comando executado com a linha de comando, o `exit code`, o `stdout` e o `stderr`. Quase todo diagnóstico deste documento está lá em texto. Use **Copiar** para levar o log para um relato de bug, e **Ir ao fim** se o indicador estiver em "pausado (rolagem manual)".

## Tabela geral

| Sintoma | Causa provável | Verificação | Correção |
| --- | --- | --- | --- |
| "MCP parado" mas o motor está rodando | motor `0.16+` exigindo token: o POST de teste recebe **401** | `curl -sS -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:8000/mcp -d '{}'` retorna 401 | configure `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do usuário (32–4096 caracteres) — na 2.2.0+ o botão **Gerar e ativar token do motor** (aba Túnel) faz isso por você e reinicia o motor; a GUI relê a variável na abertura e ao abrir a aba Túnel |
| "MCP parado" mas o processo do motor existe | `CUA_DRIVER_RS_MCP_HTTP_PORT` ausente: **sem ela o listener HTTP nem sobe** | `reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_PORT` | aba MCP & Rede -> **Aplicar Porta** (grava, confirma relendo o registro e reinicia o motor) |
| "MCP parado" e a porta está configurada | a porta configurada é outra que não a do campo | leia as linhas do `netstat` no diagnóstico: elas mostram a porta real em uso | ajuste o campo Porta para a porta real, ou **Aplicar Porta** para gravar a que você quer |
| Listener só em `127.0.0.1` | **comportamento normal e esperado** do motor oficial: endereço fixo no código, sem variável de bind | badge **LOCAL apenas**; no `netstat`, a coluna LOCAL mostra `127.0.0.1:<porta>` | para LAN use **Publicar na rede** (o relay do próprio app); para internet use a aba **Túnel**. Não procure por bind `0.0.0.0` no motor: não existe |
| A janela congela e o Windows escreve **"(Não Respondendo)"** no título | até a 2.3.1, cada ação rodava `Command::output()` **síncrono na thread da UI**: `reg` ~200 ms, `powershell` 300 ms–2 s e o teste de túnel dois `curl -m 20` (até 40 s) | veja qual ação estava em curso quando travou — o console mostra o comando que ficou pendurado | **corrigido na 2.4.0** (executor de segundo plano: sonda de túnel e espera pelo motor saíram da thread da UI). Se ainda travar, **anote a ação exata** e relate: significa que aquele caminho ainda não foi migrado |
| **Publicar na rede** não sobe e a mensagem fala de MCP que não respondeu | o relay se recusa a publicar porta morta: nada respondeu em `127.0.0.1` | teste manual com `POST /mcp` em loopback | inicie o motor (**Iniciar**) e publique de novo |
| **Publicar na rede** falha com erro ao bindar `0.0.0.0:<porta>` | outro processo já ocupa a porta em todas as interfaces (não é o motor: `0.0.0.0:<p>` coexiste com `127.0.0.1:<p>`) | o console traz o erro do sistema; procure o PID no `netstat` | escolha outra porta, encerre o processo dono, ou publique num IP específico no campo **Escutar em** |
| Publicado na rede, mas a outra máquina não conecta | firewall do Windows bloqueando a entrada | o contador de conexões do relay fica em `0 ativas / 0 desde o início` mesmo com a outra máquina tentando | libere a porta de entrada no Windows Defender Firewall. A GUI **não** cria regra de firewall |
| Badge **REGRA SEM EFEITO** (caminho legado `netsh`) | a regra existe na config do `netsh` mas o listener não subiu — normalmente o serviço **IP Helper** | `netsh interface portproxy show v4tov4` lista a regra, mas `netstat -ano -p tcp` não mostra listener no IP da LAN | desde a 2.3.0 o caminho padrão é o relay, que não depende do IP Helper: use **Publicar na rede** e, se quiser limpar, **Remover regra antiga (UAC)**. Para insistir no `netsh`: `Restart-Service iphlpsvc` em terminal elevado |
| Prompt de UAC ao remover a regra antiga | `netsh ... portproxy delete` exige elevação | — | aceite o UAC. Se cancelar, o console registra a falha com o exit code — nada é dado como feito. O relay **não** pede UAC |
| Túnel sobe e a URL responde **502** | a borda está de pé mas o destino local não responde: motor parado, ou a porta mudou depois que o túnel subiu | badge da aba MCP & Rede + teste POST em loopback | religue o motor e **reinicie o túnel** (ele guarda a porta que existia no início) |
| URL do túnel responde **404** | é o porteiro de senha: caminho sem `/s/<senha>/` correta | sem senha e senha errada dão o **mesmo** 404, de propósito | use a URL completa que a GUI mostra (**Copiar URL**). Perdeu a senha? Pare e reinicie o túnel com senha nova — ela não é persistida |
| O cliente MCP **não tem onde colar** o header `Authorization` e recebe 401 | é o caso do Claude Desktop e afins: eles aceitam **uma URL** e nada mais; o motor `0.16+` é *fail-closed* | o mesmo endpoint responde `initialize` OK quando você manda o Bearer pelo `curl` | suba o túnel **com senha** e entregue a URL completa (`/s/<senha>/mcp`): desde a 2.4.0 o porteiro injeta o Bearer do motor para quem passou pela senha. Se o cliente mandar o próprio `Authorization`, o dele vence |
| Túnel vai para **ERRO** logo após iniciar | o processo do CLI saiu sozinho | o console traz o final do log do CLI | leia o motivo ali: credencial, limite de plano, rede. Casos comuns nas linhas seguintes desta tabela |
| Cloudflare quick tunnel recusa antes de subir | existe `%USERPROFILE%\.cloudflared\config.yaml`, que faz o quick tunnel falhar | `Test-Path $env:USERPROFILE\.cloudflared\config.yaml` | renomeie/mova o arquivo, ou use o túnel **nomeado** (token-file) |
| Fiz o **Login Cloudflare** e mesmo assim não há túnel nem URL fixa | **o login sozinho não cria nada** — ele só baixa o `cert.pem` | **Verificar login** confere se existe `~/.cloudflared/cert.pem` | faltam os dois passos seguintes na aba Túnel: preencher **Nome do túnel** e **Hostname público** e clicar em **Criar túnel + apontar DNS** |
| **Criar túnel + apontar DNS**: o túnel é criado mas o **DNS não** | o `cloudflared tunnel route dns` só cria registro em domínio que está **na sua conta Cloudflare**, com os nameservers delegados a ela | o console traz a recusa do próprio `cloudflared`; confira o domínio no painel da Cloudflare | delegue o domínio à sua conta Cloudflare (ou use um que já esteja) e repita o botão. A GUI não tem como contornar isso |
| ngrok recusa antes de subir | `ngrok config check` falhou: sem authtoken | `ngrok config check` no seu terminal | `ngrok config add-authtoken <SEU_TOKEN>` (conta em ngrok.com) |
| ngrok morre com "unknown flag: --traffic-policy-file" | agente ngrok antigo (ex. 3.3.x) que não conhece *traffic policy* por arquivo | `ngrok http --help` não lista `--traffic-policy-file` | resolvido automaticamente na GUI 2.2.0: ela pergunta ao binário se a flag existe e, se não, cai para `ngrok start fz-mcp` com config v2 gerada (basic_auth em arquivo com ACL restrita, mesclada ao config do authtoken). Atualizar o agente ngrok (3.9+) também resolve |
| ngrok passa no `config check` mas morre com **ERR_NGROK_105** | authtoken **inválido**: `ngrok config check` só valida a sintaxe do arquivo, não o token — o processo morre na hora de autenticar no serviço | o final do log do CLI no console traz `ERR_NGROK_105`; a GUI 2.2.0 detecta o código e explica | `ngrok config add-authtoken <SEU_TOKEN>` com o token **real** de dashboard.ngrok.com |
| SSH sai imediatamente | `BatchMode=yes` impede prompt de senha, por design — autenticação por senha não funciona aqui | o log do `ssh` mostra a falha de autenticação | use chave com **Chave (-i)**, ou um destino que aceite chave / `nokey` |
| Sonda diz **NÃO FOI POSSÍVEL VERIFICAR** | timeout de 20 s, 5xx da borda, ou `curl.exe` indisponível | `curl --version` | tente de novo; **trate como exposto** até conseguir provar o contrário |
| Sonda diz **EXPOSTO SEM AUTENTICAÇÃO** num túnel que exige token | alarme falso de GUI anterior à 2.2.0: qualquer resposta contendo `"jsonrpc"` era marcada como exposta — mas o 401 do motor `0.16+` também tem corpo JSON-RPC | repita o `POST initialize` na URL pública **sem** `Authorization`: se vier `401`, o motor está barrando | atualize a GUI para a 2.2.0+: a sonda distingue `200` com `"result"` (exposto de verdade) de `401` com corpo JSON-RPC (**MOTOR EXIGIU TOKEN**) e, quando conhece o token, prova **PROTEGIDO E FUNCIONAL** com o Bearer |
| `cua-driver` não encontrado no PATH | o motor não está instalado, ou o PATH da sessão é anterior à instalação | `cua-driver --version` no terminal; `(Get-Command cua-driver).Source` | instale o motor pelo instalador oficial do Cua e **reabra a GUI** (processo filho herda o PATH do processo pai; PATH novo exige app novo) |
| Toda ação falha com "não foi possível executar 'cua-driver'" | mesma causa acima | o console mostra o erro de execução do processo | idem |
| SmartScreen bloqueia o instalador | os binários **não são assinados** — é o aviso esperado | confira o `.sha256` publicado ao lado do artefato | `Get-FileHash ... -Algorithm SHA256` e compare com o `.sha256`. Só então "Mais informações" -> "Executar assim mesmo". Detalhes em [`SIGNING.md`](../SIGNING.md) |
| O motor morre quando eu fecho o app | **é o comportamento correto desde a 2.3.0**: o motor é processo **filho** da GUI, adotado num Job Object com `KILL_ON_JOB_CLOSE`. O kernel o encerra quando a GUI termina de qualquer forma — X, **Sair** na bandeja, `taskkill /F`, crash, logoff | ao iniciar, o console registra `cua-driver serve iniciado como FILHO (pid N) e adotado pelo Job Object` | nada a corrigir. Se você quer o motor rodando **sem** a GUI, use a tarefa de autostart do próprio `cua-driver` — ver abaixo |
| A GUI avisa que **não** vai encerrar o motor ao fechar | já havia um `cua-driver` respondendo na porta quando o app abriu (ex.: subido pelo `.mcp.json` de outro cliente MCP). Ele é marcado como externo, **não** é duplicado nem morto às cegas | o console registra `Motor EXTERNO detectado (nao e filho desta GUI)` | é o comportamento correto. Se você quer que a GUI passe a mandar no ciclo de vida, clique em **Parar** (ação explícita sua) e depois em **Iniciar** |
| O motor **não** foi adotado pelo Job Object (aviso no console) | `AssignProcessToJobObject` falhou — o console traz o código de erro do Windows | procure a linha `NAO foi adotado pelo Job Object (erro N)` | a limpeza automática **não está garantida** para aquele processo: encerre-o pelo botão **Parar** antes de fechar o app, e relate o código de erro |
| Ao abrir, o console avisa "TUNEL ORFAO encerrado" | a sessão anterior terminou sem limpeza (kill forçado, crash, queda de energia) e o túnel ficou vivo | a própria mensagem, com PID e `run_id` | nada a fazer: já foi encerrado. Mas **a máquina esteve exposta até aquele momento** — avalie se precisa trocar credenciais que estavam em uso |
| Ao abrir, o console avisa "sem privilégio para remover a sobra" | uma regra `portproxy` nossa sobreviveu e a remoção sem elevação falhou | `netsh interface portproxy show v4tov4` | use **Remover regra antiga (UAC)** (aceitando o UAC) ou feche o app, que tenta de novo com elevação |
| Caixas vazias no lugar de símbolos | fonte sem o glifo (`→`, `●`, emoji) | — | é bug, e a regra do projeto é não usar esses caracteres: `->` em texto e ponto **desenhado** para status. Relate onde apareceu |
| O download da atualização falha com erro de SHA256 | o hash do instalador baixado não confere com o `.sha256` publicado | o console mostra esperado x obtido | o instalador é **apagado** automaticamente e nada é executado. Tente de novo; se repetir, baixe manualmente da página de releases e confira o hash à mão |
| Central de Atualizações diz "não foi possível consultar o motor" | `cua-driver check-update` não executou | `cua-driver check-update --json` no terminal | mesma correção do PATH acima |

## Por que o MCP cai quando o app fecha

Isso é projeto, não defeito. Fechar a GUI significa **desligar o conjunto**.

Desde a 2.3.0 quem garante isso é o **Job Object** do Windows (`fzcomputerai/src/lifecycle.rs`): o app cria um job com `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` antes de qualquer spawn e **adota** cada filho de longa duração — o motor (`cua-driver serve`) e o processo do túnel. Quando o processo da GUI termina **de qualquer forma** (X, **Sair** na bandeja, `taskkill /F`, crash, logoff), o Windows fecha os handles e o kernel mata os filhos. Não há vigia para falhar nem janela de tempo entre uma coisa e outra.

Existe motivo técnico para isso ser assim: no Windows um filho **não** morre com o pai — `CreateProcess` não cria esse vínculo, isso é comportamento de Unix. Sem o job, o motor sobreviveria à janela.

Medido nesta máquina: com GUI + motor filho + porteiro de pé, um `taskkill /F` na GUI derrubou motor e porteiro juntos, e o `cua-driver` de **outro** cliente MCP ficou intacto; em outro teste o `cloudflared` também caiu junto.

Além disso, o `on_exit` chama `shutdown_cleanup()`, que encerra o relay da LAN e o porteiro de senha (threads deste processo) e dispara um auxiliar **independente** para o que sobrevive a um processo: valores `tunnel:*` em `HKCU` e as regras `portproxy` **legadas** registradas por este app.

Três detalhes que explicam a forma:

- **o auxiliar é desacoplado de propósito.** Uma versão anterior fazia a limpeza de forma bloqueante, com `Start-Process -Verb RunAs -Wait` para elevar o `netsh`. Como o delete exige admin, **todo** fechamento abria um UAC e o processo ficava preso esperando: a janela desaparecia e o processo continuava vivo, com as portas abertas. Agora o `on_exit` só dispara e retorna na hora;
- **saíram do fechamento** o `taskkill /F /IM cua-driver.exe` e o `cua-driver stop`: os dois matavam motor de **qualquer** origem, inclusive o daemon que outro cliente MCP tivesse subido. Parar motor alheio virou ação explícita do usuário, no botão **Parar**;
- **nunca apagamos por padrão parecido.** Nesta mesma máquina existem regras LAN->loopback de outros serviços; elas não são nossas e não podem ser tocadas. E `taskkill /IM` é proibido no projeto — mataria `cloudflared`/`ngrok`/`ssh` legítimos do usuário. Processo só é morto com identidade de 3 fatores: imagem + `CreationDate` + marcador `run_id` na linha de comando.

O watchdog em PowerShell do túnel continua existindo, mas como **fallback**: ele só é disparado quando a adoção no Job Object falha — e essa falha aparece no console.

Se você quer o motor rodando com a GUI fechada, esse é o papel da tarefa de autostart do **próprio** `cua-driver`, registrada pelo instalador oficial dele — não desta interface.

## Comandos de verificação que valem sempre

```powershell
# Motor: existe? qual versão? o que ele mesmo diz do sistema?
cua-driver --version
cua-driver check-update --json
cua-driver doctor

# Configuração do endpoint no ambiente do usuário
reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_PORT
reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_TOKEN   # não exiba o valor em público

# O que o sistema realmente tem de pé
netstat -ano -p tcp | Select-String ":8000"
netsh interface portproxy show v4tov4
Get-Service iphlpsvc | Format-List Name, Status, StartType

# O que ESTE app registrou como propriedade dele
reg query "HKCU\Software\FzComputerAI"

# Prova real do MCP (GET não serve: o endpoint responde 405)
curl -sS -X POST http://127.0.0.1:8000/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual","version":"1"}}}'
```

Interpretação rápida da última resposta:

| Resposta | Leitura |
| --- | --- |
| JSON contendo `"jsonrpc"` | MCP funcionando |
| `401` | motor `0.16+` exigindo token |
| `405` | você usou GET; o endpoint existe mas exige POST |
| `404` | é o porteiro de senha: caminho sem `/s/<senha>/` correta |
| conexão recusada | nada escutando nessa porta |

### Provando de fora da rede

A sonda da GUI sai desta mesma máquina. Para conferir o caminho inteiro **de outra rede**, use `scripts/remote-teste.py` (só biblioteca padrão do Python 3):

```bash
python remote-teste.py <URL> [--token TOKEN] [--termo TEXTO]
```

Ele faz `initialize`, `tools/list`, abre uma janela **nova** de navegador na máquina remota (nunca sequestra uma existente), navega para o buscador, digita o termo, descobre e clica no botão de pesquisa (**Search**/**Pesquisar**/**Buscar**) ou envia Enter, e confere o resultado lendo a tela de volta. Se a URL já tiver a senha (`/s/<senha>/mcp`), o `--token` não é necessário. O script reconfigura o `stdout` para UTF-8 de propósito: a resposta do motor tem emoji e o console `cp1252` do Windows quebraria. Lembre que rodar isso **opera de verdade** o computador remoto.

## Reunindo informação para relatar um problema

1. Abra o console, clique em **Copiar**.
2. Anote o badge exibido na aba MCP & Rede e, se aplicável, o badge do túnel e o resultado da sonda de exposição.
3. Rode `cua-driver check-update --json` e a versão da GUI (barra lateral, abaixo do nome).
4. Inclua as saídas de `netstat -ano -p tcp` e `netsh interface portproxy show v4tov4` referentes à porta em questão.
5. **Não** cole tokens, senhas de porteiro nem o conteúdo do token-file do Cloudflare. A GUI já mascara a senha do porteiro como `/s/***/`, mas confira antes de publicar.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — o significado exato de cada badge e do diagnóstico cru.
- [uso-tunel.md](uso-tunel.md) — problemas específicos de cada provedor de túnel.
- [atualizacao.md](atualizacao.md) — o caso do token depois de atualizar o motor.
- [acesso-remoto.md](acesso-remoto.md) — por que "listener só em loopback" não é um bug.
