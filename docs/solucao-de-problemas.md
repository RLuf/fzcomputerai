# Solução de problemas

Para quem tem um sintoma na tela e quer chegar à causa sem chutar.

**Antes de qualquer coisa: leia o console.** Ele fica no rodapé, é o mesmo em todas as seções e registra cada comando executado com a linha de comando, o `exit code`, o `stdout` e o `stderr`. Quase todo diagnóstico deste documento está lá em texto. Use **Copiar** para levar o log para um relato de bug, e **Ir ao fim** se o indicador estiver em "pausado (rolagem manual)".

## Tabela geral

| Sintoma | Causa provável | Verificação | Correção |
| --- | --- | --- | --- |
| "MCP parado" mas o motor está rodando | motor exigindo token: o POST de teste recebe **401**. Medido no binário `cua-driver` 0.17.0 em 2026-08-03: sem o header `Authorization`, a resposta é `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}`, idêntica em `POST /mcp`, `GET /mcp` e `GET /`, e **sem** header `WWW-Authenticate`. A conexão TCP é aceita normalmente (`Test-NetConnection` na porta dá `True`) — a recusa é na camada de aplicação | `curl -sS -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:8000/mcp -d '{}'` retorna 401 | configure `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do usuário (o valor é escolhido por você — o próprio motor o chama de "host-generated bearer token"; não há comando que gere um) e **reabra a GUI**, que relê a variável na abertura. Com `Authorization: Bearer <token>` a mesma requisição volta 200 com o `result` do `initialize` |
| Porta muda: nada escutando, e o motor "sobe e some" | sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` **no ambiente do processo**, o `cua-driver serve` sai com `exit 1` — o daemon nem chega a subir (medido na 0.17.0 em 2026-08-03) | o `stderr` do `serve` traz `cua-driver serve error: CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled` | a Scheduled Task `cua-driver-serve` (usada pelo `autostart kick`) herda o ambiente do **logon**: quem gravou o token em `HKCU\Environment` depois de logar sobe um daemon sem token, que morre na hora e deixa a porta muda. Faça logoff/logon, ou use a GUI 2.1.0+: quando o kick não abre a porta, ela lê porta e token do registro, para o daemon anterior e lança o `serve` com as variáveis injetadas no processo filho. Só pode existir **um** daemon — um segundo `serve` recusa com "Cua Driver daemon is already running on `\\.\pipe\cua-driver` (pid N). Run `cua-driver stop` first." |
| "MCP parado" mas o processo do motor existe | `CUA_DRIVER_RS_MCP_HTTP_PORT` ausente: **sem ela o listener HTTP nem sobe** | `reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_PORT` | aba MCP & Rede -> **Aplicar Porta** (grava, confirma relendo o registro e reinicia o motor) |
| "MCP parado" e a porta está configurada | a porta configurada é outra que não a do campo | leia as linhas do `netstat` no diagnóstico: elas mostram a porta real em uso | ajuste o campo Porta para a porta real, ou **Aplicar Porta** para gravar a que você quer |
| Listener só em `127.0.0.1` | **comportamento normal e esperado** do motor oficial: endereço fixo no código, sem variável de bind | badge **LOCAL apenas**; no `netstat`, a coluna LOCAL mostra `127.0.0.1:<porta>` | para LAN use **Aplicar Regra** (encaminhamento); para internet use a aba **Túnel**. Não procure por bind `0.0.0.0`: não existe |
| Badge **REGRA SEM EFEITO** | a regra existe na config do `netsh` mas o listener não subiu — normalmente o serviço **IP Helper** | `netsh interface portproxy show v4tov4` lista a regra, mas `netstat -ano -p tcp` não mostra listener no IP da LAN | `Restart-Service iphlpsvc` (terminal elevado); se persistir, **Remover Regra** + **Aplicar Regra**; confirme que o IP do campo é o IP atual da máquina |
| Regra funcionando, mas a outra máquina não conecta | firewall do Windows bloqueando a entrada | o console avisa "netstat mostra listener na LAN mas o teste TCP falhou (firewall?)" | libere a porta de entrada no Windows Defender Firewall. A GUI **não** cria regra de firewall |
| **Aplicar Regra** não cria nada e o console fala de porta não confirmada | nenhuma porta candidata respondeu MCP em `127.0.0.1` — a GUI se recusa a encaminhar para porta morta | teste manual com `POST /mcp` em loopback | inicie o motor (**Iniciar**) e aplique a regra de novo |
| **Aplicar Regra** diz que já existe listener que não é portproxy | outro processo (ou outro serviço) já ocupa `<IP_LAN>:<porta>` | procure a linha correspondente no `netstat` e o PID | escolha outra porta, ou encerre o processo dono da porta |
| Prompt de UAC ao aplicar/remover regra | `netsh ... portproxy add/delete` exige elevação | — | aceite o UAC. Se cancelar, o console registra a falha com o exit code — nada é dado como feito |
| Túnel sobe e a URL responde **502** | a borda está de pé mas o destino local não responde: motor parado, ou a porta mudou depois que o túnel subiu | badge da aba MCP & Rede + teste POST em loopback | religue o motor e **reinicie o túnel** (ele guarda a porta que existia no início) |
| URL do túnel responde **404** | é o porteiro de senha: caminho sem `/s/<senha>/` correta | sem senha e senha errada dão o **mesmo** 404, de propósito | use a URL completa que a GUI mostra (**Copiar URL**). Perdeu a senha? Pare e reinicie o túnel com senha nova — ela não é persistida |
| Túnel vai para **ERRO** logo após iniciar | o processo do CLI saiu sozinho | o console traz o final do log do CLI | leia o motivo ali: credencial, limite de plano, rede. Casos comuns nas linhas seguintes desta tabela |
| Cloudflare quick tunnel recusa antes de subir | existe `%USERPROFILE%\.cloudflared\config.yaml`, que faz o quick tunnel falhar | `Test-Path $env:USERPROFILE\.cloudflared\config.yaml` | renomeie/mova o arquivo, ou use o túnel **nomeado** (token-file) |
| ngrok recusa antes de subir | `ngrok config check` falhou: sem authtoken | `ngrok config check` no seu terminal | `ngrok config add-authtoken <SEU_TOKEN>` (conta em ngrok.com) |
| SSH sai imediatamente | `BatchMode=yes` impede prompt de senha, por design — autenticação por senha não funciona aqui | o log do `ssh` mostra a falha de autenticação | use chave com **Chave (-i)**, ou um destino que aceite chave / `nokey` |
| Sonda diz **NÃO FOI POSSÍVEL VERIFICAR** | timeout de 20 s, 5xx da borda, ou `curl.exe` indisponível | `curl --version` | tente de novo; **trate como exposto** até conseguir provar o contrário |
| `cua-driver` não encontrado no PATH | o motor não está instalado, ou o PATH da sessão é anterior à instalação | `cua-driver --version` no terminal; `(Get-Command cua-driver).Source` | instale o motor pelo instalador oficial do Cua. A GUI (2.1.0+) resolve o caminho do motor sozinha (PATH ou o canônico `%LOCALAPPDATA%\Programs\Cua\cua-driver\bin\cua-driver.exe`), então funciona logo após a instalação; só o **terminal** precisa de uma sessão nova para enxergar o PATH atualizado |
| Toda ação falha com "não foi possível executar 'cua-driver'" | mesma causa acima | o console mostra o erro de execução do processo | idem |
| SmartScreen bloqueia o instalador | os binários **não são assinados** — é o aviso esperado | confira o `.sha256` publicado ao lado do artefato | `Get-FileHash ... -Algorithm SHA256` e compare com o `.sha256`. Só então "Mais informações" -> "Executar assim mesmo". Detalhes em [`SIGNING.md`](../SIGNING.md) |
| O app fecha e o MCP cai junto | **comportamento intencional** do `on_exit` | o console da sessão anterior registra a limpeza | se você quer o motor rodando sem a GUI, use a tarefa de autostart do próprio `cua-driver` — ver abaixo |
| Ao abrir, o console avisa "TUNEL ORFAO encerrado" | a sessão anterior terminou sem limpeza (kill forçado, crash, queda de energia) e o túnel ficou vivo | a própria mensagem, com PID e `run_id` | nada a fazer: já foi encerrado. Mas **a máquina esteve exposta até aquele momento** — avalie se precisa trocar credenciais que estavam em uso |
| Ao abrir, o console avisa "sem privilégio para remover a sobra" | uma regra `portproxy` nossa sobreviveu e a remoção sem elevação falhou | `netsh interface portproxy show v4tov4` | use **Remover Regra** (aceitando o UAC) ou feche o app, que tenta de novo com elevação |
| Caixas vazias no lugar de símbolos | fonte sem o glifo (`→`, `●`, emoji) | — | é bug, e a regra do projeto é não usar esses caracteres: `->` em texto e ponto **desenhado** para status. Relate onde apareceu |
| O download da atualização falha com erro de SHA256 | o hash do instalador baixado não confere com o `.sha256` publicado | o console mostra esperado x obtido | o instalador é **apagado** automaticamente e nada é executado. Tente de novo; se repetir, baixe manualmente da página de releases e confira o hash à mão |
| Central de Atualizações diz "não foi possível consultar o motor" | `cua-driver check-update` não executou | `cua-driver check-update --json` no terminal | mesma correção do PATH acima |

## Por que o MCP cai quando o app fecha

Isso é projeto, não defeito. Fechar a GUI significa **desligar o conjunto**: o `on_exit` chama `shutdown_cleanup()`, que mata o processo do túnel, encerra o porteiro de senha e dispara um processo auxiliar **independente**. O auxiliar espera a GUI morrer, encerra o `cua-driver`, mata túneis registrados e remove **apenas** as regras `portproxy` registradas por este app em `HKCU\Software\FzComputerAI`.

Dois detalhes que explicam a forma:

- **o auxiliar é desacoplado de propósito.** Uma versão anterior fazia a limpeza de forma bloqueante, com `Start-Process -Verb RunAs -Wait` para elevar o `netsh`. Como o delete exige admin, **todo** fechamento abria um UAC e o processo ficava preso esperando: a janela desaparecia e o processo continuava vivo, com as portas abertas. Agora o `on_exit` só dispara e retorna na hora;
- **nunca apagamos por padrão parecido.** Nesta mesma máquina existem regras LAN->loopback de outros serviços; elas não são nossas e não podem ser tocadas. E `taskkill /IM` é proibido no projeto — mataria `cloudflared`/`ngrok`/`ssh` legítimos do usuário. Processo só é morto com identidade de 3 fatores: imagem + `CreationDate` + marcador `run_id` na linha de comando.

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

# Prova real do MCP (GET não serve; e sem o header Authorization a resposta é 401)
curl -sS -X POST http://127.0.0.1:8000/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -H "Authorization: Bearer $env:CUA_DRIVER_RS_MCP_HTTP_TOKEN" `
  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual","version":"1"}}}'
```

Interpretação rápida da última resposta:

| Resposta | Leitura |
| --- | --- |
| JSON contendo `"jsonrpc"` | MCP funcionando |
| `401` com `{"error":{"code":-32001,"message":"Authentication required"}}` | motor exigindo token: falta (ou está errado) o header `Authorization: Bearer <token>`. Medido na 0.17.0 em 2026-08-03 |
| `405` | você usou GET; o endpoint existe mas exige POST. Atenção: sem `Authorization`, o GET responde **401**, não 405 (também medido na 0.17.0) |
| conexão recusada | nada escutando nessa porta |

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
