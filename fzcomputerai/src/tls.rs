//! HTTPS para o endpoint MCP — terminação TLS DENTRO do app.
//!
//! O motor `cua-driver serve` escuta SOMENTE em `127.0.0.1:<porta>` e SOMENTE
//! em HTTP (endereço e transporte fixos no código do projeto Cua). Clientes MCP
//! que exigem `https://` (conectores hospedados, políticas corporativas,
//! navegadores com mixed-content) não conseguem falar com ele diretamente.
//!
//! Este módulo resolve isso com o MESMO desenho do Encaminhamento LAN da
//! v2.1.1: uma thread do próprio processo escuta em `<bind>:<porta_tls>`,
//! termina o TLS com `rustls` e copia bytes contra `127.0.0.1:<porta_http>`.
//! É TCP puro depois do handshake — o bearer token do motor continua sendo
//! exigido exatamente como antes; o HTTPS só protege o transporte.
//!
//! Três origens de certificado, todas gravadas em `cert_dir()`:
//!   * **Auto-assinado** (padrão, zero configuração): gerado com `rcgen` no
//!     primeiro uso — ou na instalação, via `fzcomputerai --tls-init`, o que
//!     vier primeiro. Renovado sozinho quando faltam < 30 dias.
//!   * **Let's Encrypt** (ACME RFC 8555, `instant-acme`): exige um domínio
//!     público apontando para esta máquina e a porta 80 alcançável da internet
//!     (desafio HTTP-01, respondido por um listener temporário deste módulo).
//!     Renovação automática quando faltam < 30 dias.
//!   * **Próprio**: caminhos de `.crt`/`.key` PEM informados pelo usuário.
//!
//! LIMITE NORMATIVO (AGENTS.md §4.1): o certificado auto-assinado é um
//! certificado de SERVIDOR TLS para este endpoint — NUNCA é instalado em
//! nenhuma store de confiança da máquina (`Cert:\CurrentUser\Root` etc.) e
//! NUNCA é usado para assinar binário. O cliente que quiser confiar nele faz
//! isso do lado dele (pin do fingerprint SHA-256 exibido na tela, ou
//! `--cacert` apontando para o `.crt`). Não reintroduza instalação de CA.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Context, Result};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};

/// Renova (auto-assinado ou ACME) quando faltar menos que isto para expirar.
pub const RENEW_BEFORE_DAYS: i64 = 30;
/// Validade do auto-assinado. 825 dias é o teto que Apple/Chrome aceitam
/// para cert de servidor — acima disso alguns clientes recusam de cara.
pub const SELF_SIGNED_DAYS: i64 = 825;
/// Porta padrão do listener HTTPS (a 8000 continua sendo o HTTP do motor).
pub const DEFAULT_TLS_PORT: u16 = 8443;
/// Porta do desafio ACME HTTP-01 (fixa pelo protocolo, RFC 8555 §8.3).
pub const ACME_HTTP01_PORT: u16 = 80;

pub const SELF_SIGNED_CERT: &str = "selfsigned.crt";
pub const SELF_SIGNED_KEY: &str = "selfsigned.key";
pub const ACME_CERT: &str = "letsencrypt.crt";
pub const ACME_KEY: &str = "letsencrypt.key";
pub const ACME_ACCOUNT: &str = "letsencrypt-account.json";
pub const INIT_LOG: &str = "tls-init.log";

/// Garante que o rustls tem um CryptoProvider instalado (ring). Idempotente:
/// chamado no início de `main` e defensivamente antes de qualquer uso.
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

// ───────────────────────────── diretório ─────────────────────────────

/// Diretório dos certificados:
///   * portátil → `<pasta do exe>\tls\`
///   * Windows  → `%APPDATA%\FzComputerAI\tls\`
///   * outros   → `$XDG_CONFIG_HOME/fzcomputerai/tls` ou `~/.config/fzcomputerai/tls`
pub fn cert_dir(portable: bool) -> PathBuf {
    if portable {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("tls");
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("FzComputerAI").join("tls");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("fzcomputerai").join("tls");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("fzcomputerai")
            .join("tls");
    }
    PathBuf::from("tls")
}

fn write_private(path: &Path, data: &str) -> Result<()> {
    std::fs::write(path, data).with_context(|| format!("gravar {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// ───────────────────────────── inspeção ─────────────────────────────

/// O que a tela de diagnóstico mostra sobre um certificado. Tudo lido do
/// arquivo real (x509-parser) — nada presumido.
#[derive(Clone, Debug, Default)]
pub struct CertInfo {
    /// Subject DN (usado no log/diagnóstico; a tela mostra emissor + SANs).
    #[allow(dead_code)]
    pub subject: String,
    pub issuer: String,
    pub sans: Vec<String>,
    pub not_before: String,
    pub not_after: String,
    pub days_left: i64,
    pub sha256_fingerprint: String,
    pub self_signed: bool,
}

impl CertInfo {
    pub fn expired(&self) -> bool {
        self.days_left < 0
    }
    pub fn needs_renewal(&self) -> bool {
        self.days_left < RENEW_BEFORE_DAYS
    }
}

fn first_cert_der_from_pem(pem: &[u8]) -> Result<Vec<u8>> {
    let (_, doc) = x509_parser::pem::parse_x509_pem(pem)
        .map_err(|e| anyhow!("PEM inválido: {e}"))?;
    Ok(doc.contents)
}

pub fn inspect_cert_der(der: &[u8]) -> Result<CertInfo> {
    let (_, cert) = x509_parser::parse_x509_certificate(der)
        .map_err(|e| anyhow!("X.509 inválido: {e}"))?;
    let mut sans = Vec::new();
    if let Ok(Some(ext)) = cert.subject_alternative_name() {
        for name in &ext.value.general_names {
            match name {
                x509_parser::extensions::GeneralName::DNSName(d) => sans.push(d.to_string()),
                x509_parser::extensions::GeneralName::IPAddress(ip) => {
                    let s = match ip.len() {
                        4 => std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]).to_string(),
                        16 => {
                            let mut b = [0u8; 16];
                            b.copy_from_slice(ip);
                            std::net::Ipv6Addr::from(b).to_string()
                        }
                        _ => continue,
                    };
                    sans.push(s);
                }
                _ => {}
            }
        }
    }
    let now = time::OffsetDateTime::now_utc();
    let not_after = cert.validity().not_after.to_datetime();
    let not_before = cert.validity().not_before.to_datetime();
    let days_left = (not_after - now).whole_days();
    let digest = ring::digest::digest(&ring::digest::SHA256, der);
    let fp = digest
        .as_ref()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":");
    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();
    Ok(CertInfo {
        self_signed: subject == issuer,
        subject,
        issuer,
        sans,
        not_before: fmt_dt(not_before),
        not_after: fmt_dt(not_after),
        days_left,
        sha256_fingerprint: fp,
    })
}

fn fmt_dt(t: time::OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute()
    )
}

pub fn inspect_cert_file(path: &Path) -> Result<CertInfo> {
    let pem = std::fs::read(path).with_context(|| format!("ler {}", path.display()))?;
    let der = first_cert_der_from_pem(&pem)?;
    inspect_cert_der(&der)
}

// ───────────────────────────── auto-assinado ─────────────────────────────

/// Gera (ou renova) o par auto-assinado em `dir`. Devolve `(crt, key, gerado)`.
/// Não regenera um cert válido que já cubra os SANs — a cada regeneração o
/// fingerprint muda e todo cliente que fez pin precisa refazê-lo.
pub fn ensure_self_signed(dir: &Path, sans: &[String], force: bool) -> Result<(PathBuf, PathBuf, bool)> {
    std::fs::create_dir_all(dir).with_context(|| format!("criar {}", dir.display()))?;
    let crt = dir.join(SELF_SIGNED_CERT);
    let key = dir.join(SELF_SIGNED_KEY);

    if !force && crt.exists() && key.exists() {
        if let Ok(info) = inspect_cert_file(&crt) {
            let mut wanted: Vec<String> = sans.iter().map(|s| s.to_lowercase()).collect();
            wanted.sort();
            wanted.dedup();
            let have: Vec<String> = info.sans.iter().map(|s| s.to_lowercase()).collect();
            let covers = wanted.iter().all(|w| have.contains(w));
            if !info.needs_renewal() && covers {
                return Ok((crt, key, false));
            }
        }
    }

    let mut params = rcgen::CertificateParams::new(sans.to_vec())
        .map_err(|e| anyhow!("SANs inválidos: {e}"))?;
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::hours(1);
    params.not_after = now + time::Duration::days(SELF_SIGNED_DAYS);
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, "FzComputerAI MCP endpoint");
    dn.push(rcgen::DnType::OrganizationName, "FzComputerAI (self-signed, local TLS)");
    params.distinguished_name = dn;
    params.is_ca = rcgen::IsCa::ExplicitNoCa;
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::DigitalSignature,
        rcgen::KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

    let signing_key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| anyhow!("gerar chave: {e}"))?;
    let cert = params
        .self_signed(&signing_key)
        .map_err(|e| anyhow!("assinar: {e}"))?;

    // Backup do par anterior (regra do projeto: sempre recuperável).
    for (p, name) in [(&crt, "selfsigned.prev.crt"), (&key, "selfsigned.prev.key")] {
        if p.exists() {
            let _ = std::fs::copy(p, dir.join(name));
        }
    }
    write_private(&key, &signing_key.serialize_pem())?;
    std::fs::write(&crt, cert.pem()).with_context(|| format!("gravar {}", crt.display()))?;
    Ok((crt, key, true))
}

/// SANs que o auto-assinado precisa cobrir para os clientes locais e da LAN.
pub fn default_sans(lan_ip: &str, extra_domain: &str) -> Vec<String> {
    let mut v = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let lan = lan_ip.trim();
    if !lan.is_empty() && lan != "127.0.0.1" && lan.parse::<std::net::IpAddr>().is_ok() {
        v.push(lan.to_string());
    }
    if let Some(h) = hostname_lossy() {
        if !h.is_empty() && !h.contains(' ') {
            v.push(h.to_lowercase());
        }
    }
    let d = extra_domain.trim();
    if !d.is_empty() {
        v.push(d.to_lowercase());
    }
    v.sort();
    v.dedup();
    v
}

fn hostname_lossy() -> Option<String> {
    for var in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(v) = std::env::var(var) {
            if !v.trim().is_empty() {
                return Some(v.trim().to_string());
            }
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ───────────────────────────── proxy TLS ─────────────────────────────

fn load_cert_chain(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(path)
        .map_err(|e| anyhow!("abrir {}: {e}", path.display()))?
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| anyhow!("PEM de certificado inválido em {}: {e}", path.display()))?;
    if certs.is_empty() {
        bail!("nenhum certificado PEM em {}", path.display());
    }
    Ok(certs)
}

fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_file(path)
        .map_err(|e| anyhow!("chave privada inválida em {}: {e}", path.display()))
}

pub fn build_server_config(cert_path: &Path, key_path: &Path) -> Result<Arc<rustls::ServerConfig>> {
    install_crypto_provider();
    let certs = load_cert_chain(cert_path)?;
    let key = load_key(key_path)?;
    let mut cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| anyhow!("certificado e chave não combinam: {e}"))?;
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(cfg))
}

/// Handle do listener HTTPS — cai junto com o processo, como o LAN forward.
pub struct TlsProxyHandle {
    stop: Arc<AtomicBool>,
    pub bind_ip: String,
    pub port: u16,
    #[allow(dead_code)]
    pub upstream_port: u16,
    /// Conexões aceitas desde o start (diagnóstico da tela).
    pub accepted: Arc<std::sync::atomic::AtomicU64>,
    /// Último erro de conexão (texto), para o console.
    pub last_error: Arc<Mutex<String>>,
}

impl TlsProxyHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        // Destrava o accept com uma conexão dummy.
        let target = if self.bind_ip == "0.0.0.0" { "127.0.0.1" } else { self.bind_ip.as_str() };
        if let Ok(addr) = format!("{}:{}", target, self.port).parse::<std::net::SocketAddr>() {
            let _ = TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300));
        }
    }
    pub fn accepted_count(&self) -> u64 {
        self.accepted.load(Ordering::Relaxed)
    }
    pub fn take_last_error(&self) -> String {
        self.last_error
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }
}

/// Sobe o listener HTTPS em `<bind_ip>:<port>` encaminhando para
/// `127.0.0.1:<upstream_port>`. Cada conexão: handshake rustls na thread da
/// conexão, depois cópia bidirecional (um sentido por thread).
pub fn start_tls_proxy(
    bind_ip: &str,
    port: u16,
    upstream_port: u16,
    cert_path: &Path,
    key_path: &Path,
) -> Result<TlsProxyHandle> {
    let config = build_server_config(cert_path, key_path)?;
    let listener = TcpListener::bind((bind_ip, port))
        .with_context(|| format!("escutar em {}:{}", bind_ip, port))?;
    let stop = Arc::new(AtomicBool::new(false));
    let accepted = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let last_error = Arc::new(Mutex::new(String::new()));

    let stop_t = stop.clone();
    let accepted_t = accepted.clone();
    let last_error_t = last_error.clone();
    std::thread::Builder::new()
        .name("fz-tls-accept".into())
        .spawn(move || {
            for conn in listener.incoming() {
                if stop_t.load(Ordering::SeqCst) {
                    break;
                }
                let client = match conn {
                    Ok(c) => c,
                    Err(_) => break,
                };
                accepted_t.fetch_add(1, Ordering::Relaxed);
                let config = config.clone();
                let last_error = last_error_t.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_tls_conn(client, config, upstream_port) {
                        if let Ok(mut le) = last_error.lock() {
                            *le = e.to_string();
                        }
                    }
                });
            }
        })
        .context("criar thread do listener TLS")?;

    Ok(TlsProxyHandle {
        stop,
        bind_ip: bind_ip.to_string(),
        port,
        upstream_port,
        accepted,
        last_error,
    })
}

fn handle_tls_conn(client: TcpStream, config: Arc<rustls::ServerConfig>, upstream_port: u16) -> Result<()> {
    let _ = client.set_nodelay(true);
    let conn = rustls::ServerConnection::new(config).map_err(|e| anyhow!("config TLS: {e}"))?;
    let mut tls = rustls::StreamOwned::new(conn, client);
    // Handshake completo antes de abrir o upstream: conexão que morre no
    // handshake não gasta socket no motor.
    while tls.conn.is_handshaking() {
        tls.conn
            .complete_io(&mut tls.sock)
            .map_err(|e| anyhow!("handshake TLS falhou: {e}"))?;
    }
    let upstream = TcpStream::connect(("127.0.0.1", upstream_port))
        .map_err(|e| anyhow!("motor em 127.0.0.1:{upstream_port} recusou: {e}"))?;
    let _ = upstream.set_nodelay(true);
    let mut u_out = upstream.try_clone().context("clone upstream")?;
    let mut u_in = upstream;

    // Split: o socket TCP é clonável, a sessão rustls não. A thread de
    // escrita usa `conn.writer()` + `write_tls` no clone do socket; a thread
    // de leitura usa `read_tls` + `reader()`. As duas precisam do mesmo
    // `conn`, então ele fica atrás de um Mutex segurado só por operação.
    let sock_w = tls.sock.try_clone().context("clone socket cliente")?;
    let conn = Arc::new(Mutex::new(tls.conn));
    let mut sock_r = tls.sock;

    let conn_w = conn.clone();
    let writer = std::thread::spawn(move || {
        // motor -> cliente(TLS)
        let mut sock_w = sock_w;
        let mut buf = [0u8; 16 * 1024];
        loop {
            let n = match u_in.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let mut g = match conn_w.lock() {
                Ok(g) => g,
                Err(_) => break,
            };
            if g.writer().write_all(&buf[..n]).is_err() {
                break;
            }
            while g.wants_write() {
                if g.write_tls(&mut sock_w).is_err() {
                    return;
                }
            }
        }
        if let Ok(mut g) = conn_w.lock() {
            g.send_close_notify();
            while g.wants_write() {
                if g.write_tls(&mut sock_w).is_err() {
                    break;
                }
            }
        }
        let _ = sock_w.shutdown(std::net::Shutdown::Write);
    });

    // cliente(TLS) -> motor
    let mut plain = vec![0u8; 16 * 1024];
    // Em TLS 1.3 o cliente pode mandar a requisicao no MESMO segmento do seu
    // Finished: o complete_io do handshake ja a leu e decifrou. Se so fossemos
    // olhar o plaintext depois do PROXIMO read() do socket, essa requisicao
    // ficaria presa (o cliente espera resposta, nos esperamos bytes) — sob
    // carga era exatamente o que acontecia. Drena antes de entrar no loop.
    if let Ok(mut g) = conn.lock() {
        if drain_plaintext(&mut g, &mut u_out, &mut plain) {
            let _ = u_out.shutdown(std::net::Shutdown::Write);
            let _ = writer.join();
            let _ = sock_r.shutdown(std::net::Shutdown::Both);
            return Ok(());
        }
    }
    loop {
        // Lê bytes TLS do socket FORA do lock (bloqueante), processa DENTRO.
        let mut raw = [0u8; 16 * 1024];
        let n = match sock_r.read(&mut raw) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut done = false;
        {
            let mut g = match conn.lock() {
                Ok(g) => g,
                Err(_) => break,
            };
            let mut cursor = &raw[..n];
            // Contrato do rustls: process_new_packets + drenar o plaintext
            // DEPOIS DE CADA read_tls. Encher o deframer com varios read_tls
            // seguidos faz o buffer lotar, read_tls devolver Ok(0) e o rustls
            // marcar EOF falso — a conexao morria no meio de corpos grandes.
            while !cursor.is_empty() && !done {
                match g.read_tls(&mut cursor) {
                    Ok(0) | Err(_) => {
                        done = true;
                        break;
                    }
                    Ok(_) => {}
                }
                if g.process_new_packets().is_err() {
                    done = true;
                    break;
                }
                if drain_plaintext(&mut g, &mut u_out, &mut plain) {
                    done = true;
                }
            }
            // Respostas do handshake/alertas geradas pelo processamento.
            while g.wants_write() {
                let mut s = match sock_r.try_clone() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                if g.write_tls(&mut s).is_err() {
                    break;
                }
            }
        }
        if done {
            break;
        }
    }
    let _ = u_out.shutdown(std::net::Shutdown::Write);
    let _ = writer.join();
    let _ = sock_r.shutdown(std::net::Shutdown::Both);
    Ok(())
}

/// Copia todo o plaintext ja decifrado para o motor. Devolve `true` quando a
/// conexao acabou (close_notify do cliente, EOF ou erro no upstream).
fn drain_plaintext(
    g: &mut rustls::ServerConnection,
    u_out: &mut TcpStream,
    plain: &mut [u8],
) -> bool {
    loop {
        match g.reader().read(plain) {
            Ok(0) => return true, // close_notify do cliente
            Ok(k) => {
                if u_out.write_all(&plain[..k]).is_err() {
                    return true;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return false,
            Err(_) => return true,
        }
    }
}

// ───────────────────────────── sonda HTTPS ─────────────────────────────

/// Verificador que aceita QUALQUER certificado e o captura para a tela.
/// Uso EXCLUSIVO da sonda de diagnóstico local (o app conferindo a si mesmo);
/// nunca use isto para falar com terceiros.
#[derive(Debug)]
struct CaptureVerifier {
    seen: Arc<Mutex<Option<Vec<u8>>>>,
}

impl rustls::client::danger::ServerCertVerifier for CaptureVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if let Ok(mut s) = self.seen.lock() {
            *s = Some(end_entity.as_ref().to_vec());
        }
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[derive(Clone, Debug, Default)]
pub struct HttpsProbe {
    pub tls_ok: bool,
    pub protocol: String,
    pub http_status: u16,
    pub jsonrpc: bool,
    pub cert: Option<CertInfo>,
    pub detail: String,
}

/// Sonda REAL: handshake TLS em `ip:port` (SNI = `sni`), depois POST /mcp com
/// um `initialize` JSON-RPC com o bearer token. `jsonrpc=true` só quando a
/// resposta contém "jsonrpc" — mesmo critério do probe HTTP da GUI.
pub fn probe_https(ip: &str, port: u16, sni: &str, token: &str) -> HttpsProbe {
    install_crypto_provider();
    let mut out = HttpsProbe::default();
    let seen = Arc::new(Mutex::new(None));
    let verifier = Arc::new(CaptureVerifier { seen: seen.clone() });
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    let server_name = match ServerName::try_from(sni.to_string()) {
        Ok(n) => n,
        Err(e) => {
            out.detail = format!("SNI inválido '{sni}': {e}");
            return out;
        }
    };
    let addr: std::net::SocketAddr = match format!("{ip}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            out.detail = format!("endereço inválido: {e}");
            return out;
        }
    };
    let sock = match TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3)) {
        Ok(s) => s,
        Err(e) => {
            out.detail = format!("TCP {addr}: {e}");
            return out;
        }
    };
    let _ = sock.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = sock.set_write_timeout(Some(std::time::Duration::from_secs(5)));
    let conn = match rustls::ClientConnection::new(Arc::new(config), server_name) {
        Ok(c) => c,
        Err(e) => {
            out.detail = format!("config cliente TLS: {e}");
            return out;
        }
    };
    let mut tls = rustls::StreamOwned::new(conn, sock);
    while tls.conn.is_handshaking() {
        if let Err(e) = tls.conn.complete_io(&mut tls.sock) {
            out.detail = format!("handshake TLS: {e}");
            return out;
        }
    }
    out.tls_ok = true;
    out.protocol = match tls.conn.protocol_version() {
        Some(v) => format!("{v:?}"),
        None => "?".into(),
    };
    if let Ok(s) = seen.lock() {
        if let Some(der) = s.as_ref() {
            out.cert = inspect_cert_der(der).ok();
        }
    }
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"fzcomputerai-https-probe","version":"1"}}}"#;
    let auth = if token.is_empty() {
        String::new()
    } else {
        format!("Authorization: Bearer {token}\r\n")
    };
    let req = format!(
        "POST /mcp HTTP/1.1\r\nHost: {sni}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    if let Err(e) = tls.write_all(req.as_bytes()) {
        out.detail = format!("escrever POST: {e}");
        return out;
    }
    let mut resp = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match tls.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                resp.extend_from_slice(&buf[..n]);
                if resp.len() > 256 * 1024 || String::from_utf8_lossy(&resp).contains("jsonrpc") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&resp);
    if let Some(line) = text.lines().next() {
        let mut it = line.split_whitespace();
        it.next();
        out.http_status = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    }
    out.jsonrpc = text.contains("jsonrpc");
    out.detail = text.lines().next().unwrap_or("").to_string();
    out
}

// ───────────────────────────── ACME / Let's Encrypt ─────────────────────────────

/// Progresso e resultado da emissão, entregues à thread da GUI via canal.
#[derive(Debug)]
pub enum AcmeEvent {
    Log(String),
    Done(Result<(PathBuf, PathBuf)>),
}

pub struct AcmeRequest {
    pub domain: String,
    pub email: String,
    pub staging: bool,
    pub dir: PathBuf,
}

/// Dispara a emissão Let's Encrypt em uma thread com runtime tokio próprio.
/// O desafio HTTP-01 é respondido por um listener temporário em `0.0.0.0:80`
/// (RFC 8555 §8.3 — a porta é fixa; a CA só consulta 80). Pré-requisitos que
/// NÃO dá para automatizar deste lado: DNS do domínio apontando para o IP
/// público desta máquina e a 80 aberta/encaminhada no roteador e firewall.
pub fn acme_issue_async(req: AcmeRequest) -> std::sync::mpsc::Receiver<AcmeEvent> {
    let (tx, rx) = std::sync::mpsc::channel::<AcmeEvent>();
    std::thread::Builder::new()
        .name("fz-acme".into())
        .spawn(move || {
            install_crypto_provider();
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(AcmeEvent::Done(Err(anyhow!("runtime tokio: {e}"))));
                    return;
                }
            };
            let res = rt.block_on(acme_issue(&req, &tx));
            let _ = tx.send(AcmeEvent::Done(res));
        })
        .ok();
    rx
}

type ChallengeMap = Arc<Mutex<std::collections::HashMap<String, String>>>;

/// Responde `GET /.well-known/acme-challenge/<token>` com o key authorization.
/// Qualquer outro caminho → 404. Cai quando `stop` vira true.
fn start_http01_responder(map: ChallengeMap, stop: Arc<AtomicBool>) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", ACME_HTTP01_PORT)).with_context(|| {
        format!(
            "escutar em 0.0.0.0:{} para o desafio HTTP-01 (porta ocupada por IIS/Apache/outro servidor web? no Linux exige CAP_NET_BIND_SERVICE)",
            ACME_HTTP01_PORT
        )
    })?;
    listener.set_nonblocking(true)?;
    std::thread::Builder::new()
        .name("fz-acme-http01".into())
        .spawn(move || loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            match listener.accept() {
                Ok((mut s, _)) => {
                    let _ = s.set_nonblocking(false);
                    let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    let mut buf = [0u8; 4096];
                    let n = s.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let path = req
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("");
                    const PFX: &str = "/.well-known/acme-challenge/";
                    let body = path
                        .strip_prefix(PFX)
                        .and_then(|tok| map.lock().ok().and_then(|m| m.get(tok).cloned()));
                    let resp = match body {
                        Some(ka) => format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            ka.len(),
                            ka
                        ),
                        None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
                    };
                    let _ = s.write_all(resp.as_bytes());
                    let _ = s.shutdown(std::net::Shutdown::Both);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(_) => break,
            }
        })?;
    Ok(())
}

async fn acme_issue(req: &AcmeRequest, tx: &std::sync::mpsc::Sender<AcmeEvent>) -> Result<(PathBuf, PathBuf)> {
    use instant_acme::{
        Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt,
        NewAccount, NewOrder, OrderStatus, RetryPolicy,
    };
    let log = |m: String| {
        let _ = tx.send(AcmeEvent::Log(m));
    };
    let domain = req.domain.trim().to_lowercase();
    if domain.is_empty() || domain.contains('/') || domain.contains(' ') {
        bail!("domínio inválido: '{}'", req.domain);
    }
    if domain.parse::<std::net::IpAddr>().is_ok() {
        bail!("Let's Encrypt não emite certificado para endereço IP — informe um nome DNS público.");
    }
    std::fs::create_dir_all(&req.dir)?;
    let directory = if req.staging {
        LetsEncrypt::Staging.url().to_owned()
    } else {
        LetsEncrypt::Production.url().to_owned()
    };
    log(format!(
        "[acme] Diretório ACME: {} ({})",
        directory,
        if req.staging { "STAGING — cert de teste, NÃO confiável" } else { "produção" }
    ));

    // Conta: reaproveita a credencial salva (os limites de rate do LE contam
    // conta nova também).
    let acct_path = req.dir.join(ACME_ACCOUNT);
    let account = match std::fs::read_to_string(&acct_path)
        .ok()
        .and_then(|j| serde_json::from_str::<AccountCredentials>(&j).ok())
    {
        Some(creds) => {
            log("[acme] Conta ACME existente carregada.".into());
            Account::builder()?.from_credentials(creds).await?
        }
        None => {
            let mut contact: Vec<String> = Vec::new();
            let email = req.email.trim();
            if !email.is_empty() {
                contact.push(format!("mailto:{email}"));
            }
            let contact_refs: Vec<&str> = contact.iter().map(|s| s.as_str()).collect();
            let (account, creds) = Account::builder()?
                .create(
                    &NewAccount {
                        contact: &contact_refs,
                        terms_of_service_agreed: true,
                        only_return_existing: false,
                    },
                    directory.clone(),
                    None,
                )
                .await?;
            write_private(&acct_path, &serde_json::to_string(&creds)?)?;
            log(format!("[acme] Conta ACME criada e salva em {}.", acct_path.display()));
            account
        }
    };

    let idents = [Identifier::Dns(domain.clone())];
    let mut order = account.new_order(&NewOrder::new(&idents)).await?;
    log(format!("[acme] Ordem criada: {:?}", order.state().status));

    let map: ChallengeMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let mut responder_up = false;

    let mut authorizations = order.authorizations();
    while let Some(result) = authorizations.next().await {
        let mut authz = result?;
        match authz.status {
            AuthorizationStatus::Pending => {}
            AuthorizationStatus::Valid => continue,
            other => bail!("autorização em estado inesperado: {other:?}"),
        }
        let mut challenge = authz
            .challenge(ChallengeType::Http01)
            .ok_or_else(|| anyhow!("a CA não ofereceu desafio HTTP-01"))?;
        let key_auth = challenge.key_authorization();
        let token = challenge.token.clone();
        if let Ok(mut m) = map.lock() {
            m.insert(token.clone(), key_auth.as_str().to_string());
        }
        if !responder_up {
            start_http01_responder(map.clone(), stop.clone())?;
            responder_up = true;
            log(format!(
                "[acme] Respondedor HTTP-01 de pé em 0.0.0.0:{} — a CA vai buscar http://{}/.well-known/acme-challenge/{}",
                ACME_HTTP01_PORT, domain, token
            ));
        }
        challenge.set_ready().await?;
        log("[acme] Desafio marcado como pronto; aguardando validação da CA...".into());
    }

    let status = order.poll_ready(&RetryPolicy::default()).await;
    stop.store(true, Ordering::SeqCst);
    let status = status?;
    if status != OrderStatus::Ready {
        bail!(
            "validação falhou — status da ordem: {status:?}. Causas comuns: o DNS de {domain} não aponta para o IP público desta máquina, ou a porta 80 não está encaminhada/aberta até aqui."
        );
    }
    log("[acme] Ordem validada. Finalizando (CSR gerado pela biblioteca)...".into());
    let private_key_pem = order.finalize().await?;
    let cert_chain_pem = order.poll_certificate(&RetryPolicy::default()).await?;

    let crt = req.dir.join(ACME_CERT);
    let key = req.dir.join(ACME_KEY);
    // Backup do par anterior antes de sobrescrever (regra: sempre recuperável).
    for (p, name) in [(&crt, "letsencrypt.prev.crt"), (&key, "letsencrypt.prev.key")] {
        if p.exists() {
            let _ = std::fs::copy(p, req.dir.join(name));
        }
    }
    write_private(&key, &private_key_pem)?;
    std::fs::write(&crt, &cert_chain_pem)?;
    let info = inspect_cert_file(&crt)?;
    log(format!(
        "[acme] Certificado emitido para {} — válido até {} ({} dias). Emissor: {}",
        domain, info.not_after, info.days_left, info.issuer
    ));
    Ok((crt, key))
}

// ───────────────────────────── --tls-init (instalador) ─────────────────────────────

/// Caminho de linha de comando usado pelo instalador (`fzcomputerai --tls-init`)
/// e disponível para scripts: gera o auto-assinado se ainda não existir e
/// registra o resultado em `<dir>/tls-init.log` (o binário é
/// `windows_subsystem = "windows"`, sem console para imprimir).
/// Exit code: 0 = ok, 1 = erro.
pub fn cli_tls_init(portable: bool, lan_ip: &str) -> Result<PathBuf> {
    install_crypto_provider();
    let dir = cert_dir(portable);
    let sans = default_sans(lan_ip, "");
    let result = ensure_self_signed(&dir, &sans, false);
    let mut log = String::new();
    log.push_str(&format!(
        "[{}] fzcomputerai {} --tls-init\n",
        fmt_dt(time::OffsetDateTime::now_utc()),
        env!("CARGO_PKG_VERSION")
    ));
    match &result {
        Ok((crt, key, generated)) => {
            log.push_str(&format!(
                "{}: {}\nchave: {}\n",
                if *generated { "GERADO" } else { "JA EXISTIA (valido, mantido)" },
                crt.display(),
                key.display()
            ));
            if let Ok(info) = inspect_cert_file(crt) {
                log.push_str(&format!(
                    "SANs: {}\nvalido ate: {} ({} dias)\nSHA-256: {}\n",
                    info.sans.join(", "),
                    info.not_after,
                    info.days_left,
                    info.sha256_fingerprint
                ));
            }
        }
        Err(e) => log.push_str(&format!("ERRO: {e:#}\n")),
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(INIT_LOG), &log);
    result.map(|(crt, _, _)| crt)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upstream falso que imita o motor: responde qualquer POST com JSON-RPC.
    fn fake_engine() -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for c in l.incoming() {
                let Ok(mut s) = c else { break };
                std::thread::spawn(move || {
                    let mut buf = vec![0u8; 8192];
                    let mut got = Vec::new();
                    loop {
                        let n = s.read(&mut buf).unwrap_or(0);
                        if n == 0 { break; }
                        got.extend_from_slice(&buf[..n]);
                        if let Some(p) = find(&got, b"\r\n\r\n") {
                            let head = String::from_utf8_lossy(&got[..p]).to_string();
                            let cl: usize = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0))).unwrap_or(0);
                            if got.len() >= p + 4 + cl { break; }
                        }
                    }
                    let body = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
                    let auth_ok = String::from_utf8_lossy(&got).contains("Bearer segredo");
                    let resp = if auth_ok {
                        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                    } else {
                        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                    };
                    let _ = s.write_all(resp.as_bytes());
                    let _ = s.shutdown(std::net::Shutdown::Both);
                });
            }
        });
        port
    }
    fn find(h: &[u8], n: &[u8]) -> Option<usize> { h.windows(n.len()).position(|w| w == n) }

    #[test]
    fn self_signed_proxy_end_to_end() {
        install_crypto_provider();
        let dir = std::env::temp_dir().join(format!("fz-tls-test-{}", std::process::id()));
        let sans = default_sans("192.168.0.101", "");
        let (crt, key, generated) = ensure_self_signed(&dir, &sans, false).unwrap();
        assert!(generated);
        let info = inspect_cert_file(&crt).unwrap();
        assert!(info.self_signed);
        assert!(info.sans.contains(&"localhost".to_string()));
        assert!(info.sans.contains(&"192.168.0.101".to_string()));
        assert!(info.days_left > SELF_SIGNED_DAYS - 2);
        // idempotente: nao regenera
        let (_, _, again) = ensure_self_signed(&dir, &sans, false).unwrap();
        assert!(!again);
        // SAN novo => regenera
        let (_, _, again) = ensure_self_signed(&dir, &default_sans("10.0.0.5", "meu.dominio.com"), false).unwrap();
        assert!(again);

        let upstream = fake_engine();
        // porta livre para o proxy
        let free = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let h = start_tls_proxy("127.0.0.1", free, upstream, &crt, &key).unwrap();

        let p = probe_https("127.0.0.1", free, "localhost", "segredo");
        assert!(p.tls_ok, "{}", p.detail);
        assert_eq!(p.http_status, 200, "{}", p.detail);
        assert!(p.jsonrpc);
        let c = p.cert.unwrap();
        assert_eq!(c.sha256_fingerprint, inspect_cert_file(&crt).unwrap().sha256_fingerprint);

        let p2 = probe_https("127.0.0.1", free, "localhost", "errado");
        assert!(p2.tls_ok);
        assert_eq!(p2.http_status, 401);
        assert!(!p2.jsonrpc);
        assert_eq!(h.accepted_count(), 2);

        h.stop();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let p3 = probe_https("127.0.0.1", free, "localhost", "segredo");
        assert!(!p3.tls_ok);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Corpo grande (1 MiB) nos dois sentidos: o proxy copia por chunks e
    /// nao pode truncar nem travar.
    #[test]
    fn proxy_large_body_both_directions() {
        install_crypto_provider();
        let dir = std::env::temp_dir().join(format!("fz-tls-big-{}", std::process::id()));
        let (crt, key, _) = ensure_self_signed(&dir, &["localhost".to_string()], true).unwrap();
        // upstream eco: devolve exatamente o que recebeu (apos cabecalho fixo)
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let up = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut b = [0u8; 65536];
            while got.len() < 1 << 20 {
                let n = s.read(&mut b).unwrap();
                if n == 0 { break; }
                got.extend_from_slice(&b[..n]);
            }
            s.write_all(&got).unwrap();
            let _ = s.shutdown(std::net::Shutdown::Both);
        });
        let free = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let h = start_tls_proxy("127.0.0.1", free, up, &crt, &key).unwrap();

        let seen = Arc::new(Mutex::new(None));
        let cfg = rustls::ClientConfig::builder().dangerous()
            .with_custom_certificate_verifier(Arc::new(CaptureVerifier { seen })).with_no_client_auth();
        let sock = TcpStream::connect(("127.0.0.1", free)).unwrap();
        let conn = rustls::ClientConnection::new(Arc::new(cfg), ServerName::try_from("localhost").unwrap()).unwrap();
        let mut tls = rustls::StreamOwned::new(conn, sock);
        let payload: Vec<u8> = (0..(1u32 << 20)).map(|i| (i % 251) as u8).collect();
        tls.write_all(&payload).unwrap();
        tls.flush().unwrap();
        let mut back = Vec::new();
        let mut b = [0u8; 65536];
        while back.len() < payload.len() {
            match tls.read(&mut b) { Ok(0) => break, Ok(n) => back.extend_from_slice(&b[..n]), Err(e) => panic!("{e}") }
        }
        assert_eq!(back.len(), payload.len());
        assert!(back == payload);
        h.stop();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
