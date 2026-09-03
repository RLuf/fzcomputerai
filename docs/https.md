# HTTPS no endpoint MCP

> **Versão:** recurso introduzido na **v2.2.0**. Código: `fzcomputerai/src/tls.rs` (mecânica) e o bloco
> `impl AppState` "HTTPS do endpoint MCP" em `fzcomputerai/src/app.rs` (orquestração); tela em
> `fzcomputerai/src/tabs/network.rs` (`render_https`).

## O problema que isto resolve

O motor `cua-driver serve` escuta **somente** em `127.0.0.1:<porta>` e **somente HTTP** — endereço e
transporte são fixos no código do projeto Cua, e esta GUI não altera o motor. Clientes MCP que exigem
`https://` (conectores hospedados, políticas corporativas, navegadores com regra de *mixed content*) não
conseguem falar com ele.

A aba **Túnel** já resolvia isso para a internet (Cloudflare/ngrok entregam HTTPS público). O que faltava era
HTTPS **na LAN e na própria máquina**, sem depender de serviço externo. É o que este recurso faz.

## Como funciona

O app sobe um **listener TLS próprio** — uma thread do processo — em `<bind>:<porta_https>` (padrão
`8443`), termina o TLS com [`rustls`](https://github.com/rustls/rustls) e copia os bytes, nos dois
sentidos, contra `127.0.0.1:<porta_http>` (onde o motor escuta). É **exatamente o desenho do Encaminhamento
LAN** da v2.1.1:

- sem admin/UAC, sem `netsh`, sem regra no sistema;
- só escuta enquanto o app está vivo — ao fechar, a porta fecha junto;
- depois do handshake é TCP puro: o **bearer token do motor continua obrigatório** (`Authorization: Bearer`).
  O HTTPS protege o transporte; ele **não** substitui a autenticação.

```
cliente ──HTTPS──▶ <ip>:8443 (thread do app, rustls) ──HTTP──▶ 127.0.0.1:8000 (cua-driver serve)
```

## Ligar

Aba **MCP & Rede** → painel **HTTPS do endpoint MCP** (na área rolável, acima do diagnóstico):

1. Marque **Ligar HTTPS**. A preferência é persistida (`tlscfg:*`) e o listener sobe junto com o app nas
   próximas aberturas.
2. **Porta HTTPS** (padrão `8443` — não pode ser a mesma do HTTP do motor). Se a porta estiver ocupada ou reservada pelo sistema (caso medido: `8443` presa pelo PID 4 do Windows, `WSAEACCES`), o app sobe na **próxima livre** entre as 20 seguintes, atualiza a preferência e explica no console.
3. **Escutar em**: `127.0.0.1` (só esta máquina), `<IP da LAN>` (padrão) ou `0.0.0.0` (todas as interfaces).
4. Escolha a origem do certificado (abaixo) e clique **Aplicar / Reiniciar HTTPS**.

O badge à direita só fica **verde** depois de uma sonda real: handshake TLS + `POST initialize` JSON-RPC
atrás do listener. "TLS OK, motor não responde" (amarelo) significa que o TLS está de pé mas o motor não
respondeu — motor parado, ou token ausente. **Testar Endpoint** (no painel do motor) e **Testar HTTPS** refazem a sonda;
a verificação de startup também a inclui, e desde a v2.3.2 um vigia em segundo plano reavalia sozinho (a cada 5 s) quando o motor passa a responder ou para — o badge não depende mais de clique.

## Certificados

Todos os arquivos ficam em **`%APPDATA%\FzComputerAI\tls\`** (modo portátil: `tls\` ao lado do exe; Linux:
`~/.config/fzcomputerai/tls`). O botão **Abrir pasta dos certificados** leva até lá.

### 1. Auto-assinado (padrão, zero configuração)

Gerado pelo próprio app com `rcgen` (ECDSA P-256, validade 825 dias) **na instalação ou no primeiro run —
o que vier primeiro**:

- o instalador executa `fzcomputerai --tls-init` ao fim do setup (também no upgrade silencioso) e grava o
  resultado em `tls\tls-init.log`;
- a GUI repete a mesma chamada no startup; se o cert já existe, é válido (> 30 dias) e cobre os SANs, **é
  mantido** — regenerar trocaria o fingerprint e quebraria o pin de todo cliente.

SANs incluídos: `localhost`, `127.0.0.1`, o IP da LAN, o nome da máquina e o domínio do campo *Domínio*
(quando preenchido). Se o IP da LAN **mudar** (DHCP), o startup detecta que o cert não cobre o IP novo e
**regenera** — o fingerprint muda e o console avisa; clientes com pin precisam atualizar. Para evitar isso,
fixe o IP da máquina no roteador (reserva DHCP) ou use um nome (Let's Encrypt). Renovação automática quando faltam menos de 30 dias. **Regenerar auto-assinado** força
um par novo (o anterior fica em `selfsigned.prev.*`).

**Como o cliente confia nele.** O certificado **não é instalado** em nenhuma store de confiança do Windows
(isso é proibido no projeto — `AGENTS.md` §4.1 — porque altera a postura de segurança da máquina). A
confiança é decidida **do lado do cliente**, e a tela dá as duas formas:

```bash
# (a) pelo arquivo .crt
curl --cacert "%APPDATA%\FzComputerAI\tls\selfsigned.crt" \
     -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"x","version":"1"}}}' \
     https://192.168.0.10:8443/mcp

# (b) por pin do fingerprint SHA-256 (o botão Copiar da tela entrega no formato AA:BB:...)
curl --pinnedpubkey "sha256//<base64>" ...   # ou o mecanismo de pin do seu cliente
```

Clientes MCP que não têm opção de CA própria (por exemplo alguns conectores hospedados) **não vão aceitar** um
auto-assinado — use Let's Encrypt.

### 2. Let's Encrypt (ACME)

Emite um certificado **confiado por qualquer cliente** via [`instant-acme`](https://github.com/djc/instant-acme)
(RFC 8555). Dois desafios:

#### 2a. DNS-01 via API do Cloudflare — padrão, e o caminho para **rede interna**

Caso real: a máquina está em `192.168.0.10`, o MCP é usado só na LAN, mas o cliente exige `https://` com
certificado válido (senão pede confirmação de segurança). Let's Encrypt **não** emite para IP nem para nome de
máquina sem DNS público — então usa-se um nome numa zona sua no Cloudflare, por exemplo
`mcp.exemplo.com.br`, apontando para o **IP privado**. A CA só consulta o DNS; nenhuma porta precisa
estar aberta na internet.

1. No Cloudflare, crie um **API token** com `Zone.DNS:Edit` e `Zone:Read` na zona (`exemplo.com.br`). (template "Edit zone DNS" já traz as duas). O botão **Verificar token** lista as zonas que ele enxerga — o endpoint oficial `/user/tokens/verify` **não** serve para token restrito a zona (responde "Invalid API Token" mesmo com o token bom; medido).
2. No painel HTTPS: **Let's Encrypt** → desafio **DNS-01 via Cloudflare**. O campo Domínio aceita **vários nomes separados por vírgula** (ex.: `mcp.exemplo.com.br, home.exemplo.com.br` — um interno, um público); todos entram no mesmo certificado. → cole o token → **Verificar token**
   (mostra a zona encontrada) → preencha **Domínio público** e **E-mail** → deixe marcado **Criar/atualizar
   registro A -> IP da LAN** → **Aplicar** (grava o token em arquivo) → **Emitir Let's Encrypt**.
3. Acompanhe o console (`[acme]`): registro A `mcp.exemplo… -> 192.168.0.10`, TXT `_acme-challenge` criado,
   propagação confirmada no DoH do 1.1.1.1, validação, certificado emitido, TXT removido.
4. O modo troca sozinho para Let's Encrypt e o listener recarrega. A URL passa a ser
   `https://mcp.exemplo.com.br:<porta>/mcp` — resolve para o IP da LAN de qualquer máquina da rede.

O token fica em `cloudflare-api-token.txt` (0600) na pasta dos certificados — nunca no registro, log ou console.
Renovação automática com menos de 30 dias, pelo mesmo caminho (o token precisa continuar válido).

#### 2b. HTTP-01 (porta 80 pública)

Para máquina exposta diretamente na internet. Pré-requisitos que **não dá para automatizar** deste lado:

1. um **nome DNS público** apontando para o **IP público** desta máquina;
2. a **porta 80** chegando até aqui, vinda da internet (encaminhamento no roteador + regra no firewall). A CA
   busca `http://<dominio>/.well-known/acme-challenge/<token>` — a porta é fixa pelo protocolo.

Passos: selecione **HTTP-01**, preencha domínio e e-mail, **Emitir**. Durante a emissão o app abre um
respondedor temporário em `0.0.0.0:80`; ele cai assim que a CA valida.

Comum aos dois: o par vai para `letsencrypt.crt/.key`, a conta ACME para `letsencrypt-account.json`.
**Staging** testa contra o ambiente de teste do Let's Encrypt (cert **não** confiável, sem gastar o limite de
rate da produção — 5 falhas/hora por conta+domínio, 50 certs/semana por domínio).

### 3. Certificado próprio

Informe os caminhos do `.crt` (cadeia PEM, folha primeiro) e da `.key` (PKCS#8/SEC1/PKCS#1 PEM). Útil com uma
CA interna da empresa — a confiança, nesse caso, já está distribuída pelos administradores.

## O que a tela mostra (tudo lido de verdade)

| Campo | Origem |
| --- | --- |
| Emissor, SANs, validade, dias restantes | `x509-parser` sobre o arquivo em uso — ou sobre o cert **realmente servido** no último handshake da sonda |
| SHA-256 | digest do DER do certificado (formato `AA:BB:…`), copiável |
| URL HTTPS (estado real) | só mostra o IP da LAN / domínio quando a sonda naquele endereço passou |
| protocolo, HTTP, JSON-RPC, conexões aceitas | resultado da sonda e contador do listener |
| Linha `HTTPS / TLS -> HTTP` na grade de diagnóstico | mesmo status do badge |

## OAuth 2.1 para conectores (v2.3.0)

Conectores hospedados (Claude.ai, Gemini) e clientes MCP que seguem a
[especificação de autorização do MCP](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization)
não aceitam bearer estático: eles precisam de um servidor OAuth. O app serve um, **dentro do listener HTTPS**,
na mesma origem do `/mcp`:

| Caminho | Papel |
| --- | --- |
| `/.well-known/oauth-protected-resource` | RFC 9728 — diz qual servidor de autorização protege o `/mcp` |
| `/.well-known/oauth-authorization-server` | RFC 8414 — endpoints, PKCE S256, grants suportados |
| `/register` | RFC 7591 — o conector registra-se sozinho (sem segredo) |
| `/authorize` | página que pede a **senha de autorização** do app; emite o `code` |
| `/token` | `authorization_code` (PKCE obrigatório) e `refresh_token` (com rotação) |

Fluxo real, visto do conector: `POST /mcp` sem credencial → **401** com
`WWW-Authenticate: Bearer resource_metadata="…/.well-known/oauth-protected-resource"` → lê os dois metadata →
`POST /register` → abre `/authorize` no navegador do usuário → você digita a senha de autorização → volta com
`code` → `POST /token` → recebe `access_token` + `refresh_token` → `POST /mcp` com `Authorization: Bearer
<access_token>`. O app **troca** esse header pelo bearer do motor ao encaminhar: o conector nunca vê o token do
motor, o motor nunca vê o token OAuth. Acesso expira em 24 h; o refresh renova (30 dias).

Ligar: painel HTTPS → **OAuth 2.1 para conectores** → **Gerar senha de autorização** (mostrada uma vez; só o
SHA-256 é gravado). **Revogar todos os conectores** invalida clientes e tokens (a senha fica). Estado em
`oauth-state.json` (0600) na pasta dos certificados.

Pré-requisitos: HTTPS com certificado que o conector **confie** (Let's Encrypt — auto-assinado não serve para
Claude.ai/Gemini) e, para conectores na nuvem, a URL precisa ser alcançável da internet (túnel ou
port-forward); para clientes na LAN, o nome com Let's Encrypt via DNS-01 já basta.

Compatibilidade: requisição com `Authorization: Bearer <token do motor>` continua passando intacta.

Clientes nativos (Claude Code) usam **CIMD** — o `client_id` é a URL do documento de metadata deles — e redirect de **loopback com porta efêmera**; os dois são aceitos (v2.3.1). As preferências HTTPS ficam em `tls-config.json` na pasta dos certificados (o registro é só migração).

### Cliente em outra máquina (Claude Code remoto, SSH, servidor sem navegador)

O redirect do Claude Code é **loopback da máquina onde ele roda** (`http://localhost:<porta efêmera>/callback`).
Se o Claude Code está num servidor (sessão SSH, VPS, container) e o navegador está no seu desktop, a página de
`/authorize` autoriza normalmente, mas o redirect cai no `localhost` **do desktop**, onde não há nada escutando:
o navegador mostra erro de conexão e o Claude Code fica esperando. Não é falha do app nem do OAuth — é a
topologia.

Procedimento (medido em 2026-09-03: Claude Code num servidor remoto → app na LAN de casa, via NAT na 8444):

1. No Claude Code, deixe o fluxo gerar a URL de `/authorize` (a ferramenta `authenticate` do servidor MCP
   imprime a URL quando não consegue abrir navegador).
2. Abra essa URL no navegador do desktop, digite a senha de autorização e clique **Autorizar**.
3. O navegador vai para `http://localhost:<porta>/callback?code=…&state=…` e mostra erro de conexão.
   **Copie a URL inteira da barra de endereços.**
4. Cole no Claude Code: ele chama `complete_authentication` com essa URL, troca o `code` no `/token` e as
   ferramentas aparecem.

O `code` expira rápido e o `state` pertence àquela sessão do Claude Code — faça os quatro passos de uma vez.
Com `offline_access` o cliente guarda o refresh token; só é preciso repetir se voltar **401**.

Sintoma de quem trava aqui: `/mcp` responde 401 (esperado), os dois metadata e o `/register` respondem, a tela
de senha aparece, e mesmo assim o cliente "nunca conecta". Se for isso, é o callback.

## Configuração nos clientes

```json
{
  "mcpServers": {
    "fzcomputerai": {
      "type": "http",
      "url": "https://192.168.0.10:8443/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

Com **OAuth 2.1 ligado**, omita `headers`: o cliente recebe o 401, descobre o OAuth pelo `WWW-Authenticate`
e faz o fluxo sozinho. No Claude Code:

```bash
claude mcp add --transport http fzcomputerai https://mcp.exemplo.com.br:8443/mcp
```

Com auto-assinado, o cliente precisa aceitar o `.crt` (Node: `NODE_EXTRA_CA_CERTS=…\selfsigned.crt`; Python
`requests`: `verify="…\\selfsigned.crt"`). Com Let's Encrypt, nada a configurar além da URL com o domínio.

## Limites e decisões

- **HTTP/1.1 apenas** (ALPN `http/1.1`). O motor é HTTP/1.1; sem ganho em anunciar h2.
- **Sem TLS-ALPN-01.** DNS-01 é só via API do Cloudflare (automático, sem travar a UI esperando o usuário editar
  DNS à mão); outros provedores de DNS ficam para quando houver caso real.
- **Chave privada** só no disco (`0600` no unix), nunca no registro, log, console ou linha de comando.
- **A sonda interna aceita qualquer certificado** (`CaptureVerifier`) — ela existe só para o app conferir a si
  mesmo e capturar o cert servido. Não é um cliente de uso geral.
- **Backend criptográfico `ring`** em todas as crates; `aws-lc-rs` exigiria cmake+nasm no runner Windows do
  release.


### Identidade pelo Cloudflare Access (OIDC)

Para o login do OAuth ser feito pelo Cloudflare Access (GitHub, Google, One-time PIN…) em vez de só a senha do app:

1. No Cloudflare Zero Trust, crie uma aplicação **Access for SaaS – OIDC** com redirect URI `https://<host>:<porta>/oauth/cf/callback`, PKCE ligado, scopes `openid email profile`, e as políticas de acesso desejadas.
2. Crie `cloudflare-oidc.json` na pasta dos certificados (`%APPDATA%\FzComputerAI\tls`):
   ```json
   {
     "client_id": "<Client ID da aplicação>",
     "client_secret": "<Client Secret, ou null para cliente público com PKCE>",
     "discovery_url": "https://<team>.cloudflareaccess.com/cdn-cgi/access/sso/oidc/<Client ID>/.well-known/openid-configuration",
     "callback_url": "https://<host>:<porta>/oauth/cf/callback"
   }
   ```
3. Reinicie o app (ou desligue/ligue o OAuth). O painel mostra `Cloudflare Access (OIDC): ATIVO`.

Fluxo: cliente MCP → `/authorize` do app → login no Cloudflare → `/oauth/cf/callback` → senha do app (se definida) → código → `/token` do app. O `id_token` é obtido direto do token endpoint do Cloudflare por TLS verificado; o `email` aparece na página de autorização. Sem o arquivo, o `/authorize` volta a ser a página de senha.

## Solução de problemas

| Sintoma | Causa provável | O que fazer |
| --- | --- | --- |
| `FALHA ao subir o listener HTTPS: escutar em …: … (10048/EADDRINUSE)` | porta ocupada | troque a porta ou feche o outro programa (`netstat -ano \| findstr :8443`) |
| badge amarelo "TLS OK, motor não responde" | motor parado ou token ausente | **Iniciar** o motor; conferir `CUA_DRIVER_RS_MCP_HTTP_TOKEN` |
| cliente recusa: *self-signed certificate* / *unable to verify* | esperado com auto-assinado | passar o `.crt` ao cliente ou usar Let's Encrypt |
| cliente recusa: *hostname mismatch* | você acessou por um nome/IP que não está nos SANs | acesse por um SAN listado, ou preencha *Domínio* e **Regenerar auto-assinado** |
| `FALHA … nenhuma porta livre entre 8443 e 8463` | faixa inteira reservada/ocupada | escolha outra porta (ex.: 9443) |
| `[acme] … nenhuma zona do Cloudflare acessível pelo token` | token sem `Zone:Read`/`Zone.DNS:Edit` na zona, ou domínio fora das suas zonas | recrie o token com as duas permissões na zona certa; **Verificar token** mostra o que ele enxerga |
| `[acme] AVISO: TXT ainda não visível…` seguido de `Invalid` | propagação lenta | clique **Emitir** de novo após 1–2 min |
| `[acme] FALHOU … escutar em 0.0.0.0:80` | outra coisa na porta 80 (IIS, Apache, Skype antigo) | libere a 80 durante a emissão |
| `[acme] validação falhou — status Invalid` | DNS não aponta para cá ou porta 80 não chega | testar de fora: `curl http://<dominio>/.well-known/acme-challenge/x` deve dar **404 do app**, não timeout |
| `[acme] … rateLimited` | limite do Let's Encrypt | esperar; testar com **Staging** |
| cert expirou e não renovou | app fechado no período, ou porta 80 fechada | abrir o app (renova no startup) / reabrir a 80 e **Emitir** |

## Segurança — o que este recurso NÃO faz

Não instala CA nem certificado em store de confiança; não assina binários; não mexe em SmartScreen/Defender;
não afirma remover aviso de navegador. Tudo isso continua proibido por `AGENTS.md` §4.1. Servir TLS com um
certificado de servidor é uma coisa; alterar em quem a máquina do usuário confia é outra — e esta GUI só faz
a primeira.
