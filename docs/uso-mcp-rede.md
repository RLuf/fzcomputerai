# Aba MCP & Rede — passo a passo

Para quem vai colocar o endpoint MCP de pé e precisa entender exatamente o que cada badge está afirmando.

Esta é a seção inicial do aplicativo. Ela tem três áreas: **controles do serviço** (topo, sempre visível), **configuração de porta/IP e encaminhamento** (meio, sempre visível) e **diagnóstico** (embaixo, única área que rola). A saída de todo comando vai para o console global do rodapé — comum a todas as seções. Código: `fzcomputerai/src/tabs/network.rs` e os métodos correspondentes em `fzcomputerai/src/app.rs`.

## 1. Controles do serviço CUA Driver

| Botão | Comando real | O que acontece depois |
| --- | --- | --- |
| **Iniciar** / *Start* | `cua-driver serve` como **processo filho** desta GUI | espera o endpoint responder **em segundo plano** e atualiza o badge |
| **Parar** / *Stop* | mata o filho pelo handle do processo — ou, se o motor de pé não é nosso, a via oficial `cua-driver stop` | refaz o teste do endpoint e atualiza o badge |
| **Reiniciar** / *Restart* | **Parar + Iniciar** (o mesmo filho, recriado) | idem |
| **Testar Endpoint** / *Test Endpoint* | nenhum processo — sonda de rede própria | `POST /mcp` em `127.0.0.1` e no IP da LAN + `netstat -ano -p tcp` |
| **Iniciar com Windows** (checkbox) | `reg add`/`reg delete` em `HKCU\...\CurrentVersion\Run`, valor `FzComputerAI` | o checkbox é **relido do registro** depois de gravar, então ele reflete o estado real, não o clique |

O badge à direita do título tem três estados:

| Badge | Significado exato |
| --- | --- |
| **ATIVO (local + LAN) :porta** / *ACTIVE (local + LAN)* | o MCP respondeu JSON-RPC em `127.0.0.1:<porta>` **e** em `<IP_LAN>:<porta>`, **e** o `netstat` confirma o listener na LAN |
| **LOCAL apenas :porta** / *LOCAL only* | o MCP responde em `127.0.0.1` e nada mais. É o estado **normal e esperado** do motor oficial |
| **PARADO** / *STOPPED* | nada respondeu na porta configurada |

### O motor é processo filho da GUI (desde a 2.3.0)

Você normalmente não precisa clicar em **Iniciar**: **ao abrir o app, se nada estiver respondendo na porta, o motor sobe sozinho como processo filho** — e é encerrado quando o app fecha. Quem garante isso é um **Job Object do Windows** criado antes de qualquer spawn: no Windows um processo filho *não* morre com o pai (isso é comportamento de Unix), então o vínculo é feito no kernel. Com ele, o motor cai junto no fechamento normal, no **Sair** da bandeja, num `taskkill /F` na GUI, num crash ou no logoff.

Se **já houver um motor respondendo que não foi iniciado por esta GUI** (por exemplo, subido por um cliente MCP seu), ele é detectado como **externo**: nada é duplicado e nada é morto às cegas — a interface avisa que aquele motor **não** será encerrado ao fechar o app. Encerrá-lo é ação explícita sua, no botão **Parar**.

> "Iniciar com Windows" liga a **GUI** no logon — e, como o motor sobe junto com ela, é por aí que se tem o motor no logon. A GUI **não** aciona mais a tarefa de autostart do motor (`cua-driver autostart kick` saiu de todos os caminhos executáveis, inclusive do Reiniciar). Se o instalador oficial do `cua-driver` registrou uma tarefa nesta máquina, ela continua sendo assunto dele — e um motor subido por ela aparece aqui como **externo**.
>
> **Motores `0.16+`:** o **Testar Endpoint** envia `Authorization: Bearer <token>` automaticamente quando encontra `CUA_DRIVER_RS_MCP_HTTP_TOKEN` em `HKCU\Environment` (relido na abertura do app e ao abrir a aba Túnel). Uma resposta **401** significa **motor vivo exigindo token** — não "parado". Se ainda não há token nenhum configurado, ele pode ser gerado na aba **Túnel** (botão **Gerar e ativar token do motor**).

## 2. Configuração de Porta & Rede

Dois campos e dois botões.

**Porta TCP HTTP (padrão 8000)** — é a porta do endpoint MCP HTTP do motor.

**Aplicar Porta** / *Apply Port* faz, nesta ordem, tudo logado:

1. grava `CUA_DRIVER_RS_MCP_HTTP_PORT` em `HKCU\Environment` via `[Environment]::SetEnvironmentVariable(..., 'User')`;
2. **relê o registro** com `reg query` e só considera sucesso se o valor conferir;
3. reinicia o motor **filho** (Parar + Iniciar) para ele subir já com a porta nova. Se não houver motor filho, ele diz isso no console e **não** reinicia motor de terceiro — a porta fica gravada e você usa **Iniciar** quando quiser;
4. registra no console a nota de que o motor oficial escuta **somente** em `127.0.0.1` e que nada é gravado para "bind";
5. se encontrar sobra de `CUA_DRIVER_RS_MCP_HTTP_BIND` no ambiente do usuário, **apaga** — configuração que não faz nada só atrapalha o diagnóstico;
6. refaz o teste do endpoint.

Sem essa variável definida, **o motor não sobe listener HTTP nenhum**. O botão não promete bind em todas as interfaces porque isso não existe no motor oficial (detalhes em [acesso-remoto.md](acesso-remoto.md)).

**Endereço IP da LAN (autodetectado, editável)** — a autodetecção não envia pacote: um `UdpSocket` faz `connect` para `8.8.8.8:80` apenas para o sistema escolher a interface de saída, e o IP local dessa interface é lido. Se a detecção cair em `127.0.0.1` (máquina sem rota externa), os testes de LAN são desativados com aviso no console — sondar loopback contra loopback não prova nada. O relay não depende desse campo: ele escuta no que estiver em **Escutar em** (`0.0.0.0` por padrão).

**Verificar Atualizações** / *Check for Updates* abre a Central de Atualizações, que cuida da GUI **e** do motor. Ver [atualizacao.md](atualizacao.md).

## 3. Encaminhamento LAN (relay do próprio app)

O mapeamento exibido em destaque é literal:

```text
0.0.0.0:<porta>  ->  127.0.0.1:<porta confirmada do motor>
```

Desde a 2.3.0 quem faz esse encaminhamento é um **relay TCP dentro do processo da GUI** — não mais o `netsh interface portproxy`. Um socket escuta em `0.0.0.0` (ou no IP que você escolher no campo **Escutar em**) e copia bytes nos dois sentidos até o motor em `127.0.0.1`, **sem inspecionar HTTP**: keep-alive e streaming SSE passam intactos.

O que muda na prática:

| | `netsh portproxy` (até a 2.2) | relay interno (2.3.0+) |
| --- | --- | --- |
| **Elevação** | pode exigir UAC para criar a regra | **não pede UAC** |
| **Rastro no sistema** | regra persistida — **sobrevive a reiniciar o Windows** | nenhum: vive na memória do processo |
| **Ciclo de vida** | continua servindo depois que o app fecha | **encerra junto com o app** |
| **Dependência** | serviço IP Helper (`iphlpsvc`) | nenhuma |
| **Uso real visível** | nada | contador de conexões (ativas / total desde o início) |

**Mesma porta, sem mexer no motor.** Medido nesta plataforma: um listener em `0.0.0.0:8000` **coexiste** com o `127.0.0.1:8000` do motor. Por isso a publicação usa a **mesma porta** do endpoint — a URL da LAN tem a porta que você já usa em loopback.

| Botão | O que faz |
| --- | --- |
| **Publicar na rede** / *Publish on network* | sobe o relay em `<Escutar em>:<porta>` |
| **Parar** / *Stop* | encerra o relay; a porta deixa de responder pela rede na hora |
| **Atualizar Status** / *Refresh Status* | reexecuta o teste do endpoint |

O campo **Escutar em** aceita `0.0.0.0` (qualquer interface, padrão) ou um IP específico desta máquina — há botões de atalho para os dois. Ele só é editável com o relay parado.

### O que "Publicar na rede" faz por dentro

1. **Descobre a porta de destino real** (`detect_confirmed_cua_port`): testa, em ordem, a porta de `HKCU\Environment`, a porta do campo da UI e 8000 — e usa a **primeira que responder MCP de verdade** em `127.0.0.1`. Se nenhuma responder, **nada é publicado**: encaminhar para porta morta só mascararia o problema. A mensagem pede para iniciar o motor primeiro.
2. Faz `bind` em `<Escutar em>:<porta>`. Se o bind falhar, o **erro real do sistema** aparece na tela e no console — nada de "provavelmente funcionou".
3. Sobe a thread que aceita conexões; cada conexão ganha a sua própria thread, que copia os bytes nos dois sentidos e mantém os contadores.
4. Registra o mapeamento publicado no console e refaz o teste do endpoint.

### Os 2 estados do badge

| Badge | Significado exato |
| --- | --- |
| **PUBLICADO NA REDE** / *PUBLISHED ON NETWORK* | o relay deste app está escutando e encaminhando |
| **SÓ LOCAL** / *LOCAL ONLY* | não há relay; o MCP só responde em `127.0.0.1` |

São **dois** porque os dois são verificáveis pelo próprio processo: o socket está escutando ou não está. Sumiu o antigo terceiro estado "REGRA SEM EFEITO" — o limbo em que a regra existia na config do `netsh` mas o IP Helper não subia listener nenhum.

Testado pela LAN nesta versão, em `http://192.168.0.101:8000/mcp`: `initialize` OK, `tools/list` com 55 ferramentas e `tools/call get_screen_size` executando de verdade (4096x2160 @ 1.75x).

### O que o relay não faz

- **Não abre porta no firewall.** Se o badge diz PUBLICADO e outra máquina não conecta, o problema é o Windows Defender Firewall — a GUI **não** cria regra de firewall. Ela avisa esse caso no console ("netstat mostra listener na LAN mas o teste TCP falhou (firewall?)").
- **Não autentica: ele repassa.** Com motor `0.16+`, quem chega pela LAN precisa do mesmo `Authorization: Bearer` que um cliente local precisaria. Com motor antigo, sem token, quem alcança a porta na sua rede controla mouse, teclado e tela desta máquina.

### Regras portproxy legadas

Máquinas que usaram versões anteriores têm regras `netsh` gravadas — e elas **sobrevivem a reiniciar o Windows**, mesmo que este app nunca mais as use. Quando existe uma regra para o par IP:porta atual, aparece um aviso e o botão **Remover regra antiga (UAC)**; quando não existe nenhuma, nada disso aparece na tela. A lista completa de regras da máquina continua no diagnóstico (seção 4).

## 4. Lendo o diagnóstico

A área rolável mostra apenas fatos verificados.

**Endpoint MCP HTTP (JSON-RPC) — estado real.** Tabela com porta, host, transporte e status. O host é o **real**: em "LOCAL APENAS" ele mostra `127.0.0.1 (loopback)`, não o IP que você gostaria de usar. O transporte é `HTTP / JSON-RPC` — não há WebSocket aqui, e a tela não anuncia o que não existe.

**URL de Conexão MCP (estado real).** Só mostra o IP da LAN quando `netstat` + POST confirmaram o listener na LAN; nos outros casos mostra a URL de loopback. Tem botão **Copiar**.

**Conexões reais na porta (`netstat -ano`).** As linhas cruas, com as mesmas colunas do terminal (`PROT / LOCAL / REMOTO / ESTADO / PID`). Inclui:

- `LISTENING` — soquetes em espera;
- `ESTABLISHED` — conexões MCP em andamento;
- listeners em portas altas (≥ 1024) no IP da LAN, para que um listener órfão de outra porta apareça em vez de ficar invisível. Portas de serviço do sistema (137/139/445...) são filtradas para não poluir.

Nota que evita a confusão mais comum: em um listener em espera, a coluna REMOTO aparece como `0.0.0.0:0`. Esse é o formato padrão do Windows para "aguardando conexões" — **não** é um destino nem indício de bind em todas as interfaces.

Com o relay publicado, o listener em `0.0.0.0:<porta>` aparece aqui com o **PID da própria GUI** — é ela quem está segurando o socket. Parar o relay (ou fechar o app) faz essa linha sumir.

**Regras portproxy existentes (`netsh show v4tov4`).** Aparece **só quando a máquina tem alguma**: todas as regras `v4tov4`, cruas — inclusive as que não são nossas e as órfãs em outras portas. Para limpar pela GUI uma regra do par IP:porta que está nos campos, use **Remover regra antiga (UAC)** na seção 3.

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

Se o comando 3 retornar **401**, o motor instalado é da série `0.16+` e exige token: acrescente `-H "Authorization: Bearer <token>"`. A GUI faz isso automaticamente quando encontra `CUA_DRIVER_RS_MCP_HTTP_TOKEN` em `HKCU\Environment`; se não existir token nenhum, gere um pela aba **Túnel** (botão **Gerar e ativar token do motor**).

## Ver também

- [acesso-remoto.md](acesso-remoto.md) — quando usar encaminhamento, quando usar túnel, quando usar VPN.
- [uso-tunel.md](uso-tunel.md) — expor o MCP na internet com senha e verificação de exposição.
- [solucao-de-problemas.md](solucao-de-problemas.md) — tabela sintoma -> causa -> verificação -> correção.
- [atualizacao.md](atualizacao.md) — o botão Verificar Atualizações e o que ele faz com cada componente.
