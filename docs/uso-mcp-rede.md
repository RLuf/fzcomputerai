# Aba MCP & Rede — passo a passo

Para quem vai colocar o endpoint MCP de pé e precisa entender exatamente o que cada badge está afirmando.

Esta é a seção inicial do aplicativo. Ela tem três áreas: **controles do serviço** (topo, sempre visível), **configuração de porta/IP e encaminhamento** (meio, sempre visível) e **diagnóstico** (embaixo, única área que rola). A saída de todo comando vai para o console global do rodapé — comum a todas as seções. Código: `fzcomputerai/src/tabs/network.rs` e os métodos correspondentes em `fzcomputerai/src/app.rs`.

## 1. Controles do serviço CUA Driver

| Botão | Comando real | O que acontece depois |
| --- | --- | --- |
| **Iniciar** / *Start* | `cua-driver serve` como **processo filho da GUI** (porta e token injetados no ambiente do filho) | refaz o teste do endpoint e atualiza o badge. Se o endpoint **já responde**, o botão não encosta no motor (ver abaixo) |
| **Parar** / *Stop* | `cua-driver stop` | refaz o teste do endpoint e atualiza o badge |
| **Reiniciar** / *Restart* | `cua-driver stop` e, em seguida, `cua-driver serve` como processo filho | idem — é o caminho para **forçar** a troca de processo |
| **Testar Endpoint** / *Test Endpoint* | nenhum processo — sonda de rede própria | `POST /mcp` em `127.0.0.1` e no IP da LAN + `netstat -ano -p tcp` |
| *(automático)* vigia de status | thread `fz-status-watch` — `POST initialize` em `127.0.0.1:<porta>` a cada 5 s, fora da thread da UI | quando o resultado **muda** (motor iniciado ou derrubado fora da GUI), roda a mesma verificação do *Testar Endpoint* e loga `[vigia] …`. O badge nunca fica parado num estado velho (v2.3.2) |
| **Iniciar com Windows** (checkbox) | `reg add`/`reg delete` em `HKCU\...\CurrentVersion\Run`, valor `FzComputerAI` | o checkbox é **relido do registro** depois de gravar, então ele reflete o estado real, não o clique |

O badge à direita do título tem três estados:

| Badge | Significado exato |
| --- | --- |
| **ATIVO (local + LAN) :porta** / *ACTIVE (local + LAN)* | o MCP respondeu JSON-RPC em `127.0.0.1:<porta>` **e** em `<IP_LAN>:<porta>`, **e** o `netstat` confirma o listener na LAN |
| **LOCAL apenas :porta** / *LOCAL only* | o MCP responde em `127.0.0.1` e nada mais. É o estado **normal e esperado** do motor oficial |
| **PARADO** / *STOPPED* | nada respondeu na porta configurada |

> "Iniciar com Windows" liga a **GUI** no logon. A tarefa de autostart do **motor** é assunto do próprio `cua-driver` (registrada pelo instalador oficial dele) — são coisas separadas.

### A GUI é dona do daemon (v2.1.1)

Até a v2.1.0, "Iniciar" chamava `cua-driver autostart kick` e quem subia o motor era a **Scheduled Task** `cua-driver-serve` — o daemon nascia filho do Agendador de Tarefas. Três consequências, todas medidas:

- **os logs do motor sumiam.** O `stdout` pertencia à tarefa, então a atividade de clientes MCP externos (conector do Claude, Antigravity, Cursor) não aparecia no console do app;
- **a tarefa herda o ambiente do logon.** Token e porta gravados em `HKCU\Environment` *depois* de você já estar logado não eram vistos, e o daemon 0.16+ morria no ato — porta muda e a GUI dizendo apenas **PARADO**;
- a GUI não controlava o ciclo de vida daquilo que ela gerencia.

Agora ela lança `cua-driver serve` como **processo filho**, com porta e token injetados no ambiente do filho. O `stdout` e o `stderr` vão para `%TEMP%\fzcomputerai-update\cua-driver-serve.log`, que o console do rodapé acompanha como um `tail -f`, com prefixo `[motor]`. A Scheduled Task continua existindo como **último recurso**: se for ela que subir o motor, o console avisa que o processo não é da GUI e que não haverá logs.

> **"Iniciar" não derruba mais um daemon saudável.** Antes a função parava o motor e subia outro **sem checar** se o endpoint já respondia. No Windows, sockets da porta que já receberam conexão ficam retidos em `TIME_WAIT` por minutos, então o `serve` novo não conseguia o bind — `MCP HTTP transport disabled — bind 127.0.0.1:8000 failed (os error 10048)`, ou seja, daemon zumbi: pipe vivo, porta muda. Na prática, clicar **Iniciar** quebrava o que estava funcionando. Hoje, com o endpoint respondendo, o botão não encosta no motor; **Reiniciar** continua sendo o caminho para forçar a troca de processo.

## 2. Configuração de Porta & Rede

Dois campos e dois botões.

**Porta TCP HTTP (padrão 8000)** — é a porta do endpoint MCP HTTP do motor.

**Aplicar Porta** / *Apply Port* faz, nesta ordem, tudo logado:

1. grava `CUA_DRIVER_RS_MCP_HTTP_PORT` em `HKCU\Environment` via `[Environment]::SetEnvironmentVariable(..., 'User')`;
2. **relê o registro** com `reg query` e só considera sucesso se o valor conferir;
3. reinicia o motor (`cua-driver stop` seguido de `cua-driver serve` como processo filho da GUI, já com a porta nova no ambiente);
4. registra no console a nota de que o motor oficial escuta **somente** em `127.0.0.1` e que nada é gravado para "bind";
5. se encontrar sobra de `CUA_DRIVER_RS_MCP_HTTP_BIND` no ambiente do usuário, **apaga** — configuração que não faz nada só atrapalha o diagnóstico;
6. refaz o teste do endpoint.

Sem essa variável definida, **o motor não sobe listener HTTP nenhum**. O botão não promete bind em todas as interfaces porque isso não existe no motor oficial (detalhes em [acesso-remoto.md](acesso-remoto.md)).

**Endereço IP da LAN (autodetectado, editável)** — a autodetecção não envia pacote: um `UdpSocket` faz `connect` para `8.8.8.8:80` apenas para o sistema escolher a interface de saída, e o IP local dessa interface é lido. Se a detecção cair em `127.0.0.1` (máquina sem rota externa), os testes de LAN e o encaminhamento são desativados com aviso no console — regra de loopback para loopback não faz sentido.

**Verificar e Atualizar** / *Check & Update* abre a Central de Atualizações, que cuida da GUI **e** do motor — e **age** em vez de só relatar: motor desatualizado é atualizado automaticamente de ponta a ponta (para o daemon antigo, aplica a última versão estável e religa o autostart, sem mais cliques); GUI desatualizada baixa o instalador em segundo plano com SHA256 conferido, e somente a troca final pede confirmação (exige fechar o aplicativo). Ver [atualizacao.md](atualizacao.md).

## 3. Encaminhamento LAN

O mapeamento exibido em destaque é literal:

```text
<IP_LAN>:<porta>  ->  127.0.0.1:<porta>
```

A partir da v2.1.1 quem faz esse encaminhamento é o **próprio aplicativo**: uma thread do processo escuta em `<IP_LAN>:<porta>` e copia bytes contra `127.0.0.1:<porta>` (`std::net::TcpListener` + `std::io::copy`, zero dependência nova). É TCP puro — `curl`, `telnet` e `nc` atravessam igual. Não exige elevação, não depende do serviço IP Helper e não deixa nada gravado no Windows.

`netsh interface portproxy` continua no código **apenas como fallback**, para quando o bind no IP da LAN falhar. Nesse caminho valem as restrições de sempre: depende do serviço IP Helper (`iphlpsvc`) e a criação da regra pode exigir elevação, com prompt de UAC.

### Por que saiu do `portproxy`

O `portproxy` é uma regra **estática** do serviço IP Helper. Os três motivos, todos medidos:

- exigia **admin/UAC** tanto para criar quanto para remover;
- continuava aparecendo como `LISTENING` na LAN **mesmo com o motor morto** — aceitava a conexão, que então morria no destino. Falso positivo de serviço no ar, exatamente o que este app existe para não fazer;
- **sobrevivia** ao fechamento do app e ao reboot, o que obrigava uma rotina de limpeza.

Evidência do comportamento atual (2026-08-03): com o app aberto, o `netstat` mostrou `127.0.0.1:8000` **e** `192.168.0.101:8000` em `LISTENING`, o MCP respondeu HTTP 200 nos dois, e `netsh interface portproxy show v4tov4` não tinha **nenhuma** regra. Ao fechar o app, as duas portas fecharam junto e nada ficou no sistema.

| Botão | O que faz |
| --- | --- |
| **Aplicar Regra** / *Apply Rule* | liga o encaminhamento `<IP_LAN>:<porta> -> 127.0.0.1:<porta confirmada>` |
| **Remover Regra** / *Remove Rule* | desliga o encaminhamento do par IP:porta que estiver nos campos — e apaga a regra `netsh` se o fallback tiver sido usado |
| **Atualizar Status** / *Refresh Status* | reexecuta o teste do endpoint, que já recalcula este badge |

### O que "Aplicar Regra" faz por dentro

1. **Recusa** se o IP for loopback ou a porta for inválida.
2. **Descobre a porta de destino real** (`detect_confirmed_cua_port`): testa, em ordem, a porta de `HKCU\Environment`, a porta do campo da UI e 8000 — e usa a **primeira que responder MCP de verdade** em `127.0.0.1`. Se nenhuma responder, **nenhuma regra é criada**: encaminhar para porta morta só mascararia o problema. A mensagem pede para iniciar o motor primeiro.
3. Se o encaminhamento já estiver ligado para esse par IP:porta, não faz nada.
4. Se já existir um **listener de outro processo** em `<IP_LAN>:<porta>`, **nada é alterado** — subir o nosso conflitaria com o que já está lá.
5. Sobe a thread de encaminhamento: `bind` em `<IP_LAN>:<porta>` e, para cada conexão aceita, abre `127.0.0.1:<porta confirmada>` e emenda os dois soquetes copiando bytes nos dois sentidos.
6. **Só se o bind falhar**, cai para o fallback: `netsh interface portproxy add v4tov4 listenport=... listenaddress=... connectport=... connectaddress=127.0.0.1` e, se ainda falhar, uma segunda tentativa elevada por `Start-Process -Verb RunAs -Wait -PassThru`, propagando o exit code do `netsh` elevado (UAC cancelado ⇒ falha registrada, não silenciada). Nesse caminho a regra é relida em `netsh interface portproxy show v4tov4` e confirmada por comparação de tokens (nunca `contains()` solto), e registrada como propriedade deste app em `HKCU\Software\FzComputerAI`, valor `portproxy:<ip>:<porta>` — **só regras registradas aqui são removidas** na limpeza automática.
7. Confirma com `netstat` que `<IP_LAN>:<porta>` está `LISTENING`.
8. Refaz o teste do endpoint nos dois endereços.

### Os 3 estados do badge

| Badge | Encaminhamento ligado? | Listener de pé no `netstat`? | Leitura |
| --- | --- | --- | --- |
| **REGRA FUNCIONANDO** / *RULE WORKING* | sim | sim | verificado dos dois lados |
| **REGRA SEM EFEITO** / *RULE NOT EFFECTIVE* | sim | **não** | a configuração existe mas o Windows não está servindo a porta |
| **SEM REGRA** / *NO RULE* | não | — | nada configurado para este par IP:porta |

### Resolvendo "REGRA SEM EFEITO"

Com o encaminhamento feito pelo app este estado praticamente não aparece: a thread ou consegue o bind (e aí o listener é do próprio processo) ou falha na hora, com erro no console. Ele sobrou para o caminho de **fallback** `netsh`, onde o listener é do **IP Helper** e não do app — e aí significa quase sempre que o IP Helper não subiu o listener. Na ordem:

1. Confirme o serviço e ligue se estiver parado (precisa de terminal elevado):

```powershell
Get-Service iphlpsvc | Format-List Name, Status, StartType
Start-Service iphlpsvc          # se estiver Stopped
Restart-Service iphlpsvc        # se estiver Running mas sem servir
```

2. Confira o que o Windows realmente tem na config e no socket:

```powershell
netsh interface portproxy show v4tov4
netstat -ano -p tcp | Select-String ":8000"
```

3. Se a regra continua listada sem listener, remova e reaplique pela GUI (**Remover Regra**, depois **Aplicar Regra**). O `netsh` pode manter entrada órfã depois de troca de IP da máquina.
4. Confirme que o IP no campo é o IP **atual** da interface ativa. Regra criada para um IP que a máquina não tem mais fica na config e nunca sobe listener.
5. Se o listener sobe mas outra máquina não conecta, o problema saiu do encaminhamento e virou **firewall** — vale para os dois caminhos, o do app e o de fallback. A GUI avisa isso no console ("netstat mostra listener na LAN mas o teste TCP falhou (firewall?)"). É preciso liberar a porta de entrada no Windows Defender Firewall — a GUI **não** cria regra de firewall.

## 3.5. HTTPS do endpoint (v2.2.0)

Painel na área rolável, acima do diagnóstico. Liga um listener TLS **dentro do app** em `<bind>:8443` que
encaminha para o motor em `127.0.0.1:<porta>` — mesma mecânica do Encaminhamento LAN (thread do processo,
sem admin, cai ao fechar). Certificado auto-assinado gerado no setup/primeiro run, Let's Encrypt ou próprio.
O badge só fica verde após handshake TLS + `POST initialize` reais; **Testar Endpoint** e o vigia automático (5 s) incluem o HTTPS. O
bearer token continua obrigatório. Guia completo: [https.md](https.md).

## 4. Lendo o diagnóstico

A área rolável mostra apenas fatos verificados.

**Endpoint MCP HTTP (JSON-RPC) — estado real.** Tabela com porta, host, transporte e status. O host é o **real**: em "LOCAL APENAS" ele mostra `127.0.0.1 (loopback)`, não o IP que você gostaria de usar. O transporte é `HTTP / JSON-RPC` — não há WebSocket aqui, e a tela não anuncia o que não existe.

**URL de Conexão MCP (estado real).** Só mostra o IP da LAN quando `netstat` + POST confirmaram o listener na LAN; nos outros casos mostra a URL de loopback. Tem botão **Copiar**.

**Linha `HTTPS / TLS -> HTTP`** (v2.2.0). Porta, host e status do listener HTTPS, com o mesmo critério: `LISTENING (TLS + JSON-RPC)` só depois da sonda TLS passar. O host mostrado é o que a sonda alcançou de fato.

**Conexões reais na porta (`netstat -ano`).** As linhas cruas, com as mesmas colunas do terminal (`PROT / LOCAL / REMOTO / ESTADO / PID`). Inclui:

- `LISTENING` — soquetes em espera;
- `ESTABLISHED` — conexões MCP em andamento;
- listeners em portas altas (≥ 1024) no IP da LAN, para que um listener órfão de outra porta apareça em vez de ficar invisível. Portas de serviço do sistema (137/139/445...) são filtradas para não poluir.

Nota que evita a confusão mais comum: em um listener em espera, a coluna REMOTO aparece como `0.0.0.0:0`. Esse é o formato padrão do Windows para "aguardando conexões" — **não** é um destino nem indício de bind em todas as interfaces.

**Regras portproxy existentes (`netsh show v4tov4`).** Todas as regras `v4tov4` da máquina, cruas — inclusive as que não são nossas e as órfãs em outras portas. Com o encaminhamento feito pelo app, o normal é esta lista aparecer **vazia** mesmo com a LAN funcionando; ela só ganha uma linha nossa quando o fallback `netsh` entra em ação. Para limpar uma órfã pela GUI: ajuste os campos Porta e IP para os da regra e use **Remover Regra**.

## 5. Checklist rápido

```powershell
# 1. o motor existe e está no PATH?
cua-driver --version

# 2. a porta está configurada no ambiente do usuário?
reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_PORT

# 3. o endpoint responde MCP de verdade? (GET não prova nada: responde 405)
curl -sS -X POST http://127.0.0.1:8000/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual","version":"1"}}}'
```

Se o comando 3 retornar **401**, falta o token. Acrescente `-H "Authorization: Bearer <token>"`. A GUI faz isso automaticamente com o valor de `CUA_DRIVER_RS_MCP_HTTP_TOKEN` em `HKCU\Environment` — que ela mesma gera na primeira vez que precisa. Para ler o valor:

```powershell
reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_TOKEN
```

O contrato do token não é citação de documentação: foi **medido no binário `cua-driver` 0.17.0 em 2026-08-03**.

- Sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente **do processo**, o daemon nem sobe: `cua-driver serve` sai com exit 1 e stderr `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`. Não é recusa de requisição, é recusa de inicialização — na GUI isso aparece como badge **PARADO** e porta muda.
- Com o daemon no ar, qualquer requisição sem `Authorization` recebe **HTTP 401** com corpo `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` — idêntico em `POST /mcp`, `GET /mcp` e `GET /`, e sem cabeçalho `WWW-Authenticate`. A conexão TCP em si é aceita (`Test-NetConnection` dá `True`): a recusa é na camada de aplicação, não no socket. Com o `Bearer` correto, **HTTP 200** e o `result` do `initialize`.
- O token é gerado pelo **host** — e o host é este aplicativo. O motor chama o valor de *host-generated bearer token* e não tem comando para produzi-lo (nem o `cua-driver`, nem o `install.ps1` oficial). A partir da v2.1.1 a GUI gera 32 bytes do RNG do Windows (64 caracteres hex) e persiste em `HKCU\Environment` na primeira vez que precisa: você não precisa saber que a variável existe. Ela **nunca** imprime o valor no console — para lê-lo, use o `reg query` acima. Persistir no registro é também o que faz a Tarefa Agendada enxergar o token no próximo logon, quando ela for o caminho usado.

## Ver também

- [acesso-remoto.md](acesso-remoto.md) — quando usar encaminhamento, quando usar túnel, quando usar VPN.
- [uso-tunel.md](uso-tunel.md) — expor o MCP na internet com senha e verificação de exposição.
- [solucao-de-problemas.md](solucao-de-problemas.md) — tabela sintoma -> causa -> verificação -> correção.
- [atualizacao.md](atualizacao.md) — o botão Verificar e Atualizar e o que ele faz com cada componente.
