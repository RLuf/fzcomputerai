# HANDOFF — FzComputerAI (estado em 26/07/2026)

Documento de passagem para sessão limpa. Contém o que está feito, o que falta,
e os fatos já medidos (não re-investigar).

---

## REGRAS DE TRABALHO (exigências do Roger — ler antes de tocar em qualquer coisa)

1. **Escopo estrito.** Corrigir SOMENTE o que foi apontado. Não reorganizar estrutura,
   não deletar arquivos, perder tempo com o que deixa de tornar funcional, seguranca nao eh o merito.
2. **`src/tabs/driver.rs` e `src/tabs/skills.rs` avaliar. a ideia eh fornecer uma skill no modelo claude padrao para ajudar o agente.
  3. **Proibido placeholder.** Nada de valor chumbado, mock, ou verificação que finge
   testar. Foi exatamente o defeito encontrado no teste de endpoint (ver abaixo).
4. **Usar APIs/ferramentas oficiais.** Se algo bloquear, parar e reportar — nunca improvisar.
5. **Testar antes de entregar.** Não entregar com base em leitura de código.
6. **Honestidade total.** Listar sempre o que ficou incompleto.

---

## ESTADO DO REPOSITÓRIO

- Branch: `feat/gui-debug-console-ci-installer`
- PR: https://github.com/RLuf/fzcomputerai/pull/1 (2 commits, aberto)
- Commits já enviados: `294ab7d`, `b95bdd7`

### Alterações no disco AINDA NÃO COMMITADAS

| Arquivo | O que é |
|---|---|
| `fzcomputerai/build.rs` (novo) | Embute VersionInfo no .exe — **funciona, verificado** |
| `fzcomputerai/Cargo.toml` | `build = "build.rs"` + `winresource` como build-dep só no Windows |
| `fzcomputerai/Cargo.lock` | consequência do acima |
| `installer/fzcomputerai.iss` | instalador Inno Setup 

Verificação do build.rs (executada, resultado real):
```
FileVersion     : 1.0.2.0
ProductVersion  : 1.0.2.0
CompanyName     : Webstorage Tecnologia
FileDescription : FzComputerAI - Computer Vision, MCP & CLI Hub
LegalCopyright  : Roger Luft / Webstorage Tecnologia - CC BY 4.0
```

---

## PENDÊNCIAS — a lista do Roger, nenhuma concluída

### 1. Teste do endpoint MCP mente sobre a LAN  ← **defeito principal**
O SOFTWARE MOSTRA UMA COISA E FAZ OUTRA.. POR PADRA O CUA ABRE LOCALHOST BASTA FAZER UM NETSH DIRECIONANDO A PORTA 
A interface exibe `Host/IP = 192.168.0.101`, pinta **LISTENING** em verde e publica
`http://192.168.0.101:8000/mcp` como "URL de Conexão MCP para Agentes Remotos" —
mas `check_port_status()` (`fzcomputerai/src/app.rs:189`) conecta em **127.0.0.1**.
Valida um endereço e pinta outro de verde. - 

**Evidência medida nesta máquina:**
```
127.0.0.1:8000      -> CONECTOU
192.168.0.101:8000  -> FALHOU (timed out)
netstat -nat         -> TCP 127.0.0.1:8000 LISTENING   (só loopback)
netsh ... show v4tov4 -> NÃO existe regra para 8000 (existe para 8082)
```

**O que fazer:** testar os DOIS endereços (loopback e o IP da LAN exibido), reportar
cada um no Console Debug, e o status distinguir: `LISTENING (local + LAN)` verde /
`LOCAL APENAS (não acessível pela LAN)` amarelo / `STOPPED` vermelho. O cabeçalho do
topo tem que seguir o mesmo critério.

**Fonte de verdade definida pelo Roger:** confirmar com
`netstat -nat | grep <porta> | grep <ip_lan>`. Só pintar verde se a linha
`<ip_lan>:<porta> ... LISTENING` existir no netstat.

### 2. O app é quem deve aplicar o netsh (não sugerir)

Instrução literal: *"não é portproxy, é o app quem deve fazer o netsh"*.
O botão "Regra Windows PortProxy (netsh)" existe (`apply_portproxy()`,
`app.rs:296`) mas nunca criou a regra da 8000. Precisa: aplicar com elevação,
reler `netsh interface portproxy show v4tov4`, confirmar com `netstat`, refazer o
teste TCP e logar cada etapa real.

**Pré-requisitos já verificados (ambos OK, não bloqueiam):**
- Firewall do perfil atual: **Desligado** (`BlockInbound` mas estado desligado)
- `iphlpsvc` (IP Helper, exigido pelo portproxy): **Running / Automatic**
- Regra análoga que já funciona nesta máquina: `192.168.0.101 8082 -> 127.0.0.1 8082`

### 3. Alternativa: fazer o driver ouvir em 0.0.0.0

`cua/libs/cua-driver/rust/crates/cua-driver/src/mcp_http.rs:43-45` — a função
`spawn()` monta o endereço a partir de `([127, 0, 0, 1], port)`. O bind em loopback
está **chumbado no código**, com o comentário "loopback only". A única variável lida é
`CUA_DRIVER_RS_MCP_HTTP_PORT` (`configured_port()`, linha 35) — **não existe** variável
de host/bind.

Fazer o bind em `0.0.0.0` exige patch nessa linha + recompilar e instalar o driver.
O driver instalado hoje (`0.8.3`, em `%LOCALAPPDATA%\Programs\Cua\cua-driver\bin`)
veio do release oficial do trycua, não do fork local.

### 4. Diagnóstico (aba Doctor & Skills) não funciona

`run_doctor()` (`app.rs:528`) chama `cua-driver doctor`.

**Fatos medidos (não re-investigar):**
- `cua-driver doctor` roda em **0,77s**, sai com **exit 0**
- A saída do diagnóstico vai para **stdout** (o banner de update vai para stderr)
- `cua-driver` **é encontrado** pela GUI — o Console Debug mostrou o resultado real de
  `cua-driver call get_screen_size` (JSON com height/scale_factor/width)
- Binário em `C:\Users\noob\AppData\Local\Programs\Cua\cua-driver\bin\cua-driver.exe`,
  presente no PATH do **usuário** (HKCU), ausente no PATH do sistema

**Causa ainda NÃO identificada.** Próximo passo: rodar a GUI e clicar no botão para
observar o comportamento real (não deduzir do código).

### 5. Rodapé e Donate

Pedido literal: *"no rodapé não coloca imóvel site somente no help, coloca donate com
meu celular logo da webstorage"*.

- Rodapé hoje (`app.rs`, `bottom_panel`): `Grupo FazAI | Webstorage Tecnologia | Imóvel Site`
- **Remover** "Imóvel Site" do rodapé (fica só na janela Ajuda & Sobre)
- **Adicionar** Donate com o celular
- **Logo da Webstorage no rodapé:** a GUI não renderiza imagem nenhuma hoje. Exigiria
  adicionar `egui_extras` + `image` ao `Cargo.toml` e redimensionar o PNG
  (`assets/img/webstorage-logo.png` tem 4863x4862, 305 KB). **Perguntar ao Roger antes.**

### 6. Logo do Imóvel Site no GitHub

`assets/img/imovelsite-logo.png` (260x120, 10 KB) e `assets/img/webstorage-logo.png`
(4863x4862, 305 KB). Suspeita: lado a lado com o mesmo `width=180` nos READMEs, um vira
quadradão e o outro fica espremido. Verificar o link raw real do GitHub com `curl`.

### 7. Outras pendências herdadas

- **Assinatura**: commits saem assinados com `id_ed25519_fzrepo`, mas aparecem
  "Unverified" — falta registrar a chave no GitHub como *Signing Key*:
  `gh auth refresh -h github.com -s admin:ssh_signing_key` e depois
  `gh ssh-key add C:/Users/noob/.ssh/id_ed25519_fzrepo.pub --type signing --title "noob-workstation-signing"`
- **WhatsApp** exibido em Ajuda & Sobre: `+55 51 99242539` — 8 dígitos após o DDD,
  celular brasileiro tem 9. Confirmar com o Roger, **não inventar dígito**.
- **Wizard interativo do instalador** nunca foi executado (só instalação silenciosa),
  então os dois caminhos de instalação do cua-driver não foram exercitados.
- **`cua/` é gitlink sem `.gitmodules`** — nunca materializa no CI; o instalador
  publicado sempre usará o fallback de rede do cua-driver.

---

## COMO TESTAR A GUI (obrigatório antes de entregar)

O MCP `fz-computer-vision` deste próprio projeto serve para testar:

```
start_session -> launch_app (path: G:\fzcomcontrol\fzcomputerai\target\release\fzcomputerai.exe)
-> get_window_state (devolve SCREENSHOT) -> click por COORDENADA DE PIXEL -> get_window_state
-> kill_app -> end_session
```

A árvore UIA do egui só expõe a barra de título — **clicar por pixel**, lendo as
coordenadas do screenshot.

Compilar: `cargo build --release --manifest-path G:\fzcomcontrol\fzcomputerai\Cargo.toml`

Instalador (Inno Setup 6.7.3 instalado por usuário, **não** em Program Files):
`"C:\Users\noob\AppData\Local\Programs\Inno Setup 6\ISCC.exe" /DAppVersion=1.0.2 installer\fzcomputerai.iss`

---

## ATUALIZAÇÃO 26/07/2026 (sessão /debug — testado na GUI real)

### Resolvido e VERIFICADO na GUI compilada

1. **Pendência 1 (endpoint mentia sobre a LAN) — CORRIGIDA.** `check_port_status`
   agora testa TCP em 127.0.0.1 E no IP da LAN, roda `netstat -ano -p tcp` como
   fonte de verdade e loga tudo no Console Debug. Status tri-estado:
   `LISTENING (local + LAN)` verde (só com linha `<ip_lan>:<porta> LISTENING` no
   netstat + TCP conectando) / `LOCAL APENAS` amarelo / `STOPPED` vermelho.
   Cabeçalho do topo segue o mesmo critério; aviso amarelo sob a URL quando ela
   não está acessível pela LAN. Verificado nos dois estados (antes/depois da regra).
2. **Pendência 2 (app aplica o netsh) — reformulada e regra ATIVA.** `apply_portproxy`:
   checa regra existente (parse por token), add direto, senão UAC com
   `-PassThru`/exit code propagado, relê `show v4tov4`, confirma netstat, refaz TCP.
   Regra `192.168.0.101:8000 -> 127.0.0.1:8000` existe e funciona:
   `POST http://192.168.0.101:8000/mcp` (initialize MCP) => **HTTP 200**.
3. **Pendência 4 (Doctor) — FUNCIONA.** Botão clicado na GUI real, saída completa
   renderizada. A observação antiga era quase certamente clique de teste que não
   chegava na janela (egui descarta clique em segundo plano — ver nota abaixo).
4. **Pendência 5 (rodapé) — feita a parte sem dependências.** Imóvel Site removido
   do rodapé (fica só em Ajuda & Sobre); Donate `+55 51 99242539` adicionado.
   Logo Webstorage NÃO adicionada (exige egui_extras+image — perguntar ao Roger).

### Ainda pendente

- Confirmar com Roger o dígito do celular (`+55 51 99242539` tem 8 dígitos após DDD).
- Logo Webstorage no rodapé (aguardando OK para novas dependências).
- Pendências herdadas do item 7 (signing key, wizard do instalador, gitlink cua/).
- Item 6 (logos no GitHub) não abordado nesta sessão.
- Cosmético: linhas longas sem espaço no painel Doctor renderizam com espaçamento
  estranho (comportamento de quebra do egui) — só estética.
- `src/tabs/driver.rs` e `src/tabs/skills.rs` existem mas NÃO estão em `tabs/mod.rs`
  (código morto, nunca compilado). Avaliar com Roger a ideia da skill padrão Claude.

### Nota para testar a GUI com fz-computer-vision

A janela egui **descarta cliques de pixel em segundo plano**. Sequência que funciona:
`bring_to_front` (repetir até `landed_on_target:true`) e clicar com
`delivery_mode:"foreground"`. Coordenadas são pixels locais do screenshot da janela.
