# FzComputerAI — Servidor Proxy OAuth 2.1 para MCP

Este proxy standalone em Python permite expor o motor local **`cua-driver`** (porta 8000) para conectores remotos MCP que exigem autenticação OAuth 2.1 (como **Gemini Apps**, **Claude**, **Cursor**, **Codex**, etc.).

---

## 🚀 Como Executar

### Pré-requisitos
* Python 3.10 ou superior instalado e no PATH do sistema.

### Opção 1: Via script de inicialização rápida (Windows)
Basta clicar duas vezes em:
```cmd
start-proxy.bat
```
O script instalará automaticamente as dependências (`fastapi`, `uvicorn`, `aiohttp`) caso não estejam presentes e iniciará o proxy na porta configurada (padrão: 8001).

### Opção 2: Linha de comando manual
1. Instale as dependências:
   ```bash
   pip install -r requirements.txt
   ```
2. Inicie o servidor:
   ```bash
   python server.py --port 8001
   ```

---

## ⚙️ Variáveis de Ambiente e Parâmetros

* `CUA_BACKEND_URL`: URL do motor CUA local (padrão: `http://127.0.0.1:8000`).
* `OAUTH_PROXY_PORT`: Porta de escuta do proxy (padrão: `8001`).
* `OAUTH_ISSUER`: URL pública ou domínio utilizado como emissor OAuth (padrão: `https://mcpoahome.rogerluft.com.br`).
* `CUA_DRIVER_RS_MCP_HTTP_TOKEN`: Bearer token do motor (carregado automaticamente da variável de ambiente do usuário gravada pelo FzComputerAI).
