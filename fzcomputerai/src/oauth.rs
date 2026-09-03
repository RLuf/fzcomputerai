//! OAuth 2.1 mínimo para conectores MCP — servido DENTRO do listener HTTPS.
//!
//! Conectores hospedados (Claude.ai, Gemini) e clientes que seguem a
//! especificação de autorização do MCP não aceitam bearer token estático: eles
//! descobrem o servidor de autorização por metadata, registram-se sozinhos
//! (RFC 7591), fazem authorization-code com PKCE S256 (RFC 7636) e trocam o
//! código por token. Este módulo implementa exatamente esse mínimo:
//!
//!   GET  /.well-known/oauth-protected-resource   (RFC 9728)
//!   GET  /.well-known/oauth-authorization-server (RFC 8414)
//!   POST /register                               (RFC 7591, sem segredo)
//!   GET  /authorize  → página com a SENHA DE AUTORIZAÇÃO do app
//!   POST /authorize  → valida a senha, emite code, redireciona
//!   POST /token      → authorization_code (PKCE) e refresh_token
//!
//! O token de acesso emitido é aleatório e vive só aqui; o proxy TLS troca
//! `Authorization: Bearer <token OAuth>` pelo bearer token do MOTOR antes de
//! encaminhar. O motor nunca vê o token OAuth e o cliente nunca vê o do motor.
//!
//! Estado (clientes, códigos, tokens, hash da senha) em `oauth-state.json`
//! (0600) na pasta dos certificados. Nada no registro, nada em log.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

pub const STATE_FILE: &str = "oauth-state.json";
pub const ACCESS_TTL_SECS: u64 = 24 * 3600;
pub const REFRESH_TTL_SECS: u64 = 30 * 24 * 3600;
pub const CODE_TTL_SECS: u64 = 300;
pub const SCOPE: &str = "mcp";

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Client {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub created: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthCode {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub expires: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Token {
    pub client_id: String,
    pub expires: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    /// SHA-256 hex da senha de autorização.
    pub password_sha256: String,
    pub clients: HashMap<String, Client>,
    pub codes: HashMap<String, AuthCode>,
    pub access: HashMap<String, Token>,
    pub refresh: HashMap<String, Token>,
}

pub struct OAuthServer {
    state: Mutex<State>,
    path: PathBuf,
}

/// Requisição HTTP já lida pelo proxy (cabeçalho + corpo).
pub struct HttpReq<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a [u8],
}

pub struct HttpResp {
    pub status: u16,
    pub content_type: &'static str,
    pub extra_headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResp {
    fn json(status: u16, v: serde_json::Value) -> Self {
        HttpResp { status, content_type: "application/json", extra_headers: vec![("Cache-Control".into(), "no-store".into())], body: v.to_string().into_bytes() }
    }
    fn html(status: u16, s: String) -> Self {
        HttpResp { status, content_type: "text/html; charset=utf-8", extra_headers: vec![("Cache-Control".into(), "no-store".into())], body: s.into_bytes() }
    }
    fn redirect(location: String) -> Self {
        HttpResp { status: 302, content_type: "text/plain", extra_headers: vec![("Location".into(), location), ("Cache-Control".into(), "no-store".into())], body: Vec::new() }
    }
    fn oauth_error(status: u16, error: &str, desc: &str) -> Self {
        Self::json(status, serde_json::json!({"error": error, "error_description": desc}))
    }
    pub fn serialize(&self) -> Vec<u8> {
        let reason = match self.status { 200 => "OK", 201 => "Created", 302 => "Found", 400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden", 404 => "Not Found", 405 => "Method Not Allowed", _ => "OK" };
        let mut out = format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type, Mcp-Protocol-Version\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\n", self.status, reason, self.content_type, self.body.len());
        for (k, v) in &self.extra_headers {
            out.push_str(&format!("{k}: {v}\r\n"));
        }
        out.push_str("\r\n");
        let mut b = out.into_bytes();
        b.extend_from_slice(&self.body);
        b
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn sha256_hex(data: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, data).as_ref().iter().map(|b| format!("{:02x}", b)).collect()
}

fn b64url_nopad(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk.len();
        let b = [chunk[0], if n > 1 { chunk[1] } else { 0 }, if n > 2 { chunk[2] } else { 0 }];
        let v = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(v >> 18) as usize & 63] as char);
        out.push(T[(v >> 12) as usize & 63] as char);
        if n > 1 { out.push(T[(v >> 6) as usize & 63] as char); }
        if n > 2 { out.push(T[v as usize & 63] as char); }
    }
    out
}

/// base64url(SHA-256(input)) — o `code_challenge` do PKCE S256.
pub fn b64url_sha256(input: &str) -> String {
    b64url_nopad(ring::digest::digest(&ring::digest::SHA256, input.as_bytes()).as_ref())
}

/// Token/segredo aleatório (32 bytes do RNG do sistema, base64url).
pub fn random_token() -> String {
    use ring::rand::SecureRandom;
    let mut b = [0u8; 32];
    let _ = ring::rand::SystemRandom::new().fill(&mut b);
    b64url_nopad(&b)
}

/// Senha legível para o usuário digitar na página de autorização.
pub fn random_password() -> String {
    use ring::rand::SecureRandom;
    const A: &[u8] = b"abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut b = [0u8; 20];
    let _ = ring::rand::SystemRandom::new().fill(&mut b);
    b.iter().map(|x| A[*x as usize % A.len()] as char).collect()
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => { out.push(b' '); i += 1; }
            b'%' if i + 2 < b.len() + 0 && i + 2 <= b.len() - 1 => {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; } else { out.push(b'%'); i += 1; }
            }
            c => { out.push(c); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn parse_form(s: &str) -> HashMap<String, String> {
    s.split('&').filter(|p| !p.is_empty()).filter_map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        Some((url_decode(k), url_decode(v)))
    }).collect()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// Compara redirect_uri com os registrados. Para loopback (`localhost`,
/// `127.0.0.1`, `[::1]`) a PORTA é ignorada (RFC 8252 §7.3) — o Claude Code
/// declara `http://localhost/callback` e usa uma porta efêmera por sessão.
pub fn redirect_matches(registered: &[String], candidate: &str) -> bool {
    fn split(u: &str) -> Option<(String, String, String)> {
        let (scheme, rest) = u.split_once("://")?;
        let (hostport, path) = match rest.find('/') { Some(i) => (&rest[..i], &rest[i..]), None => (rest, "/") };
        let host = hostport.rsplit_once(':').map(|(h, p)| if p.chars().all(|c| c.is_ascii_digit()) { h } else { hostport }).unwrap_or(hostport);
        Some((scheme.to_lowercase(), host.trim_matches(|c| c == '[' || c == ']').to_lowercase(), path.to_string()))
    }
    if registered.iter().any(|r| r == candidate) {
        return true;
    }
    let Some((cs, ch, cp)) = split(candidate) else { return false };
    let loopback = matches!(ch.as_str(), "localhost" | "127.0.0.1" | "::1");
    if !loopback {
        return false;
    }
    registered.iter().any(|r| match split(r) {
        Some((rs, rh, rp)) => rs == cs && rp == cp && matches!(rh.as_str(), "localhost" | "127.0.0.1" | "::1"),
        None => false,
    })
}

/// Client ID Metadata Document (MCP 2025-11-25): o `client_id` É uma URL
/// https que serve um JSON com `redirect_uris`/`client_name`. O Claude Code
/// usa `https://claude.ai/oauth/claude-code-client-metadata`. Busca, valida
/// (o `client_id` do documento tem de ser a própria URL) e registra em cache.
fn is_cimd_client_id(client_id: &str) -> bool {
    client_id.starts_with("https://") && client_id.len() < 512
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

impl OAuthServer {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join(STATE_FILE);
        let state = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<State>(&s).ok()).unwrap_or_default();
        OAuthServer { state: Mutex::new(state), path }
    }

    fn save(&self, st: &State) {
        if let Ok(json) = serde_json::to_string_pretty(st) {
            if let Some(p) = self.path.parent() { let _ = std::fs::create_dir_all(p); }
            let _ = std::fs::write(&self.path, json);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
            }
        }
    }

    pub fn has_password(&self) -> bool {
        self.state.lock().map(|s| !s.password_sha256.is_empty()).unwrap_or(false)
    }

    /// Define a senha de autorização (guardada só como SHA-256).
    pub fn set_password(&self, password: &str) {
        if let Ok(mut st) = self.state.lock() {
            st.password_sha256 = sha256_hex(password.trim().as_bytes());
            self.save(&st);
        }
    }

    pub fn clients_count(&self) -> usize {
        self.state.lock().map(|s| s.clients.len()).unwrap_or(0)
    }
    pub fn tokens_count(&self) -> usize {
        let n = now();
        self.state.lock().map(|s| s.access.values().filter(|t| t.expires > n).count()).unwrap_or(0)
    }

    /// Revoga tudo (clientes, códigos, tokens); a senha fica.
    pub fn revoke_all(&self) {
        if let Ok(mut st) = self.state.lock() {
            st.clients.clear();
            st.codes.clear();
            st.access.clear();
            st.refresh.clear();
            self.save(&st);
        }
    }

    fn gc(st: &mut State) {
        let n = now();
        st.codes.retain(|_, c| c.expires > n);
        st.access.retain(|_, t| t.expires > n);
        st.refresh.retain(|_, t| t.expires > n);
    }

    /// Cliente registrado — ou, se `client_id` for uma URL https (CIMD),
    /// busca o documento e registra. Devolve None se desconhecido/inválido.
    fn resolve_client(&self, client_id: &str) -> Option<Client> {
        if let Some(c) = self.state.lock().ok().and_then(|s| s.clients.get(client_id).cloned()) {
            return Some(c);
        }
        if !is_cimd_client_id(client_id) {
            return None;
        }
        let (st, body) = crate::tls::https_get(client_id).ok()?;
        if st != 200 {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(&body).ok()?;
        if v.get("client_id").and_then(|x| x.as_str()) != Some(client_id) {
            return None;
        }
        let redirect_uris: Vec<String> = v.get("redirect_uris")?.as_array()?.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
        if redirect_uris.is_empty() {
            return None;
        }
        let client = Client {
            client_name: v.get("client_name").and_then(|x| x.as_str()).unwrap_or(client_id).chars().take(80).collect(),
            redirect_uris,
            created: now(),
        };
        if let Ok(mut st) = self.state.lock() {
            st.clients.insert(client_id.to_string(), client.clone());
            self.save(&st);
        }
        Some(client)
    }

    /// Token de acesso OAuth válido? (chamado pelo proxy a cada requisição)
    pub fn validate_access(&self, token: &str) -> bool {
        let n = now();
        self.state.lock().map(|s| s.access.get(token).map(|t| t.expires > n).unwrap_or(false)).unwrap_or(false)
    }

    /// Cabeçalho que o recurso protegido devolve no 401 (RFC 9728 §5.1).
    pub fn www_authenticate(issuer: &str) -> String {
        format!("Bearer resource_metadata=\"{issuer}/.well-known/oauth-protected-resource\"")
    }

    /// Trata uma requisição. `None` = não é caminho OAuth (o proxy encaminha).
    /// `issuer` = `https://host[:porta]` da requisição.
    pub fn handle(&self, req: &HttpReq<'_>, issuer: &str) -> Option<HttpResp> {
        let path_only = req.path.split('?').next().unwrap_or("");
        let query = req.path.split_once('?').map(|(_, q)| q).unwrap_or("");
        if req.method == "OPTIONS" && matches!(path_only, "/register" | "/token" | "/authorize" | "/.well-known/oauth-authorization-server" | "/.well-known/oauth-protected-resource") {
            return Some(HttpResp { status: 200, content_type: "text/plain", extra_headers: vec![], body: Vec::new() });
        }
        match (req.method, path_only) {
            ("GET", "/.well-known/oauth-protected-resource") | ("GET", "/.well-known/oauth-protected-resource/mcp") => Some(HttpResp::json(200, serde_json::json!({
                "resource": format!("{issuer}/mcp"),
                "authorization_servers": [issuer],
                "bearer_methods_supported": ["header"],
                "scopes_supported": [SCOPE, "offline_access"],
                "resource_name": "FzComputerAI MCP"
            }))),
            ("GET", "/.well-known/oauth-authorization-server") | ("GET", "/.well-known/openid-configuration") => Some(HttpResp::json(200, serde_json::json!({
                "issuer": issuer,
                "authorization_endpoint": format!("{issuer}/authorize"),
                "token_endpoint": format!("{issuer}/token"),
                "registration_endpoint": format!("{issuer}/register"),
                "response_types_supported": ["code"],
                "response_modes_supported": ["query"],
                "grant_types_supported": ["authorization_code", "refresh_token"],
                "code_challenge_methods_supported": ["S256"],
                "token_endpoint_auth_methods_supported": ["none"],
                "client_id_metadata_document_supported": true,
                "scopes_supported": [SCOPE, "offline_access"],
                "resource_indicators_supported": true,
                "service_documentation": "https://github.com/RLuf/fzcomputerai/blob/master/docs/https.md"
            }))),
            ("POST", "/register") => Some(self.register(req.body)),
            ("GET", "/authorize") => Some(self.authorize_page(query, None)),
            ("POST", "/authorize") => Some(self.authorize_submit(req.body)),
            ("POST", "/token") => Some(self.token(req.body)),
            (_, "/register") | (_, "/authorize") | (_, "/token") => Some(HttpResp::oauth_error(405, "invalid_request", "método não suportado")),
            _ => None,
        }
    }

    fn register(&self, body: &[u8]) -> HttpResp {
        let v: serde_json::Value = match serde_json::from_slice(body) { Ok(v) => v, Err(_) => return HttpResp::oauth_error(400, "invalid_client_metadata", "corpo não é JSON") };
        let redirect_uris: Vec<String> = v.get("redirect_uris").and_then(|r| r.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
        if redirect_uris.is_empty() {
            return HttpResp::oauth_error(400, "invalid_redirect_uri", "redirect_uris obrigatório");
        }
        for u in &redirect_uris {
            let ok = u.starts_with("https://") || u.starts_with("http://localhost") || u.starts_with("http://127.0.0.1") || (!u.starts_with("http://") && u.contains("://"));
            if !ok {
                return HttpResp::oauth_error(400, "invalid_redirect_uri", "redirect_uri precisa ser https:// (ou http://localhost)");
            }
        }
        let client_name = v.get("client_name").and_then(|x| x.as_str()).unwrap_or("cliente MCP").chars().take(80).collect::<String>();
        let client_id = random_token();
        let created = now();
        if let Ok(mut st) = self.state.lock() {
            Self::gc(&mut st);
            st.clients.insert(client_id.clone(), Client { client_name: client_name.clone(), redirect_uris: redirect_uris.clone(), created });
            self.save(&st);
        }
        HttpResp::json(201, serde_json::json!({
            "client_id": client_id,
            "client_id_issued_at": created,
            "client_name": client_name,
            "redirect_uris": redirect_uris,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
            "scope": SCOPE
        }))
    }

    fn authorize_page(&self, query: &str, error: Option<&str>) -> HttpResp {
        let q = parse_form(query);
        let client_id = q.get("client_id").cloned().unwrap_or_default();
        let redirect_uri = q.get("redirect_uri").cloned().unwrap_or_default();
        let state = q.get("state").cloned().unwrap_or_default();
        let challenge = q.get("code_challenge").cloned().unwrap_or_default();
        let method = q.get("code_challenge_method").cloned().unwrap_or_default();
        let response_type = q.get("response_type").cloned().unwrap_or_default();
        let Some(client) = self.resolve_client(&client_id) else {
            return HttpResp::html(400, page("Cliente desconhecido", "<p>Este <code>client_id</code> não está registrado (nem é um Client ID Metadata Document válido). O conector precisa registrar-se em <code>/register</code> primeiro.</p>".into()));
        };
        if !redirect_matches(&client.redirect_uris, &redirect_uri) {
            return HttpResp::html(400, page("redirect_uri inválido", "<p>O <code>redirect_uri</code> não confere com o registrado pelo cliente.</p>".into()));
        }
        if response_type != "code" || method != "S256" || challenge.is_empty() {
            return HttpResp::html(400, page("Requisição inválida", "<p>Exigido: <code>response_type=code</code> e PKCE <code>code_challenge_method=S256</code>.</p>".into()));
        }
        if !self.has_password() {
            return HttpResp::html(503, page("Sem senha de autorização", "<p>Defina a senha de autorização no FzComputerAI (aba MCP &amp; Rede → HTTPS → OAuth) antes de conectar.</p>".into()));
        }
        let err_html = error.map(|e| format!("<p class=\"err\">{}</p>", html_escape(e))).unwrap_or_default();
        let loop_warn = if redirect_uri.starts_with("http://localhost") || redirect_uri.starts_with("http://127.0.0.1") {
            "<p class=\"err\">Atenção: o retorno é para um endereço local desta máquina (loopback). Só autorize se você mesmo acabou de iniciar o login no cliente (ex.: Claude Code).</p>"
        } else { "" };
        let body = format!(
            "<p><b>{}</b> quer acessar o MCP desta máquina.</p><p>Redirecionamento: <code>{}</code></p>{}{}<form method=\"post\" action=\"/authorize\">\
             <input type=\"hidden\" name=\"client_id\" value=\"{}\"><input type=\"hidden\" name=\"redirect_uri\" value=\"{}\"><input type=\"hidden\" name=\"state\" value=\"{}\"><input type=\"hidden\" name=\"code_challenge\" value=\"{}\">\
             <label>Senha de autorização do FzComputerAI<br><input type=\"password\" name=\"password\" autofocus autocomplete=\"current-password\"></label><br><br>\
             <button type=\"submit\">Autorizar</button> <a href=\"{}?error=access_denied&state={}\">Negar</a></form>",
            html_escape(&client.client_name), html_escape(&redirect_uri), loop_warn, err_html,
            html_escape(&client_id), html_escape(&redirect_uri), html_escape(&state), html_escape(&challenge),
            html_escape(&redirect_uri), url_encode(&state)
        );
        HttpResp::html(200, page("Autorizar acesso ao MCP", body))
    }

    fn authorize_submit(&self, body: &[u8]) -> HttpResp {
        let f = parse_form(&String::from_utf8_lossy(body));
        let get = |k: &str| f.get(k).cloned().unwrap_or_default();
        let (client_id, redirect_uri, state, challenge, password) = (get("client_id"), get("redirect_uri"), get("state"), get("code_challenge"), get("password"));
        let ok_client = self.resolve_client(&client_id).map(|c| redirect_matches(&c.redirect_uris, &redirect_uri)).unwrap_or(false);
        if !ok_client || challenge.is_empty() {
            return HttpResp::html(400, page("Requisição inválida", "<p>Cliente ou redirect_uri inválidos.</p>".into()));
        }
        let expected = self.state.lock().map(|s| s.password_sha256.clone()).unwrap_or_default();
        if expected.is_empty() || sha256_hex(password.trim().as_bytes()) != expected {
            // Reapresenta a página com erro (sem vazar nada).
            let q = format!("client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256&response_type=code", url_encode(&client_id), url_encode(&redirect_uri), url_encode(&state), url_encode(&challenge));
            std::thread::sleep(std::time::Duration::from_millis(800));
            return self.authorize_page(&q, Some("Senha incorreta."));
        }
        let code = random_token();
        if let Ok(mut st) = self.state.lock() {
            Self::gc(&mut st);
            st.codes.insert(code.clone(), AuthCode { client_id, redirect_uri: redirect_uri.clone(), code_challenge: challenge, expires: now() + CODE_TTL_SECS });
            self.save(&st);
        }
        let sep = if redirect_uri.contains('?') { '&' } else { '?' };
        let mut loc = format!("{redirect_uri}{sep}code={}", url_encode(&code));
        if !state.is_empty() { loc.push_str(&format!("&state={}", url_encode(&state))); }
        HttpResp::redirect(loc)
    }

    fn token(&self, body: &[u8]) -> HttpResp {
        let f = parse_form(&String::from_utf8_lossy(body));
        let get = |k: &str| f.get(k).cloned().unwrap_or_default();
        match get("grant_type").as_str() {
            "authorization_code" => {
                let (code, verifier, client_id, redirect_uri) = (get("code"), get("code_verifier"), get("client_id"), get("redirect_uri"));
                let mut st = match self.state.lock() { Ok(s) => s, Err(_) => return HttpResp::oauth_error(500, "server_error", "estado") };
                Self::gc(&mut st);
                let Some(ac) = st.codes.remove(&code) else { return HttpResp::oauth_error(400, "invalid_grant", "código inválido ou expirado") };
                if !client_id.is_empty() && client_id != ac.client_id {
                    return HttpResp::oauth_error(400, "invalid_grant", "client_id não confere");
                }
                if !redirect_uri.is_empty() && redirect_uri != ac.redirect_uri && !redirect_matches(&[ac.redirect_uri.clone()], &redirect_uri) {
                    return HttpResp::oauth_error(400, "invalid_grant", "redirect_uri não confere");
                }
                let expected = b64url_nopad(ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes()).as_ref());
                if verifier.is_empty() || expected != ac.code_challenge {
                    return HttpResp::oauth_error(400, "invalid_grant", "PKCE: code_verifier não confere");
                }
                let resp = Self::issue(&mut st, &ac.client_id);
                self.save(&st);
                resp
            }
            "refresh_token" => {
                let rt = get("refresh_token");
                let mut st = match self.state.lock() { Ok(s) => s, Err(_) => return HttpResp::oauth_error(500, "server_error", "estado") };
                Self::gc(&mut st);
                let Some(t) = st.refresh.remove(&rt) else { return HttpResp::oauth_error(400, "invalid_grant", "refresh_token inválido ou expirado") };
                let cid = get("client_id");
                if !cid.is_empty() && cid != t.client_id {
                    return HttpResp::oauth_error(400, "invalid_grant", "client_id não confere");
                }
                let resp = Self::issue(&mut st, &t.client_id);
                self.save(&st);
                resp
            }
            _ => HttpResp::oauth_error(400, "unsupported_grant_type", "use authorization_code ou refresh_token"),
        }
    }

    fn issue(st: &mut State, client_id: &str) -> HttpResp {
        let access = random_token();
        let refresh = random_token();
        st.access.insert(access.clone(), Token { client_id: client_id.to_string(), expires: now() + ACCESS_TTL_SECS });
        st.refresh.insert(refresh.clone(), Token { client_id: client_id.to_string(), expires: now() + REFRESH_TTL_SECS });
        HttpResp::json(200, serde_json::json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": ACCESS_TTL_SECS,
            "refresh_token": refresh,
            "scope": SCOPE
        }))
    }
}

fn page(title: &str, body: String) -> String {
    format!("<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{t} — FzComputerAI</title>\
<style>body{{font-family:system-ui,sans-serif;background:#0d1117;color:#e6edf3;margin:0;display:flex;justify-content:center;padding:40px 16px}}main{{max-width:520px;width:100%;background:#161b22;border:1px solid #30363d;border-radius:8px;padding:28px}}h1{{font-size:20px;margin:0 0 16px}}code{{background:#0d1117;padding:2px 6px;border-radius:4px;word-break:break-all}}input[type=password]{{width:100%;padding:10px;border-radius:6px;border:1px solid #30363d;background:#0d1117;color:#e6edf3;font-size:16px}}button{{background:#238636;color:#fff;border:0;padding:10px 18px;border-radius:6px;font-size:15px;cursor:pointer}}a{{color:#8b949e;margin-left:12px}}.err{{color:#f85149}}.foot{{color:#8b949e;font-size:12px;margin-top:20px}}</style></head>\
<body><main><h1>{t}</h1>{b}<p class=\"foot\">FzComputerAI — OAuth 2.1 (PKCE S256). A senha de autorização é definida na aba MCP &amp; Rede → HTTPS → OAuth.</p></main></body></html>", t = html_escape(title), b = body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_redirect_ignores_port() {
        let reg = vec!["http://localhost/callback".to_string(), "http://127.0.0.1/callback".to_string()];
        assert!(redirect_matches(&reg, "http://localhost:3118/callback"));
        assert!(redirect_matches(&reg, "http://127.0.0.1:49152/callback"));
        assert!(!redirect_matches(&reg, "http://localhost:3118/other"));
        assert!(!redirect_matches(&reg, "https://evil.example/callback"));
        let reg2 = vec!["https://claude.ai/api/mcp/auth_callback".to_string()];
        assert!(redirect_matches(&reg2, "https://claude.ai/api/mcp/auth_callback"));
        assert!(!redirect_matches(&reg2, "https://claude.ai:444/api/mcp/auth_callback"));
    }

    #[test]
    fn full_flow_register_authorize_token_refresh() {
        let dir = std::env::temp_dir().join(format!("fz-oauth-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let srv = OAuthServer::load(&dir);
        srv.set_password("segredo-forte");
        let issuer = "https://mcp.exemplo.com.br:8444";
        let hdrs: Vec<(String, String)> = vec![];

        // metadata
        let r = srv.handle(&HttpReq { method: "GET", path: "/.well-known/oauth-authorization-server", headers: &hdrs, body: b"" }, issuer).unwrap();
        assert_eq!(r.status, 200);
        let meta: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(meta["token_endpoint"], format!("{issuer}/token"));
        let r = srv.handle(&HttpReq { method: "GET", path: "/.well-known/oauth-protected-resource", headers: &hdrs, body: b"" }, issuer).unwrap();
        assert_eq!(r.status, 200);

        // register
        let r = srv.handle(&HttpReq { method: "POST", path: "/register", headers: &hdrs, body: br#"{"client_name":"Claude","redirect_uris":["https://claude.ai/api/mcp/auth_callback"]}"# }, issuer).unwrap();
        assert_eq!(r.status, 201, "{}", String::from_utf8_lossy(&r.body));
        let reg: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        let cid = reg["client_id"].as_str().unwrap().to_string();

        // PKCE
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = b64url_nopad(ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes()).as_ref());
        let q = format!("response_type=code&client_id={}&redirect_uri={}&state=xyz&code_challenge={}&code_challenge_method=S256", url_encode(&cid), url_encode("https://claude.ai/api/mcp/auth_callback"), challenge);
        let r = srv.handle(&HttpReq { method: "GET", path: &format!("/authorize?{q}"), headers: &hdrs, body: b"" }, issuer).unwrap();
        assert_eq!(r.status, 200);
        assert!(String::from_utf8_lossy(&r.body).contains("Autorizar"));

        // senha errada
        let form = format!("client_id={}&redirect_uri={}&state=xyz&code_challenge={}&password=errada", url_encode(&cid), url_encode("https://claude.ai/api/mcp/auth_callback"), challenge);
        let r = srv.handle(&HttpReq { method: "POST", path: "/authorize", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        assert_eq!(r.status, 200);
        assert!(String::from_utf8_lossy(&r.body).contains("Senha incorreta"));

        // senha certa -> 302 com code
        let form = format!("client_id={}&redirect_uri={}&state=xyz&code_challenge={}&password=segredo-forte", url_encode(&cid), url_encode("https://claude.ai/api/mcp/auth_callback"), challenge);
        let r = srv.handle(&HttpReq { method: "POST", path: "/authorize", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        assert_eq!(r.status, 302);
        let loc = r.extra_headers.iter().find(|(k, _)| k == "Location").unwrap().1.clone();
        assert!(loc.starts_with("https://claude.ai/api/mcp/auth_callback?code="));
        assert!(loc.ends_with("&state=xyz"));
        let code = loc.split("code=").nth(1).unwrap().split('&').next().unwrap().to_string();

        // token com verifier errado
        let form = format!("grant_type=authorization_code&code={code}&client_id={}&code_verifier=nope", url_encode(&cid));
        let r = srv.handle(&HttpReq { method: "POST", path: "/token", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        assert_eq!(r.status, 400);
        // codigo ja consumido -> precisa de um novo
        let form = format!("client_id={}&redirect_uri={}&state=xyz&code_challenge={}&password=segredo-forte", url_encode(&cid), url_encode("https://claude.ai/api/mcp/auth_callback"), challenge);
        let r = srv.handle(&HttpReq { method: "POST", path: "/authorize", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        let loc = r.extra_headers.iter().find(|(k, _)| k == "Location").unwrap().1.clone();
        let code = loc.split("code=").nth(1).unwrap().split('&').next().unwrap().to_string();
        let form = format!("grant_type=authorization_code&code={code}&client_id={}&code_verifier={verifier}&redirect_uri={}", url_encode(&cid), url_encode("https://claude.ai/api/mcp/auth_callback"));
        let r = srv.handle(&HttpReq { method: "POST", path: "/token", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
        let tok: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        let access = tok["access_token"].as_str().unwrap().to_string();
        let refresh = tok["refresh_token"].as_str().unwrap().to_string();
        assert!(srv.validate_access(&access));
        assert!(!srv.validate_access("outro"));

        // refresh
        let form = format!("grant_type=refresh_token&refresh_token={refresh}&client_id={}", url_encode(&cid));
        let r = srv.handle(&HttpReq { method: "POST", path: "/token", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        assert_eq!(r.status, 200);
        // refresh reutilizado -> invalido (rotacao)
        let r = srv.handle(&HttpReq { method: "POST", path: "/token", headers: &hdrs, body: form.as_bytes() }, issuer).unwrap();
        assert_eq!(r.status, 400);

        // persistencia
        let srv2 = OAuthServer::load(&dir);
        assert!(srv2.validate_access(&access));
        assert_eq!(srv2.clients_count(), 1);
        srv2.revoke_all();
        assert!(!srv2.validate_access(&access));

        // caminho nao-oauth -> None
        assert!(srv.handle(&HttpReq { method: "POST", path: "/mcp", headers: &hdrs, body: b"{}" }, issuer).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
