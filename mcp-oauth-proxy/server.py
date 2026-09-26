"""
Servidor Proxy OAuth 2.1 para MCP (Model Context Protocol)
Compatível com Gemini Apps, Claude, e conectores MCP remotos.
Roteia requisições autenticadas para o motor local cua-driver (porta 8000).
Cross-platform: Windows e Linux.
"""

import os
import sys
import time
import json
import base64
import hashlib
import secrets
import argparse
import urllib.parse
from pathlib import Path
from typing import Dict, Any, Optional

import uvicorn
from fastapi import FastAPI, Request, Response
from fastapi.responses import HTMLResponse, JSONResponse, RedirectResponse
from fastapi.middleware.cors import CORSMiddleware
import aiohttp

# Configurações do Backend e Issuer (podem ser sobrescritas por args CLI ou env)
BACKEND_URL = os.environ.get("CUA_BACKEND_URL", "http://127.0.0.1:8000")
DEFAULT_ISSUER = os.environ.get("OAUTH_ISSUER", "https://mcpoahome.rogerluft.com.br")
PROXY_PORT = int(os.environ.get("OAUTH_PROXY_PORT", "8001"))
STATE_FILE = Path(__file__).parent / "oauth_state.json"

# Obter token do motor (CUA_DRIVER_RS_MCP_HTTP_TOKEN)
MOTOR_TOKEN = os.environ.get("CUA_DRIVER_RS_MCP_HTTP_TOKEN", "")
if not MOTOR_TOKEN:
    # Tenta ler do registro do Windows caso a variável não esteja no processo atual
    try:
        import winreg
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Environment") as key:
            MOTOR_TOKEN, _ = winreg.QueryValueEx(key, "CUA_DRIVER_RS_MCP_HTTP_TOKEN")
    except Exception:
        pass

if not MOTOR_TOKEN:
    # Tenta ler de arquivo padrão caso configurado
    token_file = Path.home() / ".config" / "fzcomputerai" / "token"
    if token_file.exists():
        try:
            MOTOR_TOKEN = token_file.read_text(encoding="utf-8").strip()
        except Exception:
            pass

if not MOTOR_TOKEN:
    print("[AVISO] CUA_DRIVER_RS_MCP_HTTP_TOKEN não encontrado. Defina a variável de ambiente ou configure no FzComputerAI.")
else:
    print(f"[OK] Token do motor CUA carregado ({len(MOTOR_TOKEN)} chars).")

# Estado em memória
state: Dict[str, Any] = {
    "clients": {},       # client_id -> {client_name, redirect_uris, created}
    "codes": {},         # code -> {client_id, redirect_uri, code_challenge, code_challenge_method, expires}
    "tokens": {},        # access_token -> {client_id, expires, scope}
    "refresh_tokens": {} # refresh_token -> {client_id, access_token, expires}
}

def load_state():
    global state
    if STATE_FILE.exists():
        try:
            with open(STATE_FILE, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                state.update(loaded)
        except Exception as e:
            print(f"[AVISO] Erro ao carregar state: {e}")

def save_state():
    try:
        # Salva apenas clients e tokens válidos
        clean_state = {
            "clients": state["clients"],
            "tokens": {k: v for k, v in state["tokens"].items() if v.get("expires", 0) > time.time()},
            "refresh_tokens": {k: v for k, v in state["refresh_tokens"].items() if v.get("expires", 0) > time.time()}
        }
        with open(STATE_FILE, "w", encoding="utf-8") as f:
            json.dump(clean_state, f, indent=2)
    except Exception as e:
        print(f"[AVISO] Erro ao salvar state: {e}")

load_state()

app = FastAPI(title="MCP OAuth 2.1 Proxy", version="1.0.0")

# Habilitar CORS irrestrito para clientes web
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

def get_issuer(request: Request) -> str:
    host = request.headers.get("x-forwarded-host") or request.headers.get("host")
    if host:
        proto = request.headers.get("x-forwarded-proto", "https")
        return f"{proto}://{host}"
    return DEFAULT_ISSUER

def b64url_sha256(verifier: str) -> str:
    digest = hashlib.sha256(verifier.encode("ascii")).digest()
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")

# -------------------------------------------------------------
# 1. Metadados RFC 9728 (Protected Resource)
# -------------------------------------------------------------
@app.get("/.well-known/oauth-protected-resource")
@app.get("/.well-known/oauth-protected-resource/mcp")
async def oauth_protected_resource(request: Request):
    issuer = get_issuer(request)
    return JSONResponse({
        "resource": f"{issuer}/mcp",
        "authorization_servers": [issuer],
        "bearer_methods_supported": ["header"],
        "scopes_supported": ["mcp", "offline_access"],
        "resource_name": "FzComputerAI Cua Driver MCP"
    })

# -------------------------------------------------------------
# 2. Metadados RFC 8414 & OpenID Configuration
# -------------------------------------------------------------
@app.get("/.well-known/oauth-authorization-server")
@app.get("/.well-known/openid-configuration")
async def oauth_authorization_server(request: Request):
    issuer = get_issuer(request)
    return JSONResponse({
        "issuer": issuer,
        "authorization_endpoint": f"{issuer}/authorize",
        "token_endpoint": f"{issuer}/token",
        "registration_endpoint": f"{issuer}/register",
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256", "plain"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_post", "client_secret_basic"],
        "client_id_metadata_document_supported": True,
        "scopes_supported": ["mcp", "offline_access"],
        "resource_indicators_supported": True
    })

# -------------------------------------------------------------
# 3. Dynamic Client Registration (RFC 7591)
# -------------------------------------------------------------
@app.post("/register")
async def register_client(request: Request):
    try:
        body = await request.json()
    except Exception:
        body = {}

    client_name = body.get("client_name", "Cliente MCP")
    redirect_uris = body.get("redirect_uris", [])
    
    # Gera credencial para o cliente
    client_id = "client_" + secrets.token_urlsafe(24)
    client_secret = "secret_" + secrets.token_urlsafe(32)
    now = int(time.time())

    state["clients"][client_id] = {
        "client_name": client_name,
        "redirect_uris": redirect_uris,
        "client_secret": client_secret,
        "created": now
    }
    save_state()

    return JSONResponse(status_code=201, content={
        "client_id": client_id,
        "client_secret": client_secret,
        "client_id_issued_at": now,
        "client_name": client_name,
        "redirect_uris": redirect_uris,
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
        "scope": "mcp"
    })

# -------------------------------------------------------------
# 4. Autorização com Autoconexão / Fallback (/authorize)
# -------------------------------------------------------------
@app.get("/authorize")
async def authorize_get(
    request: Request,
    client_id: Optional[str] = None,
    redirect_uri: Optional[str] = None,
    response_type: Optional[str] = "code",
    state_param: Optional[str] = None,
    code_challenge: Optional[str] = None,
    code_challenge_method: Optional[str] = "S256"
):
    query_params = dict(request.query_params)
    client_id = client_id or query_params.get("client_id", "default_client")
    redirect_uri = redirect_uri or query_params.get("redirect_uri", "")
    state_val = state_param or query_params.get("state", "")
    code_challenge = code_challenge or query_params.get("code_challenge", "")
    code_challenge_method = code_challenge_method or query_params.get("code_challenge_method", "S256")

    # Se client_id não existe, registra dinamicamente para não travar clientes como Gemini
    if client_id not in state["clients"]:
        state["clients"][client_id] = {
            "client_name": "Gemini/MCP Client",
            "redirect_uris": [redirect_uri] if redirect_uri else [],
            "created": int(time.time())
        }
        save_state()

    # Gera código de autorização válido por 5 minutos
    code = "code_" + secrets.token_urlsafe(32)
    state["codes"][code] = {
        "client_id": client_id,
        "redirect_uri": redirect_uri,
        "code_challenge": code_challenge,
        "code_challenge_method": code_challenge_method,
        "expires": int(time.time()) + 300
    }

    # Se não houver redirect_uri (ex: teste manual), devolve o código diretamente
    if not redirect_uri:
        return JSONResponse({"status": "authorized", "code": code, "state": state_val})

    # Monta a URL de retorno
    sep = "&" if "?" in redirect_uri else "?"
    loc = f"{redirect_uri}{sep}code={urllib.parse.quote(code)}"
    if state_val:
        loc += f"&state={urllib.parse.quote(state_val)}"

    # Página HTML com redirecionamento automático instantâneo e botão de fallback
    html_content = f"""<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8">
  <title>Conectando ao MCP...</title>
  <meta http-equiv="refresh" content="0; url={loc}">
  <style>
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: #0f172a;
      color: #f8fafc;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100vh;
      margin: 0;
    }}
    .card {{
      background: #1e293b;
      padding: 2.5rem;
      border-radius: 12px;
      box-shadow: 0 10px 25px rgba(0,0,0,0.5);
      text-align: center;
      max-width: 420px;
    }}
    h2 {{ margin-top: 0; color: #38bdf8; }}
    p {{ color: #94a3b8; font-size: 0.95rem; }}
    .btn {{
      display: inline-block;
      margin-top: 1.5rem;
      background: #0284c7;
      color: #ffffff;
      padding: 0.75rem 1.5rem;
      border-radius: 8px;
      text-decoration: none;
      font-weight: 600;
      transition: background 0.2s;
    }}
    .btn:hover {{ background: #0369a1; }}
  </style>
</head>
<body>
  <div class="card">
    <h2>✓ Autorização Concluída</h2>
    <p>Conectando o <b>Gemini / Agente MCP</b> ao seu computador local...</p>
    <a class="btn" href="{loc}">Clique aqui se não for redirecionado</a>
  </div>
  <script>
    setTimeout(function() {{
      window.location.href = "{loc}";
    }}, 50);
  </script>
</body>
</html>
"""
    return HTMLResponse(content=html_content, status_code=200, headers={"Location": loc})

# -------------------------------------------------------------
# 5. Emissão de Token (/token)
# -------------------------------------------------------------
@app.post("/token")
async def token_endpoint(request: Request):
    content_type = request.headers.get("content-type", "")
    if "application/json" in content_type:
        try:
            body = await request.json()
        except Exception:
            body = {}
    else:
        form = await request.form()
        body = dict(form)

    grant_type = body.get("grant_type")
    
    if grant_type == "authorization_code":
        code = body.get("code")
        verifier = body.get("code_verifier", "")
        
        auth_data = state["codes"].pop(code, None)
        if not auth_data:
            return JSONResponse(status_code=400, content={"error": "invalid_grant", "error_description": "Código de autorização inválido ou expirado"})

        if auth_data["expires"] < time.time():
            return JSONResponse(status_code=400, content={"error": "invalid_grant", "error_description": "Código de autorização expirou"})

        # Verificação PKCE
        challenge = auth_data.get("code_challenge")
        method = auth_data.get("code_challenge_method", "S256")
        if challenge and verifier:
            if method == "S256":
                computed = b64url_sha256(verifier)
                if computed != challenge:
                    return JSONResponse(status_code=400, content={"error": "invalid_grant", "error_description": "PKCE code_verifier inválido"})
            elif method == "plain":
                if verifier != challenge:
                    return JSONResponse(status_code=400, content={"error": "invalid_grant", "error_description": "PKCE code_verifier não coincide"})

        # Emite novo Access Token (30 dias de validade)
        access_token = "mcpoa_" + secrets.token_urlsafe(32)
        refresh_token = "mcpr_" + secrets.token_urlsafe(32)
        expires_in = 30 * 86400
        now = int(time.time())

        state["tokens"][access_token] = {
            "client_id": auth_data["client_id"],
            "expires": now + expires_in,
            "scope": "mcp"
        }
        state["refresh_tokens"][refresh_token] = {
            "client_id": auth_data["client_id"],
            "access_token": access_token,
            "expires": now + (60 * 86400)
        }
        save_state()

        return JSONResponse({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": expires_in,
            "refresh_token": refresh_token,
            "scope": "mcp"
        })

    elif grant_type == "refresh_token":
        ref_tok = body.get("refresh_token")
        ref_data = state["refresh_tokens"].get(ref_tok)
        if not ref_data or ref_data["expires"] < time.time():
            return JSONResponse(status_code=400, content={"error": "invalid_grant", "error_description": "Refresh token inválido ou expirado"})

        new_access = "mcpoa_" + secrets.token_urlsafe(32)
        expires_in = 30 * 86400
        now = int(time.time())

        state["tokens"][new_access] = {
            "client_id": ref_data["client_id"],
            "expires": now + expires_in,
            "scope": "mcp"
        }
        save_state()

        return JSONResponse({
            "access_token": new_access,
            "token_type": "Bearer",
            "expires_in": expires_in,
            "refresh_token": ref_tok,
            "scope": "mcp"
        })

    return JSONResponse(status_code=400, content={"error": "unsupported_grant_type", "error_description": "grant_type não suportado"})

# -------------------------------------------------------------
# 6. Proxy MCP Protegido (/mcp e subrotas)
# -------------------------------------------------------------
@app.api_route("/mcp{path:path}", methods=["GET", "POST", "PUT", "DELETE", "OPTIONS", "HEAD"])
async def mcp_proxy(request: Request, path: str):
    issuer = get_issuer(request)

    # Preflight CORS
    if request.method == "OPTIONS":
        return Response(status_code=200, headers={
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
            "Access-Control-Allow-Headers": "Authorization, Content-Type, Mcp-Protocol-Version",
        })

    auth_header = request.headers.get("authorization", "")
    
    # Validação do token de acesso
    token = ""
    if auth_header.lower().startswith("bearer "):
        token = auth_header[7:].strip()

    is_valid = False
    if token:
        # Se for o próprio token direto do motor
        if MOTOR_TOKEN and token == MOTOR_TOKEN:
            is_valid = True
        # Se for um token emitido pelo OAuth
        elif token in state["tokens"]:
            token_info = state["tokens"][token]
            if token_info.get("expires", 0) > time.time():
                is_valid = True

    if not is_valid:
        # RFC 9728: retorna 401 com o cabeçalho WWW-Authenticate indicando os metadados OAuth
        meta_url = f"{issuer}/.well-known/oauth-protected-resource"
        return JSONResponse(
            status_code=401,
            headers={
                "WWW-Authenticate": f'Bearer resource_metadata="{meta_url}"',
                "Access-Control-Allow-Origin": "*"
            },
            content={
                "jsonrpc": "2.0",
                "id": None,
                "error": {
                    "code": -32001,
                    "message": "Authentication required. Conecte via OAuth 2.1."
                }
            }
        )

    # Prepara o encaminhamento para o cua-driver local (http://127.0.0.1:8000/mcp)
    target_url = f"{BACKEND_URL}/mcp{path}"
    req_body = await request.body()
    
    # Headers para o backend (substituindo pelo bearer token real do motor)
    forward_headers = {
        "Content-Type": request.headers.get("content-type", "application/json"),
        "Authorization": f"Bearer {MOTOR_TOKEN}"
    }
    if "mcp-protocol-version" in request.headers:
        forward_headers["Mcp-Protocol-Version"] = request.headers["mcp-protocol-version"]

    try:
        async with aiohttp.ClientSession() as session:
            async with session.request(
                method=request.method,
                url=target_url,
                headers=forward_headers,
                data=req_body,
                timeout=aiohttp.ClientTimeout(total=60)
            ) as resp:
                resp_body = await resp.read()
                resp_headers = dict(resp.headers)
                resp_headers["Access-Control-Allow-Origin"] = "*"
                
                # Remove content-length e transfer-encoding para deixar o FastAPI gerenciar
                resp_headers.pop("Content-Length", None)
                resp_headers.pop("Transfer-Encoding", None)
                resp_headers.pop("Content-Encoding", None)

                return Response(
                    content=resp_body,
                    status_code=resp.status,
                    headers=resp_headers
                )
    except Exception as e:
        return JSONResponse(
            status_code=502,
            content={
                "jsonrpc": "2.0",
                "id": None,
                "error": {
                    "code": -32603,
                    "message": f"Erro ao comunicar com motor local: {str(e)}"
                }
            }
        )

def main():
    global PROXY_PORT, BACKEND_URL, DEFAULT_ISSUER, MOTOR_TOKEN

    parser = argparse.ArgumentParser(description="MCP OAuth 2.1 Proxy (Cross-platform)")
    parser.add_argument("--port", type=int, default=PROXY_PORT, help="Port to listen on (default: 8001)")
    parser.add_argument("--backend", type=str, default=BACKEND_URL, help="Backend CUA Driver URL (default: http://127.0.0.1:8000)")
    parser.add_argument("--issuer", type=str, default=DEFAULT_ISSUER, help="Default OAuth Issuer URL")
    parser.add_argument("--token", type=str, default="", help="CUA Driver Motor Token")
    args = parser.parse_args()

    PROXY_PORT = args.port
    BACKEND_URL = args.backend
    DEFAULT_ISSUER = args.issuer
    if args.token:
        MOTOR_TOKEN = args.token

    print(f"========================================================")
    print(f" Iniciando Servidor Proxy OAuth 2.1 / LAN")
    print(f" Escutando em: 0.0.0.0:{PROXY_PORT}")
    print(f" Encaminhando para: {BACKEND_URL}")
    print(f" Issuer Padrao: {DEFAULT_ISSUER}")
    print(f" Token Motor Presente: {'SIM' if bool(MOTOR_TOKEN) else 'NAO'}")
    print(f"========================================================")
    uvicorn.run(app, host="0.0.0.0", port=PROXY_PORT, log_level="info")

if __name__ == "__main__":
    main()
