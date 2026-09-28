# Proxy MCP OAuth 2.1 & Conectores Externos (Python)

Guia de uso e referência técnica do micro-proxy Python auxiliar (`mcp-oauth-proxy`) incluído no instalador do FzComputerAI.

---

## 🎯 Visão Geral

O FzComputerAI oferece duas formas complementares para conectar clientes remotos ao Model Context Protocol (MCP):

1. **HTTPS & OAuth 2.1 Nativo (Rust, porta 8443):**
   Implementado diretamente em `src/tls.rs` e `src/oauth.rs`, oferecendo terminação TLS com certificados auto-assinados ou Let's Encrypt (DNS-01 / HTTP-01) e autenticação RFC 6749 / RFC 7636 (PKCE).
2. **Micro-Proxy Python Auxiliar (`mcp-oauth-proxy`, porta 8080):**
   Um micro-servidor HTTP/HTTPS leve e customizável, desenvolvido em Python, incluído na pasta `mcp-oauth-proxy/` da instalação. Ele atua como intermediário transparente quando clientes, orquestradores (n8n, Make, Dify, Flowise) ou conectores MCP não suportam os fluxos estritos de TLS ou exigem simplificação na injeção do Bearer Token.

---

## 📐 Arquitetura do Micro-Proxy

```text
┌─────────────────────────────────┐
│ Cliente Remoto / Orquestrador   │
│ (n8n, Make, Flowise, Cursor...) │
└──────────────┬──────────────────┘
               │  HTTP / HTTPS (ex: :8080)
               ▼
┌─────────────────────────────────┐
│       mcp-oauth-proxy           │  <- Python (server.py)
│ - Valida credenciais ou chaves  │     Incluso no instalador
│ - Injeta CUA Bearer Token       │
└──────────────┬──────────────────┘
               │  HTTP Loopback (127.0.0.1:8000)
               ▼
┌─────────────────────────────────┐
│      Motor cua-driver serve     │  <- CUA Driver MCP Engine
└─────────────────────────────────┘
```

O proxy resolve automaticamente o token gravado pela GUI em `HKCU\Environment\CUA_DRIVER_RS_MCP_HTTP_TOKEN`, garantindo que o cliente externo consiga interagir com as ferramentas MCP sem precisar lidar com os detalhes do ambiente Windows.

---

## 🚀 Como Iniciar

### 1. Via Instalador Windows

Durante a execução do instalador `fzcomputerai-setup-windows-x64.exe`, na tela de **Opções Adicionais**, marque a opção:

- `[x] Instalar dependencias Python do proxy OAuth 2.1 (requests, cryptography)`

O instalador rodará automaticamente `pip install -r requirements.txt` no ambiente Python padrão da máquina.

### 2. Manualmente via `start-proxy.bat`

No diretório de instalação (ou a partir da raiz do repositório):

```cmd
cd mcp-oauth-proxy
start-proxy.bat
```

O script verifica se o Python está presente, valida as dependências e inicia o serviço escutando por padrão em `http://127.0.0.1:8080` ou em rede conforme configurado.

### 3. Via Linha de Comando (PowerShell / Bash)

```powershell
cd mcp-oauth-proxy
python -m pip install -r requirements.txt
python server.py
```

---

## ⚙️ Variáveis de Ambiente & Configuração

O micro-proxy pode ser configurado por variáveis de ambiente ou pelo arquivo `.env`:

| Variável | Padrão | Descrição |
| :--- | :--- | :--- |
| `MCP_UPSTREAM_PORT` | `8000` | Porta onde o `cua-driver serve` está rodando em `127.0.0.1`. |
| `PROXY_PORT` | `8080` | Porta em que o micro-proxy receberá requisições dos clientes. |
| `PROXY_HOST` | `127.0.0.1` | Interface de escuta (`0.0.0.0` para expor na LAN ou atrás de túnel). |
| `MCP_TOKEN` | *(lido do registro)* | Token Bearer do motor. Se não fornecido, busca no ambiente do Windows. |

---

## 🔗 Integração com Clientes Populares

### 1. n8n / Make (HTTP Request Node)

- **Método:** `POST`
- **URL:** `http://<ip-do-pc>:8080/mcp`
- **Headers:** `Content-Type: application/json`
- **Body (JSON):**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

### 2. Cursor / Windsurf / Claude Code

Configure no arquivo de configuração MCP correspondente:

```json
{
  "mcpServers": {
    "fzcomputerai-proxy": {
      "url": "http://127.0.0.1:8080/mcp"
    }
  }
}
```

---

## 🛡️ Boas Práticas de Segurança

1. **Nunca exponha a porta do proxy diretamente na internet aberta sem autenticação ou firewall.**
2. Para acesso público seguro via internet, utilize a aba **Túnel** do FzComputerAI com **Cloudflare Tunnel** ou o listener **HTTPS nativo com OAuth 2.1**.
3. O micro-proxy deve ser utilizado prioritariamente para conexões em rede local privada ou ambientes de teste e desenvolvimento.
