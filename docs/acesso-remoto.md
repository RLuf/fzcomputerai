# Acesso remoto ao MCP

Para quem precisa que algo **fora desta máquina** fale com o endpoint MCP e quer escolher o caminho com os olhos abertos.

## 1. O ponto de partida: o motor só escuta em loopback

O `cua-driver` expõe o MCP HTTP em `127.0.0.1:<porta>` e **em nenhum outro endereço**. Isso não é configuração, é o comportamento do motor oficial.

## 2. Por que não existe bind `0.0.0.0`

| Fato | Como foi verificado |
| --- | --- |
| O endereço de escuta está **fixo no código do Cua** como `([127,0,0,1], port)` | leitura do upstream `trycua/cua` |
| **Não existe** variável `CUA_DRIVER_RS_MCP_HTTP_BIND` | a string não aparece no binário oficial instalado (0.8.3) **e** a busca por ela no repositório `trycua/cua` retorna zero ocorrência |
| A única variável que muda o listener é `CUA_DRIVER_RS_MCP_HTTP_PORT` | sem ela, **o listener HTTP não sobe**; com ela, sobe — em loopback |

Uma versão anterior da documentação deste projeto afirmava que havia bind `0.0.0.0`. **Era falso.** Foi corrigido no código e no texto. O que aconteceu na prática: a GUI gravava uma variável que o motor **ignora**, e a tela sugeria LAN onde não havia LAN. Hoje o botão se chama apenas **Aplicar Porta**, o console registra a nota explicando o limite, e a GUI **apaga** a variável morta se encontrar sobra dela em `HKCU\Environment`.

**Se você está pensando em reintroduzir a ideia:** não grave a variável por suposição. O critério do projeto é verificação real — se algum dia o upstream aceitar bind configurável, a mudança entra junto com confirmação no `netstat`, não antes. O comentário longo em `apply_env_port()` (`fzcomputerai/src/app.rs`) está lá para isso.

Um detalhe de leitura que causa falso positivo: no `netstat`, um listener em espera mostra `0.0.0.0:0` na coluna **REMOTO**. Isso é o formato do Windows para "aguardando conexões", não um bind em todas as interfaces. O bind é a coluna **LOCAL**.

## 3. As três formas reais de sair do loopback

| | LAN — relay do app | Internet — túnel de saída | VPN |
| --- | --- | --- | --- |
| **Alcance** | quem está na mesma rede local | qualquer lugar com a URL | quem está na sua VPN |
| **Onde é feito** | dentro da GUI, aba MCP & Rede | dentro da GUI, aba Túnel | fora da GUI, por software de VPN |
| **Abre porta de entrada?** | sim, na sua máquina, para a LAN | **não** — a conexão é iniciada por esta máquina | não expõe à internet aberta |
| **Depende de** | nada além do processo da GUI; o firewall do Windows ainda precisa permitir a entrada | `cloudflared` / `ngrok` / `ssh` + serviço do provedor | infraestrutura de VPN sua ou de terceiro |
| **Pede UAC?** | **não** (desde a 2.3.0; o `netsh portproxy` pedia) | não | depende do software de VPN |
| **Autenticação embutida** | nenhuma — quem alcança o IP:porta na LAN fala com o MCP | depende do que você ligar: senha na URL, Access, basic-auth | a da própria VPN (normalmente forte) |
| **Sobrevive ao fechar a GUI?** | não: o relay é uma thread do próprio processo | não: o processo é filho adotado pelo Job Object e morre com o app | sim, é independente do app |
| **Deixa rastro na máquina?** | não deixa regra nenhuma no sistema | valores `tunnel:*` em `HKCU`, removidos na limpeza | a configuração da VPN |
| **Custo de operação** | zero | zero a baixo; contas gratuitas têm limites | precisa manter a VPN |

### 3.1 LAN pelo relay do próprio app (2.3.0+)

O caminho padrão deixou de ser o `netsh portproxy`. A GUI sobe um **relay TCP dentro do próprio processo**: escuta em `0.0.0.0:<porta>` — ou num IP específico, campo **Escutar em** — e encaminha para `127.0.0.1:<porta do motor confirmada>`. Ele copia bytes nos dois sentidos sem inspecionar HTTP, então keep-alive e SSE passam intactos.

Três ganhos medidos, contra a regra `netsh`: **não pede UAC**; **não deixa regra no sistema** (a do `netsh` sobrevive a reiniciar o Windows); e **morre com o app**, porque é thread dele. E há uma medição que torna tudo mais simples: nesta plataforma `0.0.0.0:<porta>` **coexiste** com o `127.0.0.1:<porta>` do motor, então a publicação usa a **mesma** porta, sem tocar na configuração do motor.

O relay **não publica porta morta**: se o MCP não responder em `127.0.0.1`, nada sobe e a mensagem diz isso. Enquanto está ativo, a tela mostra o badge **PUBLICADO NA REDE** e o contador real de conexões (ativas / total desde o início).

Testado pela LAN em `http://192.168.0.101:8000/mcp`: `initialize` OK, `tools/list` com 55 ferramentas e `tools/call get_screen_size` executando de verdade (4096x2160 @ 1.75x).

Se a sua máquina usou versões anteriores, pode haver uma regra `portproxy` antiga persistida — ela sobrevive a reboot. A GUI mostra o aviso e o botão de remoção **só quando existe alguma**; a remoção ainda pede UAC, porque é o `netsh` que exige.

Passo a passo: [uso-mcp-rede.md](uso-mcp-rede.md).

### 3.2 Internet por túnel de saída

`cloudflared` (quick ou nomeado), `ngrok` ou `ssh -R`, com URL pública capturada do log do próprio CLI e, opcionalmente, um porteiro de senha local que exige `/s/<senha>/` no caminho. A GUI verifica a exposição real com um `POST initialize` na URL pública — nunca presume. Desde a 2.2.0 a sonda tem **duas fases**: primeiro sem credencial nenhuma e depois, quando a GUI conhece o token do motor, com `Authorization: Bearer` — o badge **PROTEGIDO E FUNCIONAL** significa que as duas coisas foram provadas: sem credencial, barrado; com Bearer, `initialize` OK. Na 2.4.0 esse teste roda em **segundo plano**: são dois `curl -m 20`, até 40 s, que antes congelavam a janela.

Estado medido de cada provedor:

| Provedor | Estado real |
| --- | --- |
| Cloudflare **quick tunnel** | **funciona** — sem conta, URL aleatória `*.trycloudflare.com`. Testado pela internet: sem credencial, 401; com Bearer, `initialize` OK e `tools/call` executando |
| Cloudflare **nomeado com domínio próprio** | implementado na 2.4.0, fluxo completo pela GUI (ver 3.2.1) |
| **ngrok** | depende de authtoken válido. Nesta máquina o token em `%LOCALAPPDATA%\ngrok\ngrok.yml` é **inválido** (`ERR_NGROK_105`) e o túnel morre ao autenticar. O `ngrok config check` passa porque valida a **sintaxe** do arquivo, não o token — a GUI detecta o código no log e ensina `ngrok config add-authtoken <TOKEN>`. O que falta é credencial válida, não código |
| **SSH reverso** | depende do seu servidor e de autenticação por chave (`BatchMode=yes`, sem prompt de senha) |

A chave do ngrok **não** fica no registro do Windows: varredura de `HKCU` e `HKLM` não encontrou authtoken nenhum — ele vive no `ngrok.yml`.

#### 3.2.1 Cloudflare com domínio próprio (URL fixa)

O quick tunnel dá uma URL aleatória que muda a cada start. Para uma URL **fixa** — por exemplo `mcphome.rogerluft.com.br` — o fluxo inteiro está na aba Túnel, na ordem:

1. **Login Cloudflare (OAuth)** — abre o navegador e você escolhe o domínio. **O login sozinho não cria nada**: ele só baixa o `cert.pem`;
2. **Verificar login** — confere de verdade se existe `~/.cloudflared/cert.pem`. Sem isso, os passos seguintes não têm autorização;
3. preencher **Nome do túnel** e **Hostname público**;
4. **Criar túnel + apontar DNS** — roda `cloudflared tunnel create` e `cloudflared tunnel route dns`, em segundo plano;
5. **Iniciar túnel** — passa a rodar `cloudflared tunnel run --url http://127.0.0.1:<porta> <nome>`.

Requisito que não dá para contornar pela GUI: **o domínio precisa estar na sua conta Cloudflare**, com os nameservers delegados. Sem isso o passo 4 falha na hora de criar o DNS, e o console mostra a recusa do próprio `cloudflared`.

#### 3.2.2 Quando o cliente MCP só aceita uma URL

Clientes como o Claude Desktop pedem **uma URL** e ponto — não há onde colar um header `Authorization`. Como o motor `0.16+` é *fail-closed* (401 sem Bearer), esses clientes ficariam de fora.

Na 2.4.0 o **porteiro de senha injeta o Bearer**: quem provou a senha no caminho (`/s/<senha>/mcp`) já está autenticado perante o app, então o porteiro acrescenta o `Authorization` ao falar com o motor. Se o cliente mandar o próprio `Authorization`, o dele vence.

O que isso muda no seu modelo de ameaça, dito sem rodeio: **o segredo do motor não viaja pela internet**, e **a credencial pública passa a ser a senha da URL**. Todos os limites da senha na URL continuam valendo (ver seção 5) — ela vai no caminho, aparece em log de quem estiver no meio, é gerada por PRNG simples e não tem rotação nem expiração.

Testado: URL com senha e **sem nenhum header** — `initialize` OK e `tools/call get_screen_size` executou (4096x2160). Senha errada -> 404. Sem senha -> 404.

Passo a passo, tabela de provedores e o significado de cada badge: [uso-tunel.md](uso-tunel.md).

### 3.3 VPN

Não há nada a fazer na GUI: com a VPN de pé, a outra máquina alcança o IP da interface de VPN desta máquina. Como o MCP escuta apenas em loopback, você **ainda precisa** publicar o relay — e aqui está a vantagem de restringir: coloque o **IP da interface de VPN** no campo **Escutar em** em vez de `0.0.0.0`, e o relay atende só por ali. O alcance fica restrito à VPN e a autenticação é a dela, tipicamente muito mais forte que qualquer coisa que a GUI possa oferecer.

## 4. Recomendação por cenário

| Cenário | Recomendação |
| --- | --- |
| Agente rodando **nesta máquina** (cliente MCP local) | não faça nada: use `127.0.0.1:<porta>` ou o transporte **stdio**. Não abra nada |
| Outra máquina **na mesma rede**, rede confiável (casa, laboratório) | **Publicar na rede** (relay). Lembre que na LAN **não há autenticação nenhuma**: qualquer dispositivo da rede que alcance a porta controla esta máquina |
| Outra máquina na mesma rede, rede **não** confiável (escritório compartilhado, Wi-Fi de coworking) | VPN, ou túnel com senha. Publicar direto na LAN é o pior dos casos: alcance amplo e zero autenticação. Se for publicar mesmo assim, restrinja o campo **Escutar em** a uma interface só |
| Agente **fora da rede**, uso pontual, você acompanhando | túnel Cloudflare quick **com senha na URL**, e rode a sonda de exposição para confirmar. Pare o túnel ao terminar |
| Agente fora da rede, uso **recorrente**, você quer sempre a **mesma URL** | túnel Cloudflare **nomeado com seu domínio** (3.2.1) + **Cloudflare Access** (autenticação na borda, antes de chegar à sua máquina), ou SSH reverso no **seu** servidor com autenticação no proxy |
| O cliente MCP **só aceita uma URL**, sem header | túnel **com senha na URL** (`/s/<senha>/mcp`): o porteiro injeta o Bearer do motor por você (3.2.2). A senha passa a ser a credencial pública — trate-a como tal e troque-a reiniciando o túnel |
| Você já usa ngrok | ngrok com **basic-auth** pela *traffic policy* que a GUI gera. Atenção: com token do motor ativo a borda é **ignorada** no start — basic-auth e Bearer disputam o mesmo header `Authorization` e o cliente MCP só envia um; nesse caso a proteção do túnel é o Bearer do motor |
| Infraestrutura corporativa | VPN + encaminhamento restrito ao IP da VPN. É o único arranjo em que a autenticação não depende deste app |
| Você quer "deixar sempre ligado" | não deixe. O túnel morre com o app **de propósito**. Precisar de disponibilidade permanente é sinal de que o lugar certo é um servidor, com autenticação de verdade na frente |

## 5. Implicações de segurança, sem eufemismo

**O que está em jogo.** O MCP do `cua-driver` executa clique, digitação, atalhos de teclado, captura de tela, leitura da árvore de acessibilidade, inicialização e encerramento de aplicativos. Quem consegue chamar esse endpoint **opera este computador** como se estivesse sentado nele — inclusive lendo o que estiver na tela, com a sua sessão de usuário já autenticada em tudo.

**Loopback.** Alcançável por qualquer processo desta máquina. Não há autenticação por processo; o limite é a fronteira da máquina.

**LAN pelo relay.** Alcançável por qualquer dispositivo da rede local que chegue a `IP:porta`. **Nenhuma autenticação é adicionada** — o relay repassa bytes, não pergunta quem é. Numa rede com convidados, câmeras, TVs e celulares, "rede local" é um conjunto maior do que parece. Que o relay morra com o app e não deixe regra no sistema é ganho de **higiene**, não de segurança: enquanto ele está de pé, a exposição é a mesma. Mitigações reais: usar VPN em vez disso, restringir o campo **Escutar em** a uma interface, restringir por firewall, ou aceitar o risco em rede realmente controlada.

**Internet por túnel.** A URL pública é alcançável pelo mundo inteiro. Três níveis, do pior ao melhor:

1. **sem nada** — URL aleatória apenas. A GUI classifica isso como **EXPOSTO SEM AUTENTICAÇÃO** quando a sonda confirma, e o aviso vermelho no topo da aba diz por que uma URL aleatória não é proteção: ela vaza em log, histórico, print de tela e arquivo de config do cliente MCP;
2. **senha na URL (porteiro local)** — bloqueia varredura e acesso casual: sem `/s/<senha>/` o porteiro responde 404 e o MCP nunca é tocado. Limites honestos: a senha viaja **no caminho da URL**, então aparece em logs de quem estiver no meio do caminho (a borda do provedor vê o path); a senha é gerada por PRNG simples, não por gerador criptográfico; e ela é uma credencial única, sem rotação nem expiração. **E desde a 2.4.0 ela carrega mais peso**: o porteiro injeta o Bearer do motor para quem passou pela senha (3.2.2), então quem tiver a URL completa entra sem precisar de mais nada. Em troca, o token do motor deixa de trafegar pela internet;
3. **autenticação de borda** (Cloudflare Access, basic-auth do ngrok, proxy autenticado no seu servidor) — a checagem acontece **antes** de a requisição chegar à sua máquina. É o nível mais forte que este app consegue apoiar. A sonda mostra **BORDA EXIGIU AUTENTICAÇÃO (HTTP nnn)** quando a barreira responde.

**O que muda com o token do motor novo.** Na série `0.16+`, o próprio motor exige `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32 a 4096 caracteres) e responde **401** a qualquer POST sem `Authorization: Bearer <token>` — o corpo do 401 é um erro JSON-RPC (`"Authentication required"`), não uma página de borda; ele também rejeita requisições com origem de navegador. E note: **sem nenhum token configurado o endpoint é fail-closed** — 401 para tudo. Não é "aberto": o túnel sobe, mas nenhum cliente entra. Isso é uma melhora real: mesmo que a URL vaze, sem o token não há chamada. Mas note bem:

- **não substitui** a proteção de borda — o token trafega em todo request, e quem tiver **URL + token** controla a máquina. Trate o token como senha;
- motores **<= 0.8.x** **não têm token nenhum** — por isso o instalador passou a exigir **0.16.0 como versão mínima**. Não presuma proteção pela versão que você imagina ter: confira a versão real na Central de Atualizações;
- a GUI **lê** o token de `HKCU\Environment` na abertura **e ao abrir a aba Túnel** (2.2.0) e o envia como Bearer em todos os testes — o valor nunca aparece no log. No Windows, quando o motor exige token e não há nenhum configurado (ou o conhecido é recusado), a aba Túnel oferece o botão **Gerar e ativar token do motor**: gera um token criptográfico (CSPRNG, >=32 caracteres), grava com confirmação em `HKCU\Environment` e reinicia o motor;
- com o token ativo, o snippet `mcpServers` que a GUI copia **já inclui** o header `"Authorization": "Bearer <token>"`. A senha na URL (`/s/<senha>/`) continua compatível como camada adicional: ela vai no **caminho**, o token vai no **header** — não disputam o mesmo lugar.

**O que a GUI não faz** (e não vai fingir que faz): não cria regra de firewall, não gerencia certificado, não faz rotação de credencial, não registra quem chamou o MCP, e não limita taxa. Se o seu cenário precisa de auditoria e controle de acesso, o lugar disso é a borda ou uma VPN — não este aplicativo.

**Higiene mínima recomendada.** Abra o mínimo pelo menor tempo possível; prefira senha ou Access sempre que a URL for pública; rode a sonda de exposição depois de subir e depois de qualquer mudança; feche o app quando terminar (o relay, o porteiro, o túnel e o motor caem com ele, por design — o Job Object garante isso pelo kernel); e olhe o console na abertura — se aparecer "TUNEL ORFAO encerrado", a máquina esteve exposta desde a sessão anterior.

**Conferindo de fora, de verdade.** A sonda da GUI sai desta mesma máquina. Para provar o caminho inteiro a partir de **outra** rede, o repositório traz `scripts/remote-teste.py`, que só usa a biblioteca padrão do Python 3:

```bash
python remote-teste.py <URL> [--token TOKEN] [--termo TEXTO]
```

Ele faz `initialize`, `tools/list`, abre uma janela **nova** de navegador na máquina remota (nunca sequestra uma janela existente), navega para o buscador, digita o termo, encontra e clica no botão de pesquisa (ou envia Enter) e confere o resultado **lendo a tela de volta**. Se a URL já tiver a senha (`/s/<senha>/mcp`), o `--token` não é necessário. Lembre do óbvio: rodar esse teste é operar o computador remoto de verdade.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — aplicar porta, publicar na rede pelo relay e leitura do diagnóstico cru.
- [uso-tunel.md](uso-tunel.md) — provedores, senha na URL, sonda de exposição e ciclo de vida.
- [arquitetura.md](arquitetura.md) — o transporte MCP, o relay, o ciclo de vida pelo Job Object e o princípio de status honesto.
- [faq.md](faq.md) — "o túnel é seguro?", "preciso deixar o app aberto?", "qual a diferença entre a porta e publicar na rede?".
