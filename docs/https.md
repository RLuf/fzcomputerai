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
2. **Porta HTTPS** (padrão `8443` — não pode ser a mesma do HTTP do motor).
3. **Escutar em**: `127.0.0.1` (só esta máquina), `<IP da LAN>` (padrão) ou `0.0.0.0` (todas as interfaces).
4. Escolha a origem do certificado (abaixo) e clique **Aplicar / Reiniciar HTTPS**.

O badge à direita só fica **verde** depois de uma sonda real: handshake TLS + `POST initialize` JSON-RPC
atrás do listener. "TLS OK, motor não responde" (amarelo) significa que o TLS está de pé mas o motor não
respondeu — motor parado, ou token ausente. **Testar Endpoint** (no painel do motor) e **Testar HTTPS** refazem a sonda;
a verificação de startup também a inclui.

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
     https://192.168.0.101:8443/mcp

# (b) por pin do fingerprint SHA-256 (o botão Copiar da tela entrega no formato AA:BB:...)
curl --pinnedpubkey "sha256//<base64>" ...   # ou o mecanismo de pin do seu cliente
```

Clientes MCP que não têm opção de CA própria (por exemplo alguns conectores hospedados) **não vão aceitar** um
auto-assinado — use Let's Encrypt.

### 2. Let's Encrypt (ACME, desafio HTTP-01)

Emite um certificado **confiado por qualquer cliente**, via [`instant-acme`](https://github.com/djc/instant-acme)
(RFC 8555). Pré-requisitos que **não dá para automatizar** deste lado:

1. um **nome DNS público** apontando para o **IP público** desta máquina (Let's Encrypt não emite para IP);
2. a **porta 80** chegando até aqui, vinda da internet (encaminhamento no roteador + regra no firewall). A CA
   busca `http://<dominio>/.well-known/acme-challenge/<token>` — a porta é fixa pelo protocolo.

Passos: selecione **Let's Encrypt**, preencha **Domínio público** e **E-mail** (avisos de expiração), clique
**Emitir Let's Encrypt** e acompanhe o console (`[acme]`). Durante a emissão o app abre um respondedor
temporário em `0.0.0.0:80`; ele cai assim que a CA valida. O par vai para `letsencrypt.crt/.key`, a conta
ACME para `letsencrypt-account.json`, o modo muda sozinho para Let's Encrypt e o listener é recarregado.

- **Renovação automática**: a cada 6 h o app confere a validade; com menos de 30 dias, emite de novo (mesmos
  pré-requisitos — a porta 80 precisa continuar alcançável).
- **Staging**: marque para testar contra o ambiente de teste do Let's Encrypt (cert **não** confiável, mas sem
  gastar o limite de rate da produção — 5 falhas/hora por conta+domínio, 50 certs/semana por domínio).
- Usar a **porta 443** para o HTTPS (em vez de 8443) é opcional; no Windows não exige admin.

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

## Configuração nos clientes

```json
{
  "mcpServers": {
    "fzcomputerai": {
      "type": "http",
      "url": "https://192.168.0.101:8443/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

Com auto-assinado, o cliente precisa aceitar o `.crt` (Node: `NODE_EXTRA_CA_CERTS=…\selfsigned.crt`; Python
`requests`: `verify="…\\selfsigned.crt"`). Com Let's Encrypt, nada a configurar além da URL com o domínio.

## Limites e decisões

- **HTTP/1.1 apenas** (ALPN `http/1.1`). O motor é HTTP/1.1; sem ganho em anunciar h2.
- **Sem TLS-ALPN-01 nem DNS-01.** HTTP-01 na porta 80 é o único desafio implementado — é o mais simples de
  satisfazer num Windows doméstico e não trava a UI esperando o usuário editar DNS.
- **Chave privada** só no disco (`0600` no unix), nunca no registro, log, console ou linha de comando.
- **A sonda interna aceita qualquer certificado** (`CaptureVerifier`) — ela existe só para o app conferir a si
  mesmo e capturar o cert servido. Não é um cliente de uso geral.
- **Backend criptográfico `ring`** em todas as crates; `aws-lc-rs` exigiria cmake+nasm no runner Windows do
  release.

## Solução de problemas

| Sintoma | Causa provável | O que fazer |
| --- | --- | --- |
| `FALHA ao subir o listener HTTPS: escutar em …: … (10048/EADDRINUSE)` | porta ocupada | troque a porta ou feche o outro programa (`netstat -ano \| findstr :8443`) |
| badge amarelo "TLS OK, motor não responde" | motor parado ou token ausente | **Iniciar** o motor; conferir `CUA_DRIVER_RS_MCP_HTTP_TOKEN` |
| cliente recusa: *self-signed certificate* / *unable to verify* | esperado com auto-assinado | passar o `.crt` ao cliente ou usar Let's Encrypt |
| cliente recusa: *hostname mismatch* | você acessou por um nome/IP que não está nos SANs | acesse por um SAN listado, ou preencha *Domínio* e **Regenerar auto-assinado** |
| `[acme] FALHOU … escutar em 0.0.0.0:80` | outra coisa na porta 80 (IIS, Apache, Skype antigo) | libere a 80 durante a emissão |
| `[acme] validação falhou — status Invalid` | DNS não aponta para cá ou porta 80 não chega | testar de fora: `curl http://<dominio>/.well-known/acme-challenge/x` deve dar **404 do app**, não timeout |
| `[acme] … rateLimited` | limite do Let's Encrypt | esperar; testar com **Staging** |
| cert expirou e não renovou | app fechado no período, ou porta 80 fechada | abrir o app (renova no startup) / reabrir a 80 e **Emitir** |

## Segurança — o que este recurso NÃO faz

Não instala CA nem certificado em store de confiança; não assina binários; não mexe em SmartScreen/Defender;
não afirma remover aviso de navegador. Tudo isso continua proibido por `AGENTS.md` §4.1. Servir TLS com um
certificado de servidor é uma coisa; alterar em quem a máquina do usuário confia é outra — e esta GUI só faz
a primeira.
