# Aba MCP & Rede — passo a passo

Para quem vai colocar o endpoint MCP de pé e precisa entender exatamente o que cada badge está afirmando.

Esta é a seção inicial do aplicativo. Ela tem três áreas: **controles do serviço** (topo, sempre visível), **configuração de porta/IP e encaminhamento** (meio, sempre visível) e **diagnóstico** (embaixo, única área que rola). A saída de todo comando vai para o console global do rodapé — comum a todas as seções. Código: `fzcomputerai/src/tabs/network.rs` e os métodos correspondentes em `fzcomputerai/src/app.rs`.

## 1. Controles do serviço CUA Driver

| Botão | Comando real | O que acontece depois |
| --- | --- | --- |
| **Iniciar** / *Start* | `cua-driver autostart kick` | refaz o teste do endpoint e atualiza o badge |
| **Parar** / *Stop* | `cua-driver stop` | refaz o teste do endpoint e atualiza o badge |
| **Reiniciar** / *Restart* | `cua-driver autostart kick` | idem (o `kick` é o caminho oficial do motor para religar sua tarefa) |
| **Testar Endpoint** / *Test Endpoint* | nenhum processo — sonda de rede própria | `POST /mcp` em `127.0.0.1` e no IP da LAN + `netstat -ano -p tcp` |
| **Iniciar com Windows** (checkbox) | `reg add`/`reg delete` em `HKCU\...\CurrentVersion\Run`, valor `FzComputerAI` | o checkbox é **relido do registro** depois de gravar, então ele reflete o estado real, não o clique |

O badge à direita do título tem três estados:

| Badge | Significado exato |
| --- | --- |
| **ATIVO (local + LAN) :porta** / *ACTIVE (local + LAN)* | o MCP respondeu JSON-RPC em `127.0.0.1:<porta>` **e** em `<IP_LAN>:<porta>`, **e** o `netstat` confirma o listener na LAN |
| **LOCAL apenas :porta** / *LOCAL only* | o MCP responde em `127.0.0.1` e nada mais. É o estado **normal e esperado** do motor oficial |
| **PARADO** / *STOPPED* | nada respondeu na porta configurada |

> "Iniciar com Windows" liga a **GUI** no logon. A tarefa de autostart do **motor** é assunto do próprio `cua-driver` (registrada pelo instalador oficial dele) — são coisas separadas.

## 2. Configuração de Porta & Rede

Dois campos e dois botões.

**Porta TCP HTTP (padrão 8000)** — é a porta do endpoint MCP HTTP do motor.

**Aplicar Porta** / *Apply Port* faz, nesta ordem, tudo logado:

1. grava `CUA_DRIVER_RS_MCP_HTTP_PORT` em `HKCU\Environment` via `[Environment]::SetEnvironmentVariable(..., 'User')`;
2. **relê o registro** com `reg query` e só considera sucesso se o valor conferir;
3. reinicia o motor (`cua-driver stop` seguido de `cua-driver autostart kick`);
4. registra no console a nota de que o motor oficial escuta **somente** em `127.0.0.1` e que nada é gravado para "bind";
5. se encontrar sobra de `CUA_DRIVER_RS_MCP_HTTP_BIND` no ambiente do usuário, **apaga** — configuração que não faz nada só atrapalha o diagnóstico;
6. refaz o teste do endpoint.

Sem essa variável definida, **o motor não sobe listener HTTP nenhum**. O botão não promete bind em todas as interfaces porque isso não existe no motor oficial (detalhes em [acesso-remoto.md](acesso-remoto.md)).

**Endereço IP da LAN (autodetectado, editável)** — a autodetecção não envia pacote: um `UdpSocket` faz `connect` para `8.8.8.8:80` apenas para o sistema escolher a interface de saída, e o IP local dessa interface é lido. Se a detecção cair em `127.0.0.1` (máquina sem rota externa), os testes de LAN e o encaminhamento são desativados com aviso no console — regra de loopback para loopback não faz sentido.

**Verificar e Atualizar** / *Check & Update* abre a Central de Atualizações, que cuida da GUI **e** do motor — e **age** em vez de só relatar: motor desatualizado é atualizado automaticamente de ponta a ponta (para o daemon antigo, aplica a última versão estável e religa o autostart, sem mais cliques); GUI desatualizada baixa o instalador em segundo plano com SHA256 conferido, e somente a troca final pede confirmação (exige fechar o aplicativo). Ver [atualizacao.md](atualizacao.md).

## 3. Encaminhamento LAN (portproxy)

O mapeamento exibido em destaque é literal:

```text
<IP_LAN>:<porta>  ->  127.0.0.1:<porta>
```

Isso é `netsh interface portproxy`, um recurso do Windows que **depende do serviço IP Helper (`iphlpsvc`)**. A criação da regra pode exigir elevação — nesse caso aparece um prompt de UAC.

| Botão | O que faz |
| --- | --- |
| **Aplicar Regra** / *Apply Rule* | cria a regra `<IP_LAN>:<porta> -> 127.0.0.1:<porta confirmada>` |
| **Remover Regra** / *Remove Rule* | apaga a regra do par IP:porta que estiver nos campos |
| **Atualizar Status** / *Refresh Status* | reexecuta o teste do endpoint, que já recalcula este badge |

### O que "Aplicar Regra" faz por dentro

1. **Recusa** se o IP for loopback ou a porta for inválida.
2. **Descobre a porta de destino real** (`detect_confirmed_cua_port`): testa, em ordem, a porta de `HKCU\Environment`, a porta do campo da UI e 8000 — e usa a **primeira que responder MCP de verdade** em `127.0.0.1`. Se nenhuma responder, **nenhuma regra é criada**: encaminhar para porta morta só mascararia o problema. A mensagem pede para iniciar o motor primeiro.
3. Se a regra já existir, pula o `add`.
4. Se já existir um **listener que não é regra portproxy** em `<IP_LAN>:<porta>` (outro processo usando a porta), **nada é alterado** — criar a regra conflitaria com um processo existente.
5. Executa `netsh interface portproxy add v4tov4 listenport=... listenaddress=... connectport=... connectaddress=127.0.0.1`. Se falhar, tenta de novo elevado por `Start-Process -Verb RunAs -Wait -PassThru`, propagando o exit code do `netsh` elevado (UAC cancelado ⇒ falha registrada, não silenciada).
6. **Relê** `netsh interface portproxy show v4tov4` e confirma a regra por comparação de tokens (nunca `contains()` solto).
7. Registra a regra como propriedade deste app em `HKCU\Software\FzComputerAI`, valor `portproxy:<ip>:<porta>`. **Só regras registradas aqui são removidas** na limpeza automática.
8. Confirma com `netstat` que `<IP_LAN>:<porta>` está `LISTENING`.
9. Refaz o teste do endpoint nos dois endereços.

### Os 3 estados do badge

| Badge | Regra na config do `netsh`? | Listener de pé no `netstat`? | Leitura |
| --- | --- | --- | --- |
| **REGRA FUNCIONANDO** / *RULE WORKING* | sim | sim | verificado dos dois lados |
| **REGRA SEM EFEITO** / *RULE NOT EFFECTIVE* | sim | **não** | a configuração existe mas o Windows não está servindo a porta |
| **SEM REGRA** / *NO RULE* | não | — | nada configurado para este par IP:porta |

### Resolvendo "REGRA SEM EFEITO"

Esse estado significa quase sempre que o **IP Helper** não subiu o listener. Na ordem:

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
5. Se o listener sobe mas outra máquina não conecta, o problema saiu do portproxy e virou **firewall**: a GUI avisa isso no console ("netstat mostra listener na LAN mas o teste TCP falhou (firewall?)"). É preciso liberar a porta de entrada no Windows Defender Firewall — a GUI **não** cria regra de firewall.

## 4. Lendo o diagnóstico

A área rolável mostra apenas fatos verificados.

**Endpoint MCP HTTP (JSON-RPC) — estado real.** Tabela com porta, host, transporte e status. O host é o **real**: em "LOCAL APENAS" ele mostra `127.0.0.1 (loopback)`, não o IP que você gostaria de usar. O transporte é `HTTP / JSON-RPC` — não há WebSocket aqui, e a tela não anuncia o que não existe.

**URL de Conexão MCP (estado real).** Só mostra o IP da LAN quando `netstat` + POST confirmaram o listener na LAN; nos outros casos mostra a URL de loopback. Tem botão **Copiar**.

**Conexões reais na porta (`netstat -ano`).** As linhas cruas, com as mesmas colunas do terminal (`PROT / LOCAL / REMOTO / ESTADO / PID`). Inclui:

- `LISTENING` — soquetes em espera;
- `ESTABLISHED` — conexões MCP em andamento;
- listeners em portas altas (≥ 1024) no IP da LAN, para que um listener órfão de outra porta apareça em vez de ficar invisível. Portas de serviço do sistema (137/139/445...) são filtradas para não poluir.

Nota que evita a confusão mais comum: em um listener em espera, a coluna REMOTO aparece como `0.0.0.0:0`. Esse é o formato padrão do Windows para "aguardando conexões" — **não** é um destino nem indício de bind em todas as interfaces.

**Regras portproxy existentes (`netsh show v4tov4`).** Todas as regras `v4tov4` da máquina, cruas — inclusive as que não são nossas e as órfãs em outras portas. Para limpar uma órfã pela GUI: ajuste os campos Porta e IP para os da regra e use **Remover Regra**.

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

Se o comando 3 retornar **401**, o motor instalado é da série `0.16+` e exige token: acrescente `-H "Authorization: Bearer <token>"`. A GUI faz isso automaticamente quando encontra `CUA_DRIVER_RS_MCP_HTTP_TOKEN` em `HKCU\Environment`.

## Ver também

- [acesso-remoto.md](acesso-remoto.md) — quando usar encaminhamento, quando usar túnel, quando usar VPN.
- [uso-tunel.md](uso-tunel.md) — expor o MCP na internet com senha e verificação de exposição.
- [solucao-de-problemas.md](solucao-de-problemas.md) — tabela sintoma -> causa -> verificação -> correção.
- [atualizacao.md](atualizacao.md) — o botão Verificar e Atualizar e o que ele faz com cada componente.
