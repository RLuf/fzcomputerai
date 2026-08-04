# Acesso remoto ao MCP

Para quem precisa que algo **fora desta máquina** fale com o endpoint MCP e quer escolher o caminho com os olhos abertos.

## 1. O ponto de partida: o motor só escuta em loopback

O `cua-driver` expõe o MCP HTTP em `127.0.0.1:<porta>` e **em nenhum outro endereço**. Isso não é configuração, é o comportamento do motor oficial.

## 2. Por que não existe bind `0.0.0.0`

| Fato | Como foi verificado |
| --- | --- |
| O endereço de escuta está **fixo no código do Cua** como `([127,0,0,1], port)` | leitura do upstream `trycua/cua` |
| **Não existe** variável `CUA_DRIVER_RS_MCP_HTTP_BIND` | a string não aparece no binário oficial (verificado na 0.8.3) **e** a busca por ela no repositório `trycua/cua` retorna zero ocorrência |
| A única variável que muda o listener é `CUA_DRIVER_RS_MCP_HTTP_PORT` | sem ela, **o listener HTTP não sobe**; com ela, sobe — em loopback |

Uma versão anterior da documentação deste projeto afirmava que havia bind `0.0.0.0`. **Era falso.** Foi corrigido no código e no texto. O que aconteceu na prática: a GUI gravava uma variável que o motor **ignora**, e a tela sugeria LAN onde não havia LAN. Hoje o botão se chama apenas **Aplicar Porta**, o console registra a nota explicando o limite, e a GUI **apaga** a variável morta se encontrar sobra dela em `HKCU\Environment`.

**Se você está pensando em reintroduzir a ideia:** não grave a variável por suposição. O critério do projeto é verificação real — se algum dia o upstream aceitar bind configurável, a mudança entra junto com confirmação no `netstat`, não antes. O comentário longo em `apply_env_port()` (`fzcomputerai/src/app.rs`) está lá para isso.

Um detalhe de leitura que causa falso positivo: no `netstat`, um listener em espera mostra `0.0.0.0:0` na coluna **REMOTO**. Isso é o formato do Windows para "aguardando conexões", não um bind em todas as interfaces. O bind é a coluna **LOCAL**.

## 3. As três formas reais de sair do loopback

| | LAN — encaminhamento do app | Internet — túnel de saída | VPN |
| --- | --- | --- | --- |
| **Alcance** | quem está na mesma rede local | qualquer lugar com a URL | quem está na sua VPN |
| **Onde é feito** | dentro da GUI, aba MCP & Rede | dentro da GUI, aba Túnel | fora da GUI, por software de VPN |
| **Abre porta de entrada?** | sim, na sua máquina, para a LAN | **não** — a conexão é iniciada por esta máquina | não expõe à internet aberta |
| **Depende de** | nada além do próprio app e do firewall do Windows — é uma thread do processo, sem elevação (só o **fallback** `netsh` depende do IP Helper e pode pedir UAC) | `cloudflared` / `ngrok` / `ssh` + serviço do provedor | infraestrutura de VPN sua ou de terceiro |
| **Autenticação embutida** | nenhuma no encaminhamento — a única barreira é o token do motor | depende do que você ligar: senha na URL, Access, basic-auth | a da própria VPN (normalmente forte) |
| **Sobrevive ao fechar a GUI?** | não: o listener é do próprio processo e morre com ele — nada fica gravado no Windows | não: quatro camadas garantem que o processo morra com o app | sim, é independente do app |
| **Custo de operação** | zero, já vem no app | zero a baixo; contas gratuitas têm limites | precisa manter a VPN |

### 3.1 LAN por encaminhamento (feito pelo próprio app)

Desde a v2.1.1 o encaminhamento é do **aplicativo**, não do Windows: uma thread do processo escuta em `<IP_LAN>:<porta>` e copia bytes contra `127.0.0.1:<porta confirmada>` (`std::net::TcpListener` + `std::io::copy`, sem dependência nova). É TCP puro — `curl`, `telnet` e `nc` atravessam igual. A GUI confirma o listener no `netstat` e o `POST initialize` nos dois endereços antes de pintar o badge de verde.

Por que deixou de ser `netsh interface portproxy`: aquilo é regra **estática** do serviço IP Helper — exigia admin/UAC para criar e remover, continuava mostrando `LISTENING` na LAN **mesmo com o motor morto** (aceitando conexões que morriam no destino, ou seja, falso positivo de serviço no ar) e **sobrevivia** ao fechamento do app e ao reboot, o que obrigava uma rotina de limpeza. Medido em 2026-08-03: com o app aberto, `127.0.0.1:8000` e `192.168.0.101:8000` em `LISTENING`, MCP respondendo HTTP 200 nos dois e `netsh interface portproxy show v4tov4` **sem nenhuma regra**; ao fechar o app, as duas portas fecharam junto.

O `netsh` continua como **fallback**, usado só quando o bind no IP da LAN falha. Nesse caso a regra é registrada como propriedade do app em `HKCU\Software\FzComputerAI` (`portproxy:<ip>:<porta>`) e **só regras registradas assim são removidas** na limpeza — regras de outros serviços nesta máquina não são tocadas.

A limpeza ao fechar também mudou na v2.1.1: virou código Rust nativo, com chamadas diretas e curtas, sem elevação e sem PowerShell. A versão anterior disparava um `powershell -WindowStyle Hidden` de uns 2 KB que esperava o processo morrer, matava processos, escrevia no registro, rodava `netsh` e ainda chamava `-Verb RunAs` — linha de comando que o Microsoft Defender **flagrou** nesta máquina (detecção 2147941383, 2026-08-03). No mesmo passo saiu o `taskkill /F /IM cua-driver.exe` do app e do instalador: ele matava **todo** processo com esse nome, inclusive um motor que o usuário estivesse usando para outra coisa. Agora é `cua-driver stop` (o comando oficial) e, se ainda sobrar processo, `kill` por PID com o caminho do executável conferido antes.

Passo a passo e os 3 estados do badge: [uso-mcp-rede.md](uso-mcp-rede.md).

### 3.2 Internet por túnel de saída

`cloudflared` (quick ou nomeado), `ngrok` ou `ssh -R`, com URL pública capturada do log do próprio CLI e, opcionalmente, um porteiro de senha local que exige `/s/<senha>/` no caminho. A GUI verifica a exposição real com um `POST initialize` na URL pública — nunca presume.

Passo a passo, tabela de provedores e o significado de cada badge: [uso-tunel.md](uso-tunel.md).

### 3.3 VPN

Não há nada a fazer na GUI: com a VPN de pé, a outra máquina alcança o IP da interface de VPN desta máquina. Como o MCP escuta apenas em loopback, você **ainda precisa** do encaminhamento do IP da VPN para `127.0.0.1` — coloque o IP da interface de VPN no campo IP da aba MCP & Rede e aplique. A vantagem é que o alcance fica restrito à VPN e a autenticação é a dela, tipicamente muito mais forte que qualquer coisa que a GUI possa oferecer.

## 4. Recomendação por cenário

| Cenário | Recomendação |
| --- | --- |
| Agente rodando **nesta máquina** (cliente MCP local) | não faça nada: use `127.0.0.1:<porta>` ou o transporte **stdio**. Não abra nada |
| Outra máquina **na mesma rede**, rede confiável (casa, laboratório) | encaminhamento LAN. Lembre que o encaminhamento **não adiciona autenticação nenhuma**: a única barreira é o token do motor, e qualquer dispositivo da rede que alcance a porta **com o token** controla esta máquina |
| Outra máquina na mesma rede, rede **não** confiável (escritório compartilhado, Wi-Fi de coworking) | VPN, ou túnel com senha. Encaminhamento puro na LAN é o pior dos casos: alcance amplo e nada além do token na frente |
| Agente **fora da rede**, uso pontual, você acompanhando | túnel Cloudflare quick **com senha na URL**, e rode a sonda de exposição para confirmar. Pare o túnel ao terminar |
| Agente fora da rede, uso **recorrente** | túnel Cloudflare **nomeado** + **Cloudflare Access** (autenticação na borda, antes de chegar à sua máquina), ou SSH reverso no **seu** servidor com autenticação no proxy |
| Você já usa ngrok | ngrok com **basic-auth** pela *traffic policy* que a GUI gera |
| Infraestrutura corporativa | VPN + encaminhamento restrito ao IP da VPN. É o único arranjo em que a autenticação não depende deste app |
| Você quer "deixar sempre ligado" | não deixe. O túnel morre com o app **de propósito**. Precisar de disponibilidade permanente é sinal de que o lugar certo é um servidor, com autenticação de verdade na frente |

## 5. Implicações de segurança, sem eufemismo

**O que está em jogo.** O MCP do `cua-driver` executa clique, digitação, atalhos de teclado, captura de tela, leitura da árvore de acessibilidade, inicialização e encerramento de aplicativos. Quem consegue chamar esse endpoint **opera este computador** como se estivesse sentado nele — inclusive lendo o que estiver na tela, com a sua sessão de usuário já autenticada em tudo.

**Loopback.** Alcançável por qualquer processo desta máquina. Não há autenticação por processo; o limite é a fronteira da máquina.

**LAN por encaminhamento.** Alcançável por qualquer dispositivo da rede local que chegue a `IP:porta`. **Nenhuma autenticação é adicionada pelo encaminhamento** — a thread do app copia bytes e nada mais; o `netsh`, no fallback, também não filtra nada. A única barreira que existe nesse caminho é o token do motor (adiante). Numa rede com convidados, câmeras, TVs e celulares, "rede local" é um conjunto maior do que parece. Mitigações reais: usar VPN em vez disso, restringir por firewall, ou aceitar o risco em rede realmente controlada.

**Internet por túnel.** A URL pública é alcançável pelo mundo inteiro. Três níveis, do pior ao melhor:

1. **sem nada** — URL aleatória apenas. A GUI classifica isso como **EXPOSTO SEM AUTENTICAÇÃO** quando a sonda confirma, e o aviso vermelho no topo da aba diz por que uma URL aleatória não é proteção: ela vaza em log, histórico, print de tela e arquivo de config do cliente MCP;
2. **senha na URL (porteiro local)** — bloqueia varredura e acesso casual: sem `/s/<senha>/` o porteiro responde 404 e o MCP nunca é tocado. Limites honestos: a senha viaja **no caminho da URL**, então aparece em logs de quem estiver no meio do caminho (a borda do provedor vê o path); a senha é gerada por PRNG simples, não por gerador criptográfico; e ela é uma credencial única, sem rotação nem expiração;
3. **autenticação de borda** (Cloudflare Access, basic-auth do ngrok, proxy autenticado no seu servidor) — a checagem acontece **antes** de a requisição chegar à sua máquina. É o nível mais forte que este app consegue apoiar. A sonda mostra **BORDA EXIGIU AUTENTICAÇÃO (HTTP nnn)** quando a barreira responde.

**O que muda com o token do motor novo.** Isto foi **medido em 2026-08-03 no binário `cua-driver` 0.17.0** desta máquina — não é documentação citando documentação, é o que o motor fez. Sem `CUA_DRIVER_RS_MCP_HTTP_TOKEN` no ambiente do processo, o `cua-driver serve` **nem sobe**: sai com código 1 e o stderr `CUA_DRIVER_RS_MCP_HTTP_TOKEN must be set to a host-generated bearer token when the HTTP endpoint is enabled`. Com o daemon no ar, qualquer requisição sem `Authorization: Bearer <token>` recebe **HTTP 401** com o corpo `{"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"Authentication required"}}` — idêntico em `POST /mcp`, `GET /mcp` e `GET /`, e **sem** header `WWW-Authenticate`. Com o header correto, **HTTP 200** com o `result` do `initialize`. A conexão TCP em si é aceita normalmente (a porta responde ao teste de conexão); a recusa acontece na camada de aplicação. Isso é uma melhora real: mesmo que a URL vaze, sem o token não há chamada. Mas note bem:

- **não substitui** a proteção de borda — o token trafega em todo request, e quem tiver **URL + token** controla a máquina. Trate o token como senha;
- motores antigos (por exemplo, **0.8.3**) **não têm token nenhum**. O instalador não pina versão de motor: ele executa o instalador oficial do projeto Cua, que instala a **última versão estável** publicada. Ainda assim, não presuma proteção pela versão que você imagina ter: confira a versão real na Central de Atualizações;
- o token é gerado pelo **host**, e o host é este aplicativo: o motor chama o valor de *host-generated bearer token* e não tem comando para produzi-lo (nem o `cua-driver`, nem o `install.ps1` oficial) — verificado em 2026-08-03;
- a partir da v2.1.1 a GUI **gera** o token: 32 bytes do RNG do Windows (64 caracteres hex), persistidos em `HKCU\Environment` na primeira vez que ela precisa. Você não precisa saber que a variável existe, e a GUI **nunca** imprime o valor no console. Para lê-lo: `reg query "HKCU\Environment" /v CUA_DRIVER_RS_MCP_HTTP_TOKEN`;
- desde a v2.1.1 quem sobe o motor é a própria GUI, como **processo filho**, com porta e token injetados no ambiente do filho — o token vale já nesta sessão, sem depender de novo logon. A Tarefa Agendada `cua-driver-serve` ficou como último recurso, e ela **herda o ambiente do logon**: token gravado depois de você já ter entrado na sessão só é visto no próximo logon, e até lá o daemon sobe sem token, morre na hora e deixa a porta muda. Por isso, quando é a tarefa que sobe o motor, o console avisa que o processo não é da GUI e que não haverá logs.

**O que a GUI não faz** (e não vai fingir que faz): não cria regra de firewall, não gerencia certificado, não faz rotação de credencial, não registra quem chamou o MCP, e não limita taxa. Se o seu cenário precisa de auditoria e controle de acesso, o lugar disso é a borda ou uma VPN — não este aplicativo.

**Higiene mínima recomendada.** Abra o mínimo pelo menor tempo possível; prefira senha ou Access sempre que a URL for pública; rode a sonda de exposição depois de subir e depois de qualquer mudança; feche o app quando terminar (o encaminhamento e o túnel caem com ele, por design); e olhe o console na abertura — se aparecer "TUNEL ORFAO encerrado", a máquina esteve exposta desde a sessão anterior.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — aplicar porta, encaminhamento LAN e leitura do diagnóstico cru.
- [uso-tunel.md](uso-tunel.md) — provedores, senha na URL, sonda de exposição e ciclo de vida.
- [arquitetura.md](arquitetura.md) — o transporte MCP e o princípio de status honesto.
- [faq.md](faq.md) — "o túnel é seguro?", "qual a diferença entre a porta e o encaminhamento?".
