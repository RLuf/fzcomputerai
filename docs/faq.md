# Perguntas frequentes

Para quem quer a resposta curta antes de ler o documento inteiro.

### Preciso do `cua-driver`?

Sim. Toda ação da interface termina em uma invocação de `cua-driver` como processo filho. Sem o motor instalado e no PATH, a janela abre, o console registra o erro de execução e nenhum botão produz efeito. O motor é instalado **à parte**, pelo instalador oficial do Cua — o instalador do FzComputerAI traz o componente `engine` (marcado por padrão) que dispara esse instalador como passo real da instalação, **inclusive no modo silencioso** (`/VERYSILENT`); para pular, use `/SKIPENGINE`.

### A GUI é o motor?

Não. `fzcomputerai` é a interface: ela **inicia, para, configura, diagnostica e expõe** o motor. Quem clica, digita, tira screenshot e lê a árvore de acessibilidade é o `cua-driver`, do projeto [Cua](https://github.com/trycua/cua) (MIT, Cua AI, Inc.). São dois produtos, duas versões e dois atualizadores — a Central de Atualizações mostra os dois lado a lado.

### Por que o Windows avisa que o programa é inseguro?

Porque os binários **não são assinados** com certificado de código. O SmartScreen avisa sobre qualquer executável sem assinatura e sem reputação estabelecida — o aviso é sobre a ausência de assinatura, não sobre uma ameaça detectada. A verificação de integridade disponível é o arquivo `.sha256` publicado ao lado de cada artefato:

```powershell
Get-FileHash .\fzcomputerai-setup-windows-x64.exe -Algorithm SHA256
Get-Content .\fzcomputerai-setup-windows-x64.exe.sha256
```

Confira que os valores batem antes de executar. O contexto completo (por que não há assinatura, o que seria necessário, e o que o auto-upgrade confere) está em [`SIGNING.md`](../SIGNING.md).

### Posso usar sem internet?

Sim, para o uso principal. O motor e o MCP em `127.0.0.1` funcionam offline: iniciar/parar, aplicar porta, testar endpoint, publicar na rede local (relay), calibração, janelas, gravação e `doctor` são todos locais.

Precisam de internet: verificar/aplicar atualizações (GUI e motor), qualquer túnel, baixar `cloudflared`/`ngrok`, o login e a criação de túnel nomeado no Cloudflare, e a sonda de exposição. A instalação do motor pelo instalador oficial do Cua também baixa arquivos da rede.

### O túnel é seguro?

Depende **exclusivamente** do que você põe na frente do MCP. O endpoint dá controle de mouse, teclado e tela desta máquina: quem alcança a URL e passa pela autenticação existente opera o computador.

| Configuração | Leitura honesta |
| --- | --- |
| túnel sem senha, sem token, sem Access | **não é seguro.** Qualquer pessoa com a URL controla a máquina. Uma URL aleatória não é autenticação: ela vaza em log, histórico, print e arquivo de config |
| senha na URL (porteiro local) | bloqueia varredura e acesso casual — sem `/s/<senha>/` o porteiro responde 404 e o MCP nem é tocado. Limites: a senha viaja no caminho da URL, é gerada por PRNG simples (não criptográfico) e não tem rotação nem expiração. Desde a 2.4.0 ela pesa mais: o porteiro injeta o Bearer do motor para quem passou por ela, então a URL completa é a credencial |
| autenticação de borda (Cloudflare Access, basic-auth do ngrok, proxy autenticado seu) | nível mais forte que este app apoia: a checagem acontece antes de a requisição chegar à sua máquina |
| motor `0.16+` com token | melhora real — sem `Authorization: Bearer <token>` o motor responde 401. Mas quem tiver **URL + token** controla a máquina; trate o token como senha. E motores **<= 0.8.x** **não têm token nenhum** — o instalador hoje exige 0.16.0 como mínimo |

Regra prática: suba o túnel, rode **Testar pela internet** e acredite no badge — não na intenção. Se o resultado for "NÃO FOI POSSÍVEL VERIFICAR", trate como exposto.

### O túnel subiu, mas o cliente recebe 401. E agora?

401 é o motor `0.16+` exigindo o token: sem `Authorization: Bearer <token>` correto ele recusa tudo — inclusive quando **nenhum** token foi configurado, porque o endpoint é *fail-closed* (o túnel sobe, mas nenhum cliente entra). Abra a aba Túnel e leia o aviso do topo: se ele pedir, clique em **Gerar e ativar token do motor** — a GUI gera um token criptográfico, grava em `HKCU\Environment` e reinicia o motor. Depois copie o snippet de novo: a partir daí ele já inclui o header `Authorization`. Se já havia token e o 401 continua, o motor está recusando o valor conhecido — gere um novo pelo mesmo botão.

### Posso usar o basic-auth do ngrok junto com o token do motor?

Não ao mesmo tempo. Os dois viajam no **mesmo** header `Authorization`, e o cliente MCP só envia um. Por isso, quando há token do motor ativo, a GUI **ignora** o basic-auth de borda ao iniciar o túnel (com nota na interface e no log): a proteção do túnel passa a ser o Bearer do motor. A senha na URL (`/s/<senha>/`) continua compatível com o token, porque vai no caminho, não no header.

### O badge diz "MOTOR EXIGIU TOKEN" — isso é erro?

Não — é o motor barrando quem chega **sem** Bearer, que é exatamente o que você quer numa URL pública. Para a prova completa, rode **Testar pela internet** com o token configurado: a sonda repete o teste **com** o Bearer e, se o `initialize` responder, o badge vira **PROTEGIDO E FUNCIONAL** — sem credencial, barrado; com credencial, funcionando.

### Preciso deixar o app aberto?

Sim, se quiser o MCP de pé. Desde a 2.3.0 o motor é **processo filho** da GUI: ela dá `spawn` em `cua-driver serve` e adota o filho num Job Object do Windows, então fechar a janela encerra o motor. Vale também para o relay da LAN e o porteiro de senha, que são threads do próprio processo, e para o processo do túnel, que também é filho adotado.

Você pode deixar o app aberto e fora do caminho: ele fica na bandeja e tem a opção **Iniciar com o Windows**. Se o que você quer é o motor rodando **sem** a GUI, esse é o papel da tarefa de autostart do **próprio** `cua-driver`, registrada pelo instalador oficial dele — e aí o ciclo de vida é dele, não deste app.

### Por que o MCP cai quando eu fecho o app?

É intencional, e desde a 2.3.0 quem garante isso é o **kernel**, não um vigia. O app cria um Job Object com `KILL_ON_JOB_CLOSE` antes de qualquer spawn e adota cada filho de longa duração; quando o processo da GUI termina — X, **Sair** na bandeja, `taskkill /F`, crash, logoff — o Windows mata os filhos junto. Isso existe porque no Windows um filho **não** morre com o pai por conta própria (`CreateProcess` não cria esse vínculo; isso é Unix).

O `on_exit` ainda encerra o relay e o porteiro de senha e dispara um auxiliar para limpar o que sobrevive a um processo: os registros `tunnel:*` e as regras de encaminhamento **legadas** que este app criou (em `HKCU\Software\FzComputerAI`). O objetivo é não deixar porta aberta nem URL pública viva com o software "fechado".

Uma coisa que **saiu** do fechamento: matar `cua-driver` por nome de imagem. Isso derrubava motor de qualquer origem, inclusive o daemon de outro cliente MCP. Se, ao abrir, já houver um motor de terceiro respondendo, a GUI o detecta, não duplica, não mata — e avisa na tela que aquele motor **não** será encerrado ao fechar.

### Como uso o meu próprio domínio (URL fixa)?

Pela aba Túnel, com Cloudflare, em cinco passos: **Login Cloudflare (OAuth)** -> **Verificar login** -> preencher **Nome do túnel** e **Hostname público** (ex.: `mcphome.seudominio.com.br`) -> **Criar túnel + apontar DNS** -> **Iniciar túnel**.

Duas coisas para não perder tempo: **o login sozinho não cria nada** — ele só baixa o `cert.pem`, por isso existem os dois passos extras; e **o domínio precisa estar na sua conta Cloudflare**, com os nameservers delegados a ela, senão o passo do DNS falha e o console mostra a recusa do `cloudflared`. Detalhes em [acesso-remoto.md](acesso-remoto.md).

### Meu cliente MCP só aceita uma URL, sem header. E agora?

Use o túnel **com senha** e entregue a URL completa, com `/s/<senha>/mcp`. Desde a 2.4.0 o porteiro de senha **injeta o Bearer** do motor: quem provou a senha no caminho já está autenticado perante o app, então o porteiro acrescenta o `Authorization` ao falar com o motor. Se o cliente mandar o próprio `Authorization`, o dele vence.

Testado: URL com senha e **sem nenhum header** — `initialize` OK e `tools/call get_screen_size` executou. Senha errada -> 404; sem senha -> 404.

O que muda no risco: o segredo do motor deixa de viajar pela internet, e **a senha da URL passa a ser a credencial pública**. Ela continua com os limites de sempre — vai no caminho da URL, aparece em log de intermediário, é gerada por PRNG simples e não tem rotação nem expiração. Para trocar, pare e reinicie o túnel com senha nova.

### O ngrok não sobe. Está quebrado?

O que costuma faltar é **credencial válida**, não código. Nesta máquina o authtoken em `%LOCALAPPDATA%\ngrok\ngrok.yml` é inválido e o agente morre com `ERR_NGROK_105` ao autenticar. E cuidado com o falso positivo: `ngrok config check` **passa**, porque ele valida a **sintaxe** do arquivo, não o token.

A correção é `ngrok config add-authtoken <SEU_TOKEN>` com o token real de dashboard.ngrok.com — a GUI detecta o `ERR_NGROK_105` no log e mostra esse comando. Um detalhe útil: a chave do ngrok **não** fica no registro do Windows (varredura de `HKCU` e `HKLM` não encontra nada); ela vive no `ngrok.yml`.

### Qual a diferença entre "a porta" e "publicar na rede"?

São coisas diferentes e ambas necessárias para uso na LAN:

| | Porta (**Aplicar Porta**) | Relay (**Publicar na rede**) |
| --- | --- | --- |
| O que faz | grava `CUA_DRIVER_RS_MCP_HTTP_PORT` e reinicia o motor | sobe um relay TCP dentro do próprio app: `0.0.0.0:porta -> 127.0.0.1:porta` |
| Sem isso… | o motor **não sobe listener HTTP nenhum** | o endpoint existe, mas **só** em `127.0.0.1` |
| Onde atua | dentro do motor | no processo da GUI, na frente do motor |
| Depende de | nada além do registro do usuário | nada além do app estar aberto; o firewall ainda precisa permitir a entrada |
| Pede UAC? | não | **não** (a regra `netsh portproxy`, que era o caminho antigo, pedia) |
| Sobrevive ao fechar o app? | a porta gravada sim; o motor não | não: o relay é uma thread do processo |

A porta decide **em qual porta** o MCP responde. O relay decide **quem consegue chegar** nela. Como o motor oficial escuta apenas em loopback (endereço fixo no código), mudar a porta nunca publica nada na rede — para isso é o relay (LAN) ou o túnel (internet).

Detalhe medido que simplifica tudo: nesta plataforma `0.0.0.0:<porta>` **coexiste** com o `127.0.0.1:<porta>` do motor, então a publicação usa a **mesma** porta, sem mexer na configuração do motor.

### A interface travava a cada clique. Isso foi resolvido?

Na 2.4.0, sim — para os piores casos, e a causa foi medida: toda ação rodava `Command::output()` **síncrono na thread da UI**. Um `reg` custa ~200 ms, um `powershell` de 300 ms a 2 s, e o teste de exposição do túnel dispara **dois** `curl -m 20` (até 40 s). Nesse tempo o Windows escreve "(Não Respondendo)" no título — não era travamento aleatório, era a janela esperando processo externo.

Agora existe um executor de segundo plano: a tarefa roda numa thread e o resultado é aplicado na thread da UI. Foram migrados o teste de exposição do túnel e a espera pelo motor depois do start (que eram até 12x400 ms de `sleep`). Enquanto há tarefa em voo, a interface pede repintura a cada 200 ms em vez de congelar.

Se ainda travar em alguma ação, **anote qual** e relate: significa que aquele caminho específico ainda não foi migrado.

### Por que não existe bind `0.0.0.0`?

Porque o motor oficial não tem essa opção: o endereço de escuta está fixo no código do Cua como `([127,0,0,1], port)`. A variável `CUA_DRIVER_RS_MCP_HTTP_BIND` **não existe** — a string não aparece no binário instalado (0.8.3) e a busca por ela no repositório `trycua/cua` retorna zero resultado. Uma versão anterior desta documentação afirmava o contrário; era falso e foi corrigido. A GUI hoje até apaga essa variável se encontrar sobra dela no ambiente do usuário, para não confundir o diagnóstico. Detalhes em [acesso-remoto.md](acesso-remoto.md).

### O que significa "REGRA SEM EFEITO"?

É o badge do caminho **legado** (`netsh portproxy`): a regra **existe** na configuração do `netsh`, mas o **listener não está de pé** no `netstat`. Quase sempre é o serviço IP Helper (`iphlpsvc`) parado ou travado.

Desde a 2.3.0 esse não é mais o caminho padrão — o relay do próprio app não depende do IP Helper e por isso não tem esse estado de limbo: ou o socket está escutando (**PUBLICADO NA REDE**) ou não está (**SÓ LOCAL**). Se você ainda tem uma regra antiga na máquina (elas sobrevivem a reboot), a GUI mostra o aviso e o botão **Remover regra antiga (UAC)**. Passo a passo em [uso-mcp-rede.md](uso-mcp-rede.md).

### Por que o badge fica amarelo mesmo com o motor funcionando?

Amarelo ("LOCAL apenas") é o estado **correto e esperado** do motor oficial: ele responde MCP em `127.0.0.1` e em nenhum outro endereço. Verde só aparece quando o listener na LAN é confirmado no `netstat` **e** o POST no IP da LAN responde. O badge não fica verde por otimismo.

### Por que preciso de POST para testar? `curl http://127.0.0.1:8000/mcp` não serve?

Não serve como prova. O endpoint MCP responde legitimamente `405 Method Not Allowed` a um GET, o que provaria apenas que existe um socket TCP aceitando conexão — não que há um servidor MCP do outro lado. Por isso o teste da GUI é um `POST /mcp` com um `initialize` JSON-RPC real, e só conta se a resposta contiver `"jsonrpc"`.

### Onde ficam as configurações? Não achei arquivo de config.

Não existe arquivo de configuração e o *storage* do eframe não é usado. O que persiste está no registro do Windows: `CUA_DRIVER_RS_MCP_HTTP_PORT` em `HKCU\Environment`, o autostart da GUI em `HKCU\...\CurrentVersion\Run`, e as regras/túneis/preferências em `HKCU\Software\FzComputerAI` (`portproxy:*` — só do caminho legado do `netsh`, o relay não persiste nada —, `tunnel:*`, `tunnelcfg:*`). Segredos **não** vão para o registro: o token do Cloudflare fica em arquivo com ACL restrita e só o caminho é guardado; a senha do porteiro do túnel existe apenas em memória, por sessão.

### O app tem telemetria?

Não. As únicas conexões de saída que a GUI faz por conta própria são: a API do GitHub Releases (ao clicar em Verificar Atualizações), o download do instalador e dos binários de túnel (a seu pedido), e a sonda de exposição na sua própria URL pública (ao clicar em Testar pela internet). Todo comando executado aparece no console — se algo estivesse falando com a rede, estaria lá.

### O app é traduzido? Como troco o idioma?

PT-BR e inglês, com troca em tempo real pelo botão no pé da barra lateral. Não há arquivo de tradução nem recarga: cada texto nasce de um `match state.language` no próprio código.

### Por que a interface não tem ícones nem emoji?

Porque a fonte usada não tem esses glifos e eles renderizariam caixas vazias, com cara de placeholder quebrado. O projeto usa `->` no lugar de `→` e um ponto **desenhado** para os indicadores de status. É regra, não descuido.

### Encontrei um bug. O que envio?

O conteúdo do console (botão **Copiar**), o badge exibido, a versão da GUI (barra lateral) e a saída de `cua-driver check-update --json`. Se for de rede, inclua `netstat -ano -p tcp` e `netsh interface portproxy show v4tov4` para a porta em questão. **Não** cole tokens nem senhas — confira o texto antes de publicar.

## Ver também

- [uso-mcp-rede.md](uso-mcp-rede.md) — operação e diagnóstico do endpoint.
- [uso-tunel.md](uso-tunel.md) — provedores, senha na URL e verificação de exposição.
- [acesso-remoto.md](acesso-remoto.md) — LAN x internet x VPN e as implicações de cada um.
- [solucao-de-problemas.md](solucao-de-problemas.md) — sintoma -> causa -> verificação -> correção.
- [atualizacao.md](atualizacao.md) — os dois componentes e o aviso do token.
