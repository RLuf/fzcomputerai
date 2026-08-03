# Perguntas frequentes

Para quem quer a resposta curta antes de ler o documento inteiro.

### Preciso do `cua-driver`?

Sim. Toda ação da interface termina em uma invocação de `cua-driver` como processo filho. Sem o motor instalado e no PATH, a janela abre, o console registra o erro de execução e nenhum botão produz efeito. O motor é instalado **à parte**, pelo instalador oficial do Cua — o instalador do FzComputerAI traz a task que dispara esse instalador ao final (marcada por padrão), mas em instalação silenciosa essa etapa é pulada de propósito.

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

Sim, para o uso principal. O motor e o MCP em `127.0.0.1` funcionam offline: iniciar/parar, aplicar porta, testar endpoint, encaminhamento LAN, calibração, janelas, gravação e `doctor` são todos locais.

Precisam de internet: verificar/aplicar atualizações (GUI e motor), qualquer túnel, baixar `cloudflared`/`ngrok`, `cloudflared tunnel login` e a sonda de exposição. A instalação do motor pelo instalador oficial do Cua também baixa arquivos da rede.

### O túnel é seguro?

Depende **exclusivamente** do que você põe na frente do MCP. O endpoint dá controle de mouse, teclado e tela desta máquina: quem alcança a URL e passa pela autenticação existente opera o computador.

| Configuração | Leitura honesta |
| --- | --- |
| túnel sem senha, sem token, sem Access | **não é seguro.** Qualquer pessoa com a URL controla a máquina. Uma URL aleatória não é autenticação: ela vaza em log, histórico, print e arquivo de config |
| senha na URL (porteiro local) | bloqueia varredura e acesso casual — sem `/s/<senha>/` o porteiro responde 404 e o MCP nem é tocado. Limites: a senha viaja no caminho da URL, é gerada por PRNG simples (não criptográfico) e não tem rotação nem expiração |
| autenticação de borda (Cloudflare Access, basic-auth do ngrok, proxy autenticado seu) | nível mais forte que este app apoia: a checagem acontece antes de a requisição chegar à sua máquina |
| motor `0.16+` com token | melhora real — sem `Authorization: Bearer <token>` o motor responde 401. Mas quem tiver **URL + token** controla a máquina; trate o token como senha. E um motor antigo (como o 0.8.3) **não tem token nenhum** — o instalador atual aplica a última versão estável publicada |

Regra prática: suba o túnel, rode **Testar pela internet** e acredite no badge — não na intenção. Se o resultado for "NÃO FOI POSSÍVEL VERIFICAR", trate como exposto.

### Por que o MCP cai quando eu fecho o app?

É intencional. Fechar a GUI significa desligar o conjunto: o `on_exit` encerra o motor, mata o túnel e o porteiro de senha, e remove as regras de encaminhamento **que este app criou** (as registradas em `HKCU\Software\FzComputerAI`). O objetivo é não deixar porta aberta nem URL pública viva com o software "fechado".

Se você quer o motor rodando com a GUI fechada, esse é o papel da tarefa de autostart do **próprio** `cua-driver`, registrada pelo instalador oficial dele.

### Qual a diferença entre "a porta" e "o encaminhamento"?

São coisas diferentes e ambas necessárias para uso na LAN:

| | Porta (`Aplicar Porta`) | Encaminhamento (`Aplicar Regra`) |
| --- | --- | --- |
| O que faz | grava `CUA_DRIVER_RS_MCP_HTTP_PORT` e reinicia o motor | cria a regra `netsh portproxy`: `<IP_LAN>:porta -> 127.0.0.1:porta` |
| Sem isso… | o motor **não sobe listener HTTP nenhum** | o endpoint existe, mas **só** em `127.0.0.1` |
| Onde atua | dentro do motor | no Windows, na frente do motor |
| Depende de | nada além do registro do usuário | serviço IP Helper (`iphlpsvc`), pode pedir UAC, e o firewall precisa permitir a entrada |

A porta decide **em qual porta** o MCP responde. O encaminhamento decide **quem consegue chegar** nela. Como o motor oficial escuta apenas em loopback (endereço fixo no código), mudar a porta nunca publica nada na rede — para isso é encaminhamento (LAN) ou túnel (internet).

### Por que não existe bind `0.0.0.0`?

Porque o motor oficial não tem essa opção: o endereço de escuta está fixo no código do Cua como `([127,0,0,1], port)`. A variável `CUA_DRIVER_RS_MCP_HTTP_BIND` **não existe** — a string não aparece no binário do motor (verificado na 0.8.3) e a busca por ela no repositório `trycua/cua` retorna zero resultado. Uma versão anterior desta documentação afirmava o contrário; era falso e foi corrigido. A GUI hoje até apaga essa variável se encontrar sobra dela no ambiente do usuário, para não confundir o diagnóstico. Detalhes em [acesso-remoto.md](acesso-remoto.md).

### O que significa "REGRA SEM EFEITO"?

Que a regra de encaminhamento **existe** na configuração do `netsh`, mas o **listener não está de pé** no `netstat`. Quase sempre é o serviço IP Helper (`iphlpsvc`) parado ou travado. A correção habitual é `Restart-Service iphlpsvc` em terminal elevado, ou remover e reaplicar a regra pela GUI. Passo a passo em [uso-mcp-rede.md](uso-mcp-rede.md).

### Por que o badge fica amarelo mesmo com o motor funcionando?

Amarelo ("LOCAL apenas") é o estado **correto e esperado** do motor oficial: ele responde MCP em `127.0.0.1` e em nenhum outro endereço. Verde só aparece quando o listener na LAN é confirmado no `netstat` **e** o POST no IP da LAN responde. O badge não fica verde por otimismo.

### Por que preciso de POST para testar? `curl http://127.0.0.1:8000/mcp` não serve?

Não serve como prova. O endpoint MCP responde legitimamente `405 Method Not Allowed` a um GET, o que provaria apenas que existe um socket TCP aceitando conexão — não que há um servidor MCP do outro lado. Por isso o teste da GUI é um `POST /mcp` com um `initialize` JSON-RPC real, e só conta se a resposta contiver `"jsonrpc"`.

### Onde ficam as configurações? Não achei arquivo de config.

Não existe arquivo de configuração e o *storage* do eframe não é usado. O que persiste está no registro do Windows: `CUA_DRIVER_RS_MCP_HTTP_PORT` em `HKCU\Environment`, o autostart da GUI em `HKCU\...\CurrentVersion\Run`, e as regras/túneis/preferências em `HKCU\Software\FzComputerAI` (`portproxy:*`, `tunnel:*`, `tunnelcfg:*`). Segredos **não** vão para o registro: o token do Cloudflare fica em arquivo com ACL restrita e só o caminho é guardado; a senha do porteiro do túnel existe apenas em memória, por sessão.

### O app tem telemetria?

Não. As únicas conexões de saída que a GUI faz por conta própria são: a API do GitHub Releases (ao clicar em Verificar e Atualizar), o download do instalador e dos binários de túnel (a seu pedido), e a sonda de exposição na sua própria URL pública (ao clicar em Testar pela internet). Todo comando executado aparece no console — se algo estivesse falando com a rede, estaria lá.

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
