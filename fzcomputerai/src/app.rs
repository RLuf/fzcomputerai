use eframe::egui::{self, Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(PartialEq, Clone, Copy)]
pub enum Language {
    PtBr,
    English,
}

/// Status real do endpoint MCP, distinguindo loopback de LAN.
/// Verde (LanListening) SOMENTE se o netstat mostrar listener no IP da LAN
/// (ou 0.0.0.0) E o teste TCP no IP da LAN conectar.
#[derive(PartialEq, Clone, Copy)]
pub enum PortStatus {
    Stopped,
    LocalOnly,
    LanListening,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Network,
    Calibration,
    Windows,
    Recording,
    DoctorSkills,
    McpTools,
    Tunnel,
}

/// Provedor do túnel de internet (HTTPS público -> MCP HTTP local).
#[derive(PartialEq, Clone, Copy)]
pub enum TunnelProvider {
    Cloudflare,
    Ngrok,
    Ssh,
}

/// Status HONESTO do túnel, em níveis (mesma doutrina do PortStatus):
///   Starting = processo vivo, URL pública ainda NAO capturada;
///   Running  = URL pública capturada (ou informada). "Confirmado pela
///              internet" é um estado SEPARADO (tunnel_exposure), provado
///              por POST initialize real na URL pública — nunca presumido.
#[derive(PartialEq, Clone, Copy)]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

/// Resultado da SONDA DE EXPOSIÇÃO na URL pública (POST initialize SEM
/// credencial): o que a internet consegue de fato. Nunca é opinião — é o
/// que a rede respondeu.
///   Exposed     = o MCP respondeu JSON-RPC sem nenhuma credencial (aberto);
///   EdgeAuth(c) = a borda barrou (HTTP 401/403/302) — há auth na frente;
///   Unknown     = timeout/5xx/erro: não deu para verificar (tratar como exposto).
#[derive(PartialEq, Clone, Copy)]
pub enum TunnelExposure {
    Exposed,
    EdgeAuth(u16),
    Unknown,
}

/// Origem do certificado do endpoint HTTPS (ver fzcomputerai/src/tls.rs).
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TlsMode {
    /// Gerado pelo app (rcgen) na instalação ou no primeiro run — padrão.
    SelfSigned,
    /// Emitido por Let's Encrypt (ACME HTTP-01) para um domínio público.
    LetsEncrypt,
    /// Arquivos PEM (.crt/.key) informados pelo usuário.
    Custom,
}

/// Em que endereço o listener HTTPS escuta.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TlsBind {
    Loopback,
    Lan,
    All,
}

/// Status HONESTO do HTTPS (mesma doutrina do PortStatus): só fica verde
/// depois de handshake TLS real + POST initialize com resposta JSON-RPC.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TlsStatus {
    Stopped,
    /// Listener de pé, mas a sonda não obteve JSON-RPC (motor parado? token?).
    ListeningNoMcp,
    Listening,
    Error,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WindowItem {
    pub pid: u32,
    pub window_id: u64,
    pub title: String,
    pub app_name: Option<String>,
    pub minimized: Option<bool>,
}

/// Ponto de status colorido DESENHADO (painter), em vez do caractere "●":
/// a fonte proporcional padrão do egui não tem esse glifo e renderiza uma
/// caixa vazia — que parece placeholder quebrado na tela.
pub fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, color);
}

/// Cria um `Command` que, no Windows, NUNCA abre a janela preta de console
/// (CREATE_NO_WINDOW). Use SEMPRE este helper em vez de `Command::new`.
pub fn quiet_cmd(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Autodetecta o IP da LAN sem enviar nenhum pacote:
/// UDP connect apenas seleciona a interface de saída.
pub fn detect_lan_ip() -> String {
    let detected = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|sock| {
        sock.connect("8.8.8.8:80")?;
        sock.local_addr()
    });
    match detected {
        Ok(addr) => addr.ip().to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}

pub struct AppState {
    pub language: Language,
    pub active_tab: Tab,
    pub http_port: String,
    pub lan_ip: String,
    pub port_active: bool,
    pub port_status: PortStatus,
    pub daemon_running: bool,

    // Regra portproxy LAN -> localhost: estado REAL em DOIS niveis.
    //   portproxy_active    = a regra EXISTE na config (netsh show v4tov4);
    //   portproxy_effective = alem de existir, o LISTENER esta de pe no
    //                         netstat (IP Helper servindo de fato).
    // Regra na config sem listener e "SEM EFEITO" — nunca pinte verde.
    pub portproxy_active: bool,
    pub portproxy_effective: bool,

    // VERDADE na tela: linhas LISTENING reais do netstat (porta configurada
    // + qualquer porta no IP da LAN) e TODAS as regras portproxy v4tov4.
    // A UI exibe isto cru — nada de host/URL "de intencao".
    pub real_listeners: Vec<String>,
    pub portproxy_rules: Vec<String>,

    // Calibração & Visão
    pub screen_width: u32,
    pub screen_height: u32,
    pub dpi_scale: f32,
    pub test_x: String,
    pub test_y: String,

    // Janelas & Processos
    pub windows_list: Vec<WindowItem>,
    pub launch_input: String,

    // Gravação & Trajetórias
    pub is_recording: bool,
    pub recording_path: String,

    pub show_about: bool,

    // ─── SAÍDA UNIFICADA ────────────────────────────────────────────────
    // Antes cada aba tinha sua PRÓPRIA caixa de saída (calibration_log,
    // windows_log, recording_log, doctor_output, skills_output,
    // mcp_tools_output, tunnel_output) e a aba MCP & Rede tinha o Console
    // Debug — dois consoles na mesma tela, com a mesma informação. Agora:
    //   status_msg = ÚLTIMA mensagem/resultado (faixa de status);
    //   debug_log  = HISTÓRICO completo (comandos + exit + stdout/stderr),
    //                exibido no ÚNICO console global, visível em todas as
    //                abas, rolando como `tail -f`.
    pub status_msg: String,
    pub debug_log: String,
    // Segue o fim do log automaticamente; vira false quando o usuário rola
    // para cima (para poder ler) e volta a true quando ele retorna ao fim.
    pub console_follow: bool,
    // Salto explícito para o fim ("Ir ao fim"). Tem precedência sobre a
    // detecção por posição no frame do clique — sem isto o botão era anulado
    // pela posição antiga do scroll lida logo abaixo, no mesmo frame.
    pub console_jump: bool,

    // ─── tail -f do log REAL do motor ───
    // O daemon `cua-driver serve` roda destacado: sem redirecionar, o stdout
    // dele se perde e o console da GUI só mostra os comandos que a PRÓPRIA
    // GUI executa. Resultado: um cliente MCP externo (conector do Claude,
    // Antigravity…) conversando com o motor não aparecia em lugar nenhum.
    // Agora o daemon escreve num arquivo e a GUI o segue como `tail -f`.
    pub engine_log_pos: u64,
    pub engine_log_poll: Option<std::time::Instant>,

    // ─── Encaminhamento LAN feito PELO APP (substitui netsh portproxy) ───
    // Thread de forward TCP: escuta em <ip_lan>:porta e copia para
    // 127.0.0.1:porta. É filho do processo — morre junto com o app, não pede
    // admin e não deixa regra no sistema.
    pub lan_forward_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub lan_forward_addr: Option<(String, u16)>,

    // Iniciar com o Windows (HKCU\...\Run)
    pub autostart_enabled: bool,

    // ─── MODO PORTÁTIL ─────────────────────────────────────────────────
    // Ligado quando existe o arquivo-marcador `fzcomputerai.portable` ao lado
    // do executável (é o que vai dentro do .zip portátil). Nesse modo as
    // PREFERÊNCIAS vão para um .ini ao lado do exe em vez do registro, e o
    // "Iniciar com o Windows" fica indisponível (ele exige HKCU\...\Run, que
    // é justamente o rastro que o portátil não deve deixar).
    //
    // LIMITE HONESTO: o RASTREIO de limpeza (regras portproxy e PIDs de túnel)
    // continua no registro mesmo em modo portátil. Ele não é preferência do
    // usuário: é a trava que evita deixar a máquina exposta se o app morrer.
    // Trocá-lo por arquivo quebraria o watchdog em PowerShell que o lê. A UI
    // diz isso em vez de fingir "zero registro".
    pub portable_mode: bool,

    // ─── Bandeja do sistema (tray) ──────────────────────────────────────
    // Quando ligado, minimizar ESCONDE a janela em vez de deixá-la na barra
    // de tarefas — o app continua vivo (e o motor também), acessível pelo
    // ícone na área de notificação. Preferência persistida em HKCU.
    pub minimize_to_tray: bool,
    pub window_hidden: bool,

    // MCP Tools Catalog
    pub mcp_tools_filter: String,

    // Fluxo de upgrade (GitHub Releases):
    //   check -> update_available(tag) -> download em BACKGROUND (%TEMP%)
    //   -> ready.flag -> pedir para FECHAR -> instalar -> reabrir GUI + motor.
    pub update_available: Option<String>,
    pub update_downloading: bool,
    pub update_ready: bool,
    last_update_poll: Option<std::time::Instant>,

    // ─── Atualização do MOTOR cua-driver ────────────────────────────────
    // O botão "Verificar Atualizações" cuida de DUAS coisas: esta GUI e o
    // motor. Antes ele só olhava a GUI, e o motor podia ficar dezenas de
    // versões atrás sem ninguém notar (na prática ficou: 0.8.3 instalado
    // contra 0.17.0 publicado). A checagem do motor usa a API OFICIAL dele,
    // `cua-driver check-update --json`, e a aplicação usa `update --apply` —
    // nunca baixamos binário do motor por conta própria.
    pub driver_version: String,           // versão instalada (check-update)
    pub driver_latest: String,            // última publicada
    pub driver_update_available: bool,
    pub driver_notes_url: String,
    pub driver_updating: bool,            // update --apply em andamento
    pub driver_present: bool,             // cua-driver existe no PATH?
    pub update_checked: bool,             // já rodou uma verificação nesta sessão
    driver_update_poll: Option<std::time::Instant>,

    // ─── Token do endpoint HTTP do motor ────────────────────────────────
    // Versões do cua-driver a partir da série 0.16 EXIGEM
    // `CUA_DRIVER_RS_MCP_HTTP_TOKEN` (32-4096 chars) e respondem 401 a
    // qualquer POST sem `Authorization: Bearer <token>`. Sem isto, o teste de
    // status desta GUI receberia 401 e reportaria "MCP PARADO" com o motor
    // funcionando perfeitamente — ou seja, o "status honesto" mentiria. Lemos
    // o token de HKCU\Environment e o enviamos em todo probe.
    pub mcp_token: String,

    // ─── Aba Túnel: expõe o MCP HTTP local (127.0.0.1:porta) na internet ───
    // Um túnel por vez: 1 processo rastreado, 1 URL pública, 1 snippet.
    pub tunnel_provider: TunnelProvider,
    pub tunnel_status: TunnelStatus,
    pub tunnel_pid: Option<u32>,
    // URL pública BASE (sem /mcp e sem /s/<senha>); editável — o túnel
    // nomeado do Cloudflare não imprime URL, o usuário informa à mão.
    pub tunnel_public_url: String,
    pub tunnel_exposure: Option<TunnelExposure>,

    // Binários resolvidos (where.exe / {app}\tunnel / System32\OpenSSH).
    pub tunnel_cf_bin: String,
    pub tunnel_ngrok_bin: String,
    pub tunnel_ssh_bin: String,
    pub tunnel_bins_checked: bool,

    // Config por provedor (persistida em HKCU como tunnelcfg:*).
    pub tunnel_cf_token_input: String, // campo .password(true); nunca persistido/logado
    pub tunnel_cf_token_file: String,  // caminho do token-file; NAO-vazio => túnel nomeado
    pub tunnel_ngrok_use_policy: bool,
    pub tunnel_ngrok_password: String, // basic-auth da traffic policy (mostrada 1x)
    pub tunnel_ngrok_extra: String,
    pub tunnel_ssh_target: String,
    pub tunnel_ssh_remote_port: String,
    pub tunnel_ssh_key: String,
    pub tunnel_ssh_extra: String,

    // Nível 1 de auth: senha na URL (via gate local). Por sessão de túnel,
    // NUNCA persistida nem logada — aparece só na URL copiável.
    pub tunnel_gate_password: String,
    pub tunnel_gate_port: Option<u16>,

    // Modais.
    pub tunnel_show_start_modal: bool,
    pub tunnel_show_ngrok_tos: bool,

    // Download de binário em background (mesmo padrão do upgrade).
    pub tunnel_downloading: bool,

    // Identidade forte do processo (marcador na cmdline via path de log).
    pub tunnel_run_id: String,

    // ─── HTTPS do endpoint MCP (terminação TLS no próprio app) ──────────
    // O motor só fala HTTP em 127.0.0.1. Este listener (tls.rs) escuta em
    // <bind>:tls_port, termina o TLS e copia bytes para 127.0.0.1:http_port —
    // mesmo desenho do LAN forward: thread do processo, sem admin, some ao
    // fechar. Preferências persistidas como tlscfg:* (registro/ini portátil).
    pub tls_enabled: bool,
    pub tls_port: String,
    pub tls_bind: TlsBind,
    pub tls_mode: TlsMode,
    pub tls_domain: String,
    pub tls_email: String,
    pub tls_staging: bool,
    pub tls_custom_cert: String,
    pub tls_custom_key: String,
    pub tls_status: TlsStatus,
    pub tls_cert_dir: std::path::PathBuf,
    pub tls_cert_path: String,
    pub tls_cert_info: Option<crate::tls::CertInfo>,
    pub tls_probe: Option<crate::tls::HttpsProbe>,
    pub tls_probe_lan_ok: bool,
    pub tls_last_error: String,
    pub tls_acme_busy: bool,
    pub tls_accepted: u64,
    tls_proxy: Option<crate::tls::TlsProxyHandle>,
    tls_acme_rx: Option<std::sync::mpsc::Receiver<crate::tls::AcmeEvent>>,
    tls_renew_check: Option<std::time::Instant>,

    // Estado interno (privado — só o app.rs mexe).
    tunnel_child: Option<std::process::Child>,
    tunnel_gate_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    tunnel_last_poll: Option<std::time::Instant>,
    tunnel_last_probe: Option<std::time::Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            language: Language::PtBr,
            active_tab: Tab::Network,
            http_port: "8000".to_string(),
            lan_ip: detect_lan_ip(),
            port_active: false,
            port_status: PortStatus::Stopped,
            daemon_running: false,
            portproxy_active: false,
            portproxy_effective: false,
            real_listeners: Vec::new(),
            portproxy_rules: Vec::new(),

            screen_width: 1920,
            screen_height: 1080,
            dpi_scale: 1.0,
            test_x: "960".to_string(),
            test_y: "540".to_string(),

            windows_list: Vec::new(),
            launch_input: "notepad".to_string(),

            is_recording: false,
            recording_path: "./recordings".to_string(),

            show_about: false,

            status_msg: String::new(),
            debug_log: String::new(),
            console_follow: true,
            console_jump: false,
            engine_log_pos: 0,
            engine_log_poll: None,
            lan_forward_stop: None,
            lan_forward_addr: None,
            autostart_enabled: false,
            portable_mode: false, // definido de verdade no startup
            minimize_to_tray: false,
            window_hidden: false,

            mcp_tools_filter: String::new(),

            update_available: None,
            update_downloading: false,
            update_ready: false,
            last_update_poll: None,

            driver_version: String::new(),
            driver_latest: String::new(),
            driver_update_available: false,
            driver_notes_url: String::new(),
            driver_updating: false,
            driver_present: true, // definido de verdade no startup logo abaixo
            update_checked: false,
            driver_update_poll: None,
            mcp_token: String::new(),

            tunnel_provider: TunnelProvider::Cloudflare,
            tunnel_status: TunnelStatus::Stopped,
            tunnel_pid: None,
            tunnel_public_url: String::new(),
            tunnel_exposure: None,

            tunnel_cf_bin: String::new(),
            tunnel_ngrok_bin: String::new(),
            tunnel_ssh_bin: String::new(),
            tunnel_bins_checked: false,

            tunnel_cf_token_input: String::new(),
            tunnel_cf_token_file: String::new(),
            tunnel_ngrok_use_policy: false,
            tunnel_ngrok_password: String::new(),
            tunnel_ngrok_extra: String::new(),
            tunnel_ssh_target: "nokey@localhost.run".to_string(),
            tunnel_ssh_remote_port: "80".to_string(),
            tunnel_ssh_key: String::new(),
            tunnel_ssh_extra: String::new(),

            tunnel_gate_password: String::new(),
            tunnel_gate_port: None,

            tunnel_show_start_modal: false,
            tunnel_show_ngrok_tos: false,

            tunnel_downloading: false,
            tunnel_run_id: String::new(),

            tls_enabled: false,
            tls_port: crate::tls::DEFAULT_TLS_PORT.to_string(),
            tls_bind: TlsBind::Lan,
            tls_mode: TlsMode::SelfSigned,
            tls_domain: String::new(),
            tls_email: String::new(),
            tls_staging: false,
            tls_custom_cert: String::new(),
            tls_custom_key: String::new(),
            tls_status: TlsStatus::Stopped,
            tls_cert_dir: std::path::PathBuf::new(),
            tls_cert_path: String::new(),
            tls_cert_info: None,
            tls_probe: None,
            tls_probe_lan_ok: false,
            tls_last_error: String::new(),
            tls_acme_busy: false,
            tls_accepted: 0,
            tls_proxy: None,
            tls_acme_rx: None,
            tls_renew_check: None,

            tunnel_child: None,
            tunnel_gate_stop: None,
            tunnel_last_poll: None,
            tunnel_last_probe: None,
        };
        state.log_debug(&format!("[startup] IP LAN autodetectado: {}", state.lan_ip));
        // Antes de qualquer leitura de preferência: saber se estamos portáteis
        // decide DE ONDE ler (arquivo ao lado do exe x registro).
        state.detect_portable_mode();
        state.startup_reconcile_tracked_rules();
        #[cfg(target_os = "windows")]
        state.startup_reconcile_tracked_tunnels();
        #[cfg(target_os = "windows")]
        state.read_mcp_token();
        state.read_minimize_to_tray();
        state.check_driver_present();
        state.check_port_status();
        state.daemon_running = state.port_active;
        // HTTPS: lê preferências, garante o auto-assinado ("na instalação ou no
        // primeiro run, o que vier primeiro") e sobe o listener se estava ligado.
        state.tls_startup();
        #[cfg(target_os = "windows")]
        state.read_autostart();
        state.fetch_screen_info();
        state
    }
}

impl AppState {
    /// Anexa uma entrada ao Console Debug (mantém tamanho limitado e 2 linhas em branco de espaçamento).
    pub fn log_debug(&mut self, entry: &str) {
        let entry_clean = entry.trim();
        if entry_clean.is_empty() {
            return;
        }
        if !self.debug_log.is_empty() {
            self.debug_log.push_str("\n\n");
        }
        self.debug_log.push_str(entry_clean);

        const MAX_LEN: usize = 64 * 1024;
        if self.debug_log.len() > MAX_LEN {
            let mut start = self.debug_log.len() - MAX_LEN;
            while !self.debug_log.is_char_boundary(start) {
                start += 1;
            }
            self.debug_log = self.debug_log[start..].to_string();
        }
    }

    /// Executa um comando via quiet_cmd e loga comando + exit code +
    /// stdout/stderr/erro no Console Debug. Retorna o Output (se rodou).
    fn run_logged(&mut self, program: &str, args: &[&str]) -> Option<std::process::Output> {
        let cmd_line = format!("{} {}", program, args.join(" "));
        match quiet_cmd(program).args(args).output() {
            Ok(out) => {
                let code = out
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let mut entry = format!("> {}\n  exit: {}", cmd_line, code);
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stdout.trim().is_empty() {
                    entry.push_str(&format!("\n  stdout: {}", stdout.trim()));
                }
                if !stderr.trim().is_empty() {
                    entry.push_str(&format!("\n  stderr: {}", stderr.trim()));
                }
                self.log_debug(&entry);
                Some(out)
            }
            Err(e) => {
                self.log_debug(&format!("> {}\n  ERRO ao executar: {}", cmd_line, e));
                None
            }
        }
    }

    /// Teste REAL do MCP em um endereço: conexao TCP + POST /mcp com um
    /// JSON-RPC `initialize` de verdade. Retorna true SOMENTE quando a
    /// resposta HTTP contem "jsonrpc" — o servidor MCP respondeu de fato.
    /// GET nao serve como prova: o endpoint MCP legitimamente devolve
    /// 405 Method Not Allowed para GET, o que so provava o TCP.
    fn mcp_probe(&mut self, ip: &str, port: u16) -> bool {
        use std::io::{Read, Write};

        let addr: std::net::SocketAddr = match format!("{}:{}", ip, port).parse() {
            Ok(a) => a,
            Err(_) => {
                self.log_debug(&format!(
                    "> mcp-check {}:{}\n  ERRO: endereco invalido",
                    ip, port
                ));
                return false;
            }
        };
        let timeout = std::time::Duration::from_millis(1200);

        match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));

                let body = concat!(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fzcomputerai-gui","version":""#,
                    env!("CARGO_PKG_VERSION"),
                    r#""}}}"#
                );
                // Authorization SO quando existe token configurado: o motor
                // antigo (<=0.8.x) ignora o header, e o novo (>=0.16) EXIGE.
                // Assim o mesmo probe fala com as duas gerações do motor.
                let auth = if self.mcp_token.trim().is_empty() {
                    String::new()
                } else {
                    format!("Authorization: Bearer {}\r\n", self.mcp_token.trim())
                };
                let request = format!(
                    "POST /mcp HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    ip,
                    port,
                    auth,
                    body.len(),
                    body
                );

                if let Err(e) = stream.write_all(request.as_bytes()) {
                    self.log_debug(&format!(
                        "> mcp-check {}:{}\n  TCP conectou mas a escrita falhou: {}",
                        ip, port, e
                    ));
                    return false;
                }

                // Le ate 4KB (o initialize cabe com folga); o timeout de
                // leitura encerra streams SSE que ficariam abertos.
                let mut collected: Vec<u8> = Vec::new();
                let mut buf = [0u8; 1024];
                while collected.len() < 4096 {
                    match stream.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => collected.extend_from_slice(&buf[..n]),
                        Err(_) => break,
                    }
                }
                let text = String::from_utf8_lossy(&collected).to_string();
                let status_line = text.lines().next().unwrap_or("(sem resposta)").to_string();

                if text.contains("jsonrpc") {
                    self.log_debug(&format!(
                        "> mcp-check {}:{}\n  MCP OK — {} + resposta JSON-RPC ao initialize.",
                        ip, port, status_line
                    ));
                    true
                } else {
                    self.log_debug(&format!(
                        "> mcp-check {}:{}\n  TCP conectou mas a resposta NAO e MCP: {}",
                        ip, port, status_line
                    ));
                    false
                }
            }
            Err(e) => {
                self.log_debug(&format!("> mcp-check {}:{}\n  SEM conexao ({})", ip, port, e));
                false
            }
        }
    }

    /// Fonte de verdade do sistema: netstat. Retorna true se existe LISTENER
    /// em `<lan_ip>:<porta>` ou `0.0.0.0:<porta>` (que cobre todas as
    /// interfaces). Loga as linhas do netstat que batem com a porta.
    fn netstat_lan_listening(&mut self, lan_ip: &str, port: u16) -> bool {
        let out = match quiet_cmd("netstat").args(["-ano", "-p", "tcp"]).output() {
            Ok(o) => o,
            Err(e) => {
                self.log_debug(&format!("> netstat -ano -p tcp\n  ERRO ao executar: {}", e));
                return false;
            }
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let want_lan = format!("{}:{}", lan_ip, port);
        let want_any = format!("0.0.0.0:{}", port);
        let suffix = format!(":{}", port);
        let lan_prefix = format!("{}:", lan_ip);

        let mut matched: Vec<String> = Vec::new();
        let mut lan_listening = false;
        for line in text.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 5 || !cols[0].eq_ignore_ascii_case("TCP") {
                continue;
            }
            let local = cols[1];
            let remote = cols[2];
            let state = cols[3];
            let pid_txt = cols[4];

            let is_listener = state.eq_ignore_ascii_case("LISTENING") || remote == "0.0.0.0:0";

            // O que interessa na tela: QUALQUER linha (LISTENING e tambem
            // ESTABLISHED — conexoes MCP reais em andamento) cujo endereco
            // local OU remoto use a porta configurada, mais listeners em
            // portas ALTAS (>=1024) no IP da LAN — assim um listener orfao
            // (ex.: portproxy antigo em outra porta) aparece na tela, sem
            // poluir com servicos de sistema (137/139/445...).
            let local_port_high = local
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok())
                .map(|p| p >= 1024)
                .unwrap_or(false);
            let interesting = local.ends_with(&suffix)
                || remote.ends_with(&suffix)
                || (is_listener && local.starts_with(&lan_prefix) && local_port_high);
            if interesting {
                // MESMAS colunas do netstat real (local, remoto, estado,
                // PID) para a tela bater 1:1 com o que o usuario ve no
                // proprio terminal. Num listener em espera o "remoto" e
                // 0.0.0.0:0 — e o formato do Windows, nao um destino.
                matched.push(format!(
                    "TCP  {:<22} {:<22} {:<12} pid {}",
                    local, remote, state, pid_txt
                ));
            }
            if is_listener && (local == want_lan || local == want_any) {
                lan_listening = true;
            }
        }
        self.real_listeners = matched.clone();

        if matched.is_empty() {
            self.log_debug(&format!(
                "> netstat -ano -p tcp (filtro :{})\n  nenhum LISTENER na porta {}",
                port, port
            ));
        } else {
            self.log_debug(&format!(
                "> netstat -ano -p tcp (filtro :{})\n  {}",
                port,
                matched.join("\n  ")
            ));
        }
        lan_listening
    }

    /// Teste REAL do endpoint MCP nos DOIS endereços (loopback e IP da LAN
    /// exibido), com o netstat como fonte de verdade para a LAN. Atualiza
    /// `port_status`:
    ///   LanListening = netstat mostra `<ip_lan>:<porta>` (ou 0.0.0.0) LISTENING
    ///                  E o TCP no IP da LAN conectou;
    ///   LocalOnly    = só 127.0.0.1 responde;
    ///   Stopped      = nada responde.
    pub fn check_port_status(&mut self) {
        let port: u16 = match self.http_port.trim().parse() {
            Ok(p) => p,
            Err(_) => {
                self.port_active = false;
                self.port_status = PortStatus::Stopped;
                let port_txt = self.http_port.clone();
                self.log_debug(&format!("> tcp-check :{}\n  ERRO: porta invalida", port_txt));
                return;
            }
        };

        // 1) loopback — MCP de verdade (POST initialize), nao so TCP
        let local_ok = self.mcp_probe("127.0.0.1", port);

        // 2) IP da LAN exibido na interface (o endereço publicado na URL MCP)
        let lan_ip = self.lan_ip.trim().to_string();
        let lan_is_loopback = lan_ip == "127.0.0.1" || lan_ip.eq_ignore_ascii_case("localhost");
        let lan_ok = if lan_is_loopback {
            self.log_debug("[status] IP LAN e loopback — teste LAN nao se aplica.");
            false
        } else {
            self.mcp_probe(&lan_ip, port)
        };

        // 3) fonte de verdade: netstat (roda SEMPRE, para real_listeners
        // refletir a verdade mesmo com IP loopback no campo)
        let netstat_result = self.netstat_lan_listening(&lan_ip, port);
        let netstat_lan = if lan_is_loopback { false } else { netstat_result };

        // Badge do portproxy recalculado AQUI, junto do netstat que ja rodou:
        // regra na config + listener LISTENING = funcionando; regra na config
        // sem listener = SEM EFEITO (IP Helper nao subiu o listener).
        #[cfg(target_os = "windows")]
        {
            // O encaminhamento pode vir de DOIS lugares: a thread do próprio
            // app (caminho normal desde a v2.1.1) ou uma regra netsh (fallback).
            // Olhar só o netsh fazia o badge dizer "SEM REGRA" com o
            // encaminhamento do app ATIVO e o console logo abaixo confirmando
            // LISTENING nos dois IPs — a GUI desmentindo a si mesma.
            let by_app = self.lan_forward_addr.is_some();
            let exists = by_app
                || (!lan_is_loopback && self.portproxy_rule_exists(&lan_ip, &port.to_string()));
            self.portproxy_active = exists;
            self.portproxy_effective = exists && netstat_lan;
            if exists && !netstat_lan && !by_app {
                self.log_debug(
                    "[portproxy] Regra existe na config do netsh mas o listener NAO esta de pe — SEM EFEITO. Reinicie o servico IP Helper (iphlpsvc) ou remova e reaplique a regra.",
                );
            }
        }

        if netstat_lan && !lan_ok {
            self.log_debug(
                "[status] AVISO: netstat mostra listener na LAN mas o teste TCP falhou (firewall?).",
            );
        }
        if lan_ok && !netstat_lan {
            self.log_debug(
                "[status] AVISO: TCP na LAN conectou mas netstat nao mostra o listener — nao pintando verde.",
            );
        }

        self.port_active = local_ok || lan_ok;
        self.port_status = if lan_ok && netstat_lan {
            PortStatus::LanListening
        } else if local_ok {
            PortStatus::LocalOnly
        } else {
            PortStatus::Stopped
        };

        let resumo = match self.port_status {
            PortStatus::LanListening => format!(
                "[status] LISTENING (local + LAN) — MCP respondeu JSON-RPC em 127.0.0.1:{p} E em {ip}:{p}; listener confirmado no netstat.",
                p = port,
                ip = lan_ip
            ),
            PortStatus::LocalOnly => format!(
                "[status] LOCAL APENAS — MCP responde em 127.0.0.1:{} mas NAO e acessivel pela LAN ({}).",
                port, lan_ip
            ),
            PortStatus::Stopped => format!("[status] STOPPED — nada respondeu na porta {}.", port),
        };
        self.log_debug(&resumo);
        // HTTPS faz parte da mesma verificacao (Testar Endpoint / startup):
        // se o listener existe, sonda TLS + JSON-RPC atras dele tambem.
        if self.tls_proxy.is_some() {
            self.check_tls_status();
        }
    }

    /// Grava uma variavel em HKCU\Environment pela via oficial e RELÊ o
    /// registro para confirmar sucesso/falha real. Retorna true somente com
    /// o valor confirmado.
    #[cfg(target_os = "windows")]
    fn set_user_env_confirmed(&mut self, name: &str, value: &str) -> bool {
        let ps = format!(
            "[Environment]::SetEnvironmentVariable('{}', '{}', 'User')",
            name, value
        );
        let set_ok = self
            .run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])
            .map(|o| o.status.success())
            .unwrap_or(false);

        let confirmed = self
            .run_logged("reg", &["query", r"HKCU\Environment", "/v", name])
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains(value))
            .unwrap_or(false);

        set_ok && confirmed
    }

    /// Define CUA_DRIVER_RS_MCP_HTTP_PORT + CUA_DRIVER_RS_MCP_HTTP_BIND
    /// (User) e reinicia o daemon. O bind vai para 0.0.0.0 (todas as
    /// interfaces); se o daemon nao conseguir/nao subir na LAN, o
    /// check_port_status mostra honestamente LOCAL APENAS (fallback
    /// 127.0.0.1) — nunca presumimos que a LAN esta servida.
    pub fn apply_env_port(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let port = self.http_port.trim().to_string();

            let port_ok = self.set_user_env_confirmed("CUA_DRIVER_RS_MCP_HTTP_PORT", &port);
            if port_ok {
                self.log_debug(&format!(
                    "[env] OK: CUA_DRIVER_RS_MCP_HTTP_PORT = {} confirmado em HKCU\\Environment.",
                    port
                ));
            } else {
                self.log_debug(
                    "[env] FALHA: CUA_DRIVER_RS_MCP_HTTP_PORT NAO confirmado em HKCU\\Environment.",
                );
            }

            // ─── SOBRE O "BIND 0.0.0.0" (leia antes de reintroduzir) ───
            // O motor OFICIAL do projeto Cua escuta APENAS em 127.0.0.1: o
            // endereco esta fixo no codigo (`([127,0,0,1], port)`) e NAO existe
            // variavel de bind no upstream. Verificado no binario instalado
            // (0.8.3): a string CUA_DRIVER_RS_MCP_HTTP_BIND nem aparece nele,
            // e no upstream atual a busca por essa variavel no repositorio
            // inteiro retorna zero ocorrencia.
            //
            // Ou seja: gravar essa variavel NAO publica nada na LAN — o motor a
            // ignora. Quem realmente entrega a LAN e o ENCAMINHAMENTO
            // (netsh portproxy, ao lado) ou um TUNEL (aba Tunel). Ela so tem
            // efeito num motor com patch local, que nao e o que o usuario roda.
            //
            // Deixamos de gravar a variavel e passamos a DIZER a verdade. Se
            // algum dia o upstream aceitar bind configuravel, reintroduza aqui
            // — mas com verificacao real no netstat, nunca por suposicao.
            self.log_debug(
                "[env] NOTA: o motor oficial escuta somente em 127.0.0.1 (bind fixo no codigo do Cua). \
                 Nao existe variavel de bind no upstream, portanto nada e gravado para isso. \
                 Para acesso pela LAN use o Encaminhamento (portproxy); para internet, a aba Tunel.",
            );

            if port_ok {
                self.log_debug("[env] Reiniciando daemon cua-driver para aplicar a configuracao...");
                self.run_logged("cua-driver", &["stop"]);
                self.run_logged("cua-driver", &["autostart", "kick"]);
            }

            // Higiene: se uma versao anterior desta GUI (ou o instalador) deixou
            // a variavel morta no ambiente do usuario, remove — configuracao que
            // nao faz nada so gera confusao no diagnostico.
            let leftover = self
                .run_logged(
                    "reg",
                    &["query", r"HKCU\Environment", "/v", "CUA_DRIVER_RS_MCP_HTTP_BIND"],
                )
                .map(|o| o.status.success())
                .unwrap_or(false);
            if leftover {
                let _ = self.run_logged(
                    "reg",
                    &[
                        "delete",
                        r"HKCU\Environment",
                        "/v",
                        "CUA_DRIVER_RS_MCP_HTTP_BIND",
                        "/f",
                    ],
                );
                self.log_debug(
                    "[env] Removida a variavel CUA_DRIVER_RS_MCP_HTTP_BIND, que o motor oficial ignora (config morta).",
                );
            }
        }
        self.check_port_status();
    }

    // ─── MODO PORTÁTIL: preferências em arquivo, não no registro ────────

    /// Caminho do marcador que liga o modo portátil (vai dentro do .zip).
    fn portable_marker() -> Option<std::path::PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("fzcomputerai.portable")))
    }

    /// Caminho do .ini de preferências, ao lado do executável.
    fn portable_ini() -> Option<std::path::PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("fzcomputerai.ini")))
    }

    /// Detecta o modo portátil pela presença do marcador.
    pub fn detect_portable_mode(&mut self) {
        self.portable_mode = Self::portable_marker()
            .map(|p| p.exists())
            .unwrap_or(false);
        if self.portable_mode {
            self.log_debug(
                "[portatil] MODO PORTATIL ativo (marcador fzcomputerai.portable encontrado). \
                 Preferencias ficam em fzcomputerai.ini ao lado do executavel; \
                 'Iniciar com o Windows' fica indisponivel. O rastreio de limpeza \
                 (portproxy/tunel) continua no registro por seguranca.",
            );
        }
    }

    /// Lê uma preferência: arquivo no modo portátil, registro fora dele.
    fn cfg_get(&mut self, key: &str) -> Option<String> {
        if self.portable_mode {
            let path = Self::portable_ini()?;
            let text = std::fs::read_to_string(path).ok()?;
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.starts_with(';') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim() == key {
                        return Some(v.trim().to_string());
                    }
                }
            }
            None
        } else {
            #[cfg(target_os = "windows")]
            {
                let out = self.run_logged(
                    "reg",
                    &["query", r"HKCU\Software\FzComputerAI", "/v", key],
                )?;
                if !out.status.success() {
                    return None;
                }
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    if line.contains(key) {
                        if let Some(pos) = line.find("REG_SZ") {
                            return Some(line[pos + "REG_SZ".len()..].trim().to_string());
                        }
                    }
                }
                None
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = key;
                None
            }
        }
    }

    /// Grava uma preferência: arquivo no modo portátil, registro fora dele.
    fn cfg_set(&mut self, key: &str, value: &str) {
        if self.portable_mode {
            let Some(path) = Self::portable_ini() else {
                return;
            };
            // Reescreve preservando as outras chaves (sem apagar o que não é nosso).
            let mut lines: Vec<String> = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.to_string())
                .collect();
            let mut replaced = false;
            for line in lines.iter_mut() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.starts_with(';') {
                    continue;
                }
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == key {
                        *line = format!("{}={}", key, value);
                        replaced = true;
                    }
                }
            }
            if !replaced {
                if lines.is_empty() {
                    lines.push(
                        "# FzComputerAI - preferencias do MODO PORTATIL".to_string(),
                    );
                }
                lines.push(format!("{}={}", key, value));
            }
            let _ = std::fs::write(&path, lines.join("\r\n") + "\r\n");
        } else {
            #[cfg(target_os = "windows")]
            {
                let _ = self.run_logged(
                    "reg",
                    &[
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        key,
                        "/t",
                        "REG_SZ",
                        "/d",
                        value,
                        "/f",
                    ],
                );
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (key, value);
            }
        }
    }

    /// Liga/desliga "minimizar para a bandeja" e PERSISTE a preferência em
    /// `HKCU\Software\FzComputerAI` (`appcfg:minimize_to_tray`), na mesma chave
    /// já usada pelo resto do app. O ícone em si é criado/removido pelo
    /// `update()`, que é quem tem a mão na janela.
    pub fn set_minimize_to_tray(&mut self, enable: bool) {
        self.minimize_to_tray = enable;
        if !enable {
            self.window_hidden = false;
        }
        let value = if enable { "1" } else { "0" };
        self.cfg_set("appcfg:minimize_to_tray", value);
        self.log_debug(&format!(
            "[tray] Minimizar para a bandeja: {}",
            if enable { "ATIVADO" } else { "DESATIVADO" }
        ));
    }

    /// Lê a preferência da bandeja no startup (sem isso o checkbox esqueceria
    /// a escolha do usuário a cada abertura). Respeita o modo portátil.
    pub fn read_minimize_to_tray(&mut self) {
        if let Some(v) = self.cfg_get("appcfg:minimize_to_tray") {
            self.minimize_to_tray = v == "1";
        }
    }

    /// Lê o token do endpoint HTTP do motor de `HKCU\Environment`. Motores
    /// 0.16+ EXIGEM esse token; sem enviá-lo, todo probe voltaria 401 e o app
    /// reportaria "MCP parado" com o motor perfeitamente vivo.
    #[cfg(target_os = "windows")]
    pub fn read_mcp_token(&mut self) {
        if let Some(out) = self.run_logged(
            "reg",
            &[
                "query",
                r"HKCU\Environment",
                "/v",
                "CUA_DRIVER_RS_MCP_HTTP_TOKEN",
            ],
        ) {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    if !line.contains("CUA_DRIVER_RS_MCP_HTTP_TOKEN") {
                        continue;
                    }
                    // Formato: NOME<espacos>REG_SZ<espacos>VALOR — cortar em
                    // "REG_SZ" (o valor nao tem espaco, mas o parser fica
                    // correto de qualquer forma).
                    if let Some(pos) = line.find("REG_SZ") {
                        let value = line[pos + "REG_SZ".len()..].trim().to_string();
                        if !value.is_empty() {
                            self.mcp_token = value;
                            self.log_debug(
                                "[env] Token do endpoint MCP encontrado em HKCU\\Environment — sera enviado como Bearer nos testes. (valor NAO exibido)",
                            );
                        }
                    }
                }
            }
        }
    }

    /// Porta CUA CONFIGURADA e CONFIRMADA — nunca chutada.
    /// Ordem dos candidatos: (1) CUA_DRIVER_RS_MCP_HTTP_PORT em
    /// HKCU\Environment (a configuracao real do daemon), (2) a porta do campo
    /// da UI, (3) o default 8000. Retorna a PRIMEIRA que RESPONDER de fato em
    /// 127.0.0.1 (tcp_probe). Nenhuma respondeu => None: quem chamar decide o
    /// que fazer, mas regra de encaminhamento para porta morta NAO se cria.
    pub fn detect_confirmed_cua_port(&mut self) -> Option<u16> {
        let mut candidates: Vec<u16> = Vec::new();

        #[cfg(target_os = "windows")]
        if let Some(out) = self.run_logged(
            "reg",
            &[
                "query",
                r"HKCU\Environment",
                "/v",
                "CUA_DRIVER_RS_MCP_HTTP_PORT",
            ],
        ) {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    if line.contains("CUA_DRIVER_RS_MCP_HTTP_PORT") {
                        if let Some(tok) = line.split_whitespace().last() {
                            if let Ok(p) = tok.parse::<u16>() {
                                candidates.push(p);
                            }
                        }
                    }
                }
            }
        }

        if let Ok(p) = self.http_port.trim().parse::<u16>() {
            if !candidates.contains(&p) {
                candidates.push(p);
            }
        }
        if !candidates.contains(&8000) {
            candidates.push(8000);
        }

        for p in candidates {
            if self.mcp_probe("127.0.0.1", p) {
                self.log_debug(&format!(
                    "[portproxy] Porta CUA confirmada respondendo MCP em 127.0.0.1:{}.",
                    p
                ));
                return Some(p);
            }
        }
        self.log_debug(
            "[portproxy] NENHUM candidato de porta CUA respondeu em 127.0.0.1 (config HKCU, campo da UI e 8000 testados).",
        );
        None
    }

    /// Lê `netsh interface portproxy show v4tov4` e retorna true se existe
    /// linha cujo par (endereço de escuta, porta de escuta) é exatamente
    /// `<ip> <porta>`. Parse por token — nada de contains() solto.
    #[cfg(target_os = "windows")]
    fn portproxy_rule_exists(&mut self, ip: &str, port: &str) -> bool {
        match self.run_logged("netsh", &["interface", "portproxy", "show", "v4tov4"]) {
            Some(show) => {
                let text = String::from_utf8_lossy(&show.stdout).to_string();
                // Guarda TODAS as regras v4tov4 (linhas com portas numericas)
                // para a UI listar — inclusive regras orfas em outras portas.
                self.portproxy_rules = text
                    .lines()
                    .filter_map(|line| {
                        let cols: Vec<&str> = line.split_whitespace().collect();
                        if cols.len() >= 4
                            && cols[1].parse::<u16>().is_ok()
                            && cols[3].parse::<u16>().is_ok()
                        {
                            Some(format!("{}:{} -> {}:{}", cols[0], cols[1], cols[2], cols[3]))
                        } else {
                            None
                        }
                    })
                    .collect();
                text.lines().any(|line| {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    cols.len() >= 2 && cols[0] == ip && cols[1] == port
                })
            }
            None => false,
        }
    }

    /// O APP aplica a regra portproxy do netsh (não apenas sugere).
    /// Etapas, todas logadas com o resultado real:
    ///   1. verificar se a regra já existe (show v4tov4);
    ///   2. tentar `netsh ... add` direto; se falhar, elevar via UAC oficial
    ///      (Start-Process -Verb RunAs) propagando o exit code do netsh elevado;
    ///   3. reler `show v4tov4` e confirmar a regra por token;
    ///   4. confirmar com netstat que `<ip>:<porta>` está LISTENING;
    ///   5. refazer o teste TCP nos dois endereços (check_port_status).
    pub fn apply_portproxy(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let listen_port = self.http_port.trim().to_string();
            let ip = self.lan_ip.trim().to_string();

            if ip == "127.0.0.1" || ip.eq_ignore_ascii_case("localhost") {
                self.log_debug(
                    "[portproxy] IP da LAN e loopback — regra nao faz sentido. Ajuste o campo IP.",
                );
                return;
            }
            let listen_port_num: u16 = match listen_port.parse() {
                Ok(p) => p,
                Err(_) => {
                    self.log_debug("[portproxy] Porta invalida — corrija o campo Porta.");
                    return;
                }
            };

            // CAMINHO NOVO — encaminhamento DENTRO do app (thread), no lugar do
            // netsh portproxy. Sem admin, sem regra que sobrevive ao app, e o
            // que escuta na LAN é este processo: quando ele fecha, a porta
            // fecha junto. É TCP puro, então curl, telnet e nc atravessam igual.
            // O netsh continua abaixo apenas como FALLBACK, para o caso de o
            // bind no IP da LAN falhar.
            if self.start_lan_forward(&ip, listen_port_num) {
                self.portproxy_active = true;
                self.status_msg = self.tr(
                    "Encaminhamento LAN ativo (pelo proprio app, sem netsh/admin).",
                    "LAN forwarding active (by the app itself, no netsh/admin).",
                );
                self.check_port_status();
                return;
            }
            self.log_debug(
                "[lan] Nao consegui escutar no IP da LAN; caindo para o netsh portproxy (regra do sistema, exige admin).",
            );

            // Destino da regra: a porta CUA CONFIGURADA e CONFIRMADA por teste
            // TCP real — nunca a porta "sugerida" sem confirmacao. Sem porta
            // confirmada nao se cria regra nenhuma (encaminhar para porta
            // morta so mascara o problema).
            let connect_port = match self.detect_confirmed_cua_port() {
                Some(p) => p,
                None => {
                    self.log_debug(
                        "[portproxy] Regra NAO criada: nenhuma porta CUA confirmada em 127.0.0.1. Inicie o daemon (botao Iniciar) e tente de novo.",
                    );
                    self.check_port_status();
                    return;
                }
            };

            // Etapa 1: regra já existe?
            if self.portproxy_rule_exists(&ip, &listen_port) {
                self.log_debug(&format!(
                    "[portproxy] Regra {}:{} -> 127.0.0.1:{} JA existe — pulando o add.",
                    ip, listen_port, connect_port
                ));
            } else if self.netstat_lan_listening(&ip, listen_port_num) {
                // Ja ha um listener REAL em <lan>:<porta> que NAO e regra
                // portproxy (ex.: o proprio daemon com bind 0.0.0.0, ou outro
                // processo). Criar a regra sobrescreveria/conflitaria com a
                // porta de um processo existente — nao fazemos isso.
                self.log_debug(&format!(
                    "[portproxy] Ja existe listener em {}:{} que NAO e regra portproxy (bind 0.0.0.0 do daemon ou outro processo). Nada foi alterado.",
                    ip, listen_port
                ));
                self.portproxy_active = false;
                self.check_port_status();
                return;
            } else {
                // Etapa 2: aplicar (direto; se falhar, elevado via UAC)
                let netsh_args = format!(
                    "interface portproxy add v4tov4 listenport={} listenaddress={} connectport={} connectaddress=127.0.0.1",
                    listen_port, ip, connect_port
                );
                let args_vec: Vec<&str> = netsh_args.split(' ').collect();

                let direct_ok = self
                    .run_logged("netsh", &args_vec)
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                if direct_ok {
                    self.log_debug("[portproxy] netsh add direto concluiu com exit 0.");
                } else {
                    self.log_debug(
                        "[portproxy] Tentativa direta falhou — solicitando elevacao (UAC)...",
                    );
                    // -PassThru + exit $p.ExitCode: o exit code do netsh ELEVADO
                    // chega até aqui. UAC cancelado => Start-Process lança erro
                    // e o powershell sai com exit != 0.
                    let ps = format!(
                        "$p = Start-Process -FilePath netsh -ArgumentList '{}' -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
                        netsh_args
                    );
                    match self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()]) {
                        Some(o) if o.status.success() => {
                            self.log_debug("[portproxy] netsh elevado concluiu com exit 0.")
                        }
                        Some(o) => self.log_debug(&format!(
                            "[portproxy] netsh elevado FALHOU ou UAC foi cancelado (exit {:?}).",
                            o.status.code()
                        )),
                        None => {}
                    }
                }
            }

            // Etapa 3: reler a verdade do netsh
            let rule_ok = self.portproxy_rule_exists(&ip, &listen_port);
            self.portproxy_active = rule_ok;
            if rule_ok {
                self.log_debug(&format!(
                    "[portproxy] Regra CONFIRMADA no show v4tov4: {}:{} -> 127.0.0.1:{}",
                    ip, listen_port, connect_port
                ));
                // Registra a regra como PROPRIEDADE deste app (HKCU). O
                // shutdown_cleanup so remove regras registradas aqui —
                // nunca regras de outros servicos com o mesmo padrao
                // (ex.: 8082->8082 ou 9333->9222 de outras ferramentas).
                let value_name = format!("portproxy:{}:{}", ip, listen_port);
                let cp = connect_port.to_string();
                let _ = self.run_logged(
                    "reg",
                    &[
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        value_name.as_str(),
                        "/t",
                        "REG_SZ",
                        "/d",
                        cp.as_str(),
                        "/f",
                    ],
                );
            } else {
                self.log_debug(
                    "[portproxy] Regra NAO encontrada em 'show v4tov4' — nada foi aplicado.",
                );
            }

            // Etapa 4: confirmar com netstat (o listener do portproxy e do iphlpsvc)
            if let Ok(p) = listen_port.parse::<u16>() {
                let listening = self.netstat_lan_listening(&ip, p);
                if rule_ok && !listening {
                    self.log_debug(
                        "[portproxy] AVISO: regra existe no netsh mas netstat NAO mostra o listener. Verifique o servico 'IP Helper' (iphlpsvc).",
                    );
                }
            }
        }
        // Etapa 5: teste TCP real nos dois endereços + status
        self.check_port_status();
    }

    /// Regras portproxy registradas como PROPRIEDADE deste app em
    /// HKCU\Software\FzComputerAI (valores "portproxy:<ip>:<porta>").
    #[cfg(target_os = "windows")]
    fn tracked_portproxy_rules() -> Vec<(String, String)> {
        let mut tracked: Vec<(String, String)> = Vec::new();
        if let Ok(out) = quiet_cmd("reg")
            .args(["query", r"HKCU\Software\FzComputerAI"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            for line in text.lines() {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if let Some(name) = cols.first() {
                    if let Some(rest) = name.strip_prefix("portproxy:") {
                        if let Some((ip, port)) = rest.rsplit_once(':') {
                            if port.parse::<u16>().is_ok() {
                                tracked.push((ip.to_string(), port.to_string()));
                            }
                        }
                    }
                }
            }
        }
        tracked
    }

    /// RECONCILIACAO NA ABERTURA: um fechamento anterior pode ter falhado
    /// (kill forcado, UAC recusado, queda de energia) e deixado regras
    /// portproxy NOSSAS vivas — portas abertas com o software "fechado".
    /// Aqui, toda regra RASTREADA encontrada na config do netsh e removida
    /// (tentativa direta; sem privilegio, o usuario e avisado no console e
    /// a regra sai no proximo fechamento com UAC ou pelo botao Remover).
    pub fn startup_reconcile_tracked_rules(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let tracked = Self::tracked_portproxy_rules();
            if tracked.is_empty() {
                return;
            }
            for (ip, port) in tracked {
                if !self.portproxy_rule_exists(&ip, &port) {
                    // Regra ja nao existe — apenas desregistra.
                    let value_name = format!("portproxy:{}:{}", ip, port);
                    let _ = quiet_cmd("reg")
                        .args([
                            "delete",
                            r"HKCU\Software\FzComputerAI",
                            "/v",
                            value_name.as_str(),
                            "/f",
                        ])
                        .output();
                    continue;
                }
                self.log_debug(&format!(
                    "[startup] Sobra de sessao anterior: regra portproxy {}:{} ainda ativa — removendo...",
                    ip, port
                ));
                let ok = quiet_cmd("netsh")
                    .args([
                        "interface",
                        "portproxy",
                        "delete",
                        "v4tov4",
                        &format!("listenport={}", port),
                        &format!("listenaddress={}", ip),
                    ])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                if ok && !self.portproxy_rule_exists(&ip, &port) {
                    self.log_debug("[startup] Sobra removida e confirmada no show v4tov4.");
                    let value_name = format!("portproxy:{}:{}", ip, port);
                    let _ = quiet_cmd("reg")
                        .args([
                            "delete",
                            r"HKCU\Software\FzComputerAI",
                            "/v",
                            value_name.as_str(),
                            "/f",
                        ])
                        .output();
                } else {
                    self.log_debug(
                        "[startup] AVISO: sem privilegio para remover a sobra agora. Ela sera removida no fechamento (UAC) ou use o botao Remover Regra.",
                    );
                }
            }
        }
    }

    /// Encerramento LIMPO ao fechar a GUI (chamado pelo on_exit do eframe):
    ///   1. para/mata o daemon cua-driver;
    ///   2. remove as regras portproxy QUE ESTE APP CRIOU — as registradas
    ///      em HKCU\Software\FzComputerAI (valores "portproxy:<ip>:<porta>").
    /// NUNCA apaga regra por "padrao parecido": nesta mesma maquina existem
    /// regras LAN->127.0.0.1 de OUTROS servicos (ex.: 8082->8082,
    /// 9333->9222) que nao sao nossas e nao podem ser tocadas.
    ///
    /// CRITICO — POR QUE ISTO E DESACOPLADO (nao mexa): a versao anterior
    /// fazia a limpeza AQUI, de forma bloqueante, incluindo
    /// `Start-Process -Verb RunAs -Wait` para elevar o netsh. Como o netsh
    /// delete exige admin, TODO fechamento abria um UAC e o processo ficava
    /// preso esperando resposta — o app simplesmente NAO FECHAVA (janela
    /// some, processo vivo, portas abertas). Agora o on_exit apenas DISPARA
    /// um auxiliar independente (spawn, sem wait) e retorna na hora: quem
    /// espera o app morrer, mata o motor, elimina as regras e lida com o UAC
    /// e o auxiliar — nunca a GUI.
    pub fn shutdown_cleanup(&mut self) {
        // Mata o processo do túnel de imediato (instantâneo, não bloqueia o
        // fechamento). O watchdog independente cobre o caso de kill -9.
        if let Some(mut child) = self.tunnel_child.take() {
            let _ = child.kill();
        }
        self.stop_gate();
        // O encaminhamento LAN é uma thread deste processo: fechar o app já o
        // encerra. Nada a remover do sistema neste caminho.
        self.stop_lan_forward();
        // Listener HTTPS: idem — thread do processo, cai junto.
        self.stop_tls();

        // ===================================================================
        // LIMPEZA NATIVA — NUNCA VOLTE A FAZER ISTO COM POWERSHELL OCULTO
        // -------------------------------------------------------------------
        // A versão anterior disparava um `powershell -WindowStyle Hidden` de
        // ~2 KB que esperava este processo morrer, matava processos, mexia no
        // registro, rodava netsh e ainda chamava `-Verb RunAs`. O Microsoft
        // Defender FLAGROU exatamente essa linha de comando nesta máquina
        // (detecção 2147941383, 2026-08-03 19:19), e com razão: é o retrato
        // do que heurística de malware procura — script oculto + kill +
        // persistência + elevação.
        // Agora tudo é feito aqui, com chamadas diretas e curtas (cada uma
        // via quiet_cmd → CREATE_NO_WINDOW, sem console piscando):
        //   1. `cua-driver stop` — o comando OFICIAL da CLI, não kill;
        //   2. regras portproxy criadas por NÓS (registradas em
        //      HKCU\Software\FzComputerAI) removidas por netsh, uma a uma;
        //   3. os valores correspondentes apagados do registro por reg.exe.
        // Nada é elevado: se o netsh precisar de admin e falhar, o valor fica
        // no registro e a reconciliação da próxima abertura tenta de novo —
        // melhor deixar rastro do que abrir UAC no fechamento (o que já
        // travou o app fechando no passado).
        // ===================================================================
        #[cfg(target_os = "windows")]
        {
            let exe = Self::engine_exe();
            let _ = quiet_cmd(&exe).arg("stop").output();

            // Regras portproxy nossas: HKCU\Software\FzComputerAI, valores
            // "portproxy:<ip>:<porta>". Só as NOSSAS — nunca por semelhança.
            if let Ok(out) = quiet_cmd("reg")
                .args(["query", "HKCU\\Software\\FzComputerAI"])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                for line in text.lines() {
                    let line = line.trim();
                    if !line.starts_with("portproxy:") {
                        continue;
                    }
                    // "portproxy:<ip>:<porta>    REG_SZ    <dado>"
                    let name = line.split_whitespace().next().unwrap_or("").to_string();
                    let rule = name.trim_start_matches("portproxy:");
                    if let Some(idx) = rule.rfind(':') {
                        let (addr, port) = (&rule[..idx], &rule[idx + 1..]);
                        let _ = quiet_cmd("netsh")
                            .args([
                                "interface",
                                "portproxy",
                                "delete",
                                "v4tov4",
                                &format!("listenport={}", port),
                                &format!("listenaddress={}", addr),
                            ])
                            .output();
                    }
                    let _ = quiet_cmd("reg")
                        .args([
                            "delete",
                            "HKCU\\Software\\FzComputerAI",
                            "/v",
                            &name,
                            "/f",
                        ])
                        .output();
                }
            }
        }
    }

    /// Remove a regra portproxy (netsh delete) com o MESMO fluxo honesto do
    /// apply_portproxy: tentativa direta, fallback elevado via UAC oficial e
    /// confirmação relendo `show v4tov4` — o estado exibido nunca é presumido.
    pub fn remove_portproxy(&mut self) {
        // Se o encaminhamento é nosso (thread no app), basta derrubá-lo — não
        // há regra de sistema para apagar nem admin a pedir.
        if self.lan_forward_addr.is_some() {
            self.stop_lan_forward();
            self.portproxy_active = false;
            self.status_msg = self.tr(
                "Encaminhamento LAN encerrado.",
                "LAN forwarding stopped.",
            );
            self.check_port_status();
            return;
        }
        #[cfg(target_os = "windows")]
        {
            let listen_port = self.http_port.trim().to_string();
            let ip = self.lan_ip.trim().to_string();

            if listen_port.parse::<u16>().is_err() {
                self.log_debug("[portproxy] Porta invalida — corrija o campo Porta.");
                return;
            }

            if !self.portproxy_rule_exists(&ip, &listen_port) {
                self.log_debug(&format!(
                    "[portproxy] Nenhuma regra {}:{} para remover.",
                    ip, listen_port
                ));
                self.portproxy_active = false;
                return;
            }

            let netsh_args = format!(
                "interface portproxy delete v4tov4 listenport={} listenaddress={}",
                listen_port, ip
            );
            let args_vec: Vec<&str> = netsh_args.split(' ').collect();

            let direct_ok = self
                .run_logged("netsh", &args_vec)
                .map(|o| o.status.success())
                .unwrap_or(false);

            if direct_ok {
                self.log_debug("[portproxy] netsh delete direto concluiu com exit 0.");
            } else {
                self.log_debug(
                    "[portproxy] Delete direto falhou — solicitando elevacao (UAC)...",
                );
                let ps = format!(
                    "$p = Start-Process -FilePath netsh -ArgumentList '{}' -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
                    netsh_args
                );
                match self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()]) {
                    Some(o) if o.status.success() => {
                        self.log_debug("[portproxy] netsh delete elevado concluiu com exit 0.")
                    }
                    Some(o) => self.log_debug(&format!(
                        "[portproxy] netsh delete elevado FALHOU ou UAC foi cancelado (exit {:?}).",
                        o.status.code()
                    )),
                    None => {}
                }
            }

            // Verdade final: reler o netsh.
            let still_there = self.portproxy_rule_exists(&ip, &listen_port);
            self.portproxy_active = still_there;
            if still_there {
                self.log_debug(
                    "[portproxy] Regra AINDA presente apos o delete — nada foi removido.",
                );
            } else {
                self.log_debug(&format!(
                    "[portproxy] Regra {}:{} removida e confirmada no show v4tov4.",
                    ip, listen_port
                ));
                // Regra removida => sai tambem do registro de propriedade.
                let value_name = format!("portproxy:{}:{}", ip, listen_port);
                let _ = self.run_logged(
                    "reg",
                    &[
                        "delete",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        value_name.as_str(),
                        "/f",
                    ],
                );
            }
        }
        self.check_port_status();
    }

    pub fn fetch_screen_info(&mut self) {
        match self.run_logged("cua-driver", &["call", "get_screen_size"]) {
            Some(out) if out.status.success() => {
                self.status_msg = format!(
                    "get_screen_size:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "get_screen_size falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn test_click_position(&mut self) {
        let x: i32 = self.test_x.parse().unwrap_or(0);
        let y: i32 = self.test_y.parse().unwrap_or(0);
        let xs = x.to_string();
        let ys = y.to_string();

        match self.run_logged(
            "cua-driver",
            &["call", "move_cursor", "--x", xs.as_str(), "--y", ys.as_str()],
        ) {
            Some(out) if out.status.success() => {
                self.status_msg = format!(
                    "move_cursor ({}, {}):\n{}",
                    x,
                    y,
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "move_cursor ({}, {}) falhou (exit {:?}):\n{}",
                    x,
                    y,
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn refresh_windows(&mut self) {
        match self.run_logged("cua-driver", &["call", "list_windows"]) {
            Some(out) if out.status.success() => {
                self.status_msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
            Some(out) => {
                self.status_msg = format!(
                    "list_windows falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn launch_app(&mut self) {
        let app = self.launch_input.trim().to_string();
        match self.run_logged("cua-driver", &["call", "launch_app", "--app", app.as_str()]) {
            Some(out) if out.status.success() => {
                self.status_msg = format!(
                    "launch_app '{}':\n{}",
                    app,
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "launch_app '{}' falhou (exit {:?}):\n{}",
                    app,
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn start_recording(&mut self) {
        match self.run_logged("cua-driver", &["call", "start_recording"]) {
            Some(out) if out.status.success() => {
                self.is_recording = true;
                self.status_msg = format!(
                    "start_recording:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "start_recording falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn stop_recording(&mut self) {
        match self.run_logged("cua-driver", &["call", "stop_recording"]) {
            Some(out) if out.status.success() => {
                self.is_recording = false;
                self.status_msg = format!(
                    "stop_recording:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.status_msg = format!(
                    "stop_recording falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    /// Sobe o daemon do motor. O `autostart kick` sozinho NÃO basta desde o
    /// motor 0.16+: a Scheduled Task herda o ambiente do LOGON, e quem
    /// configurou `CUA_DRIVER_RS_MCP_HTTP_TOKEN` depois de logar (o caso
    /// normal) sobe um daemon sem token — que morre com
    /// "must be set to a host-generated bearer token" e deixa a porta muda.
    /// Por isso o fallback lança `serve` DIRETO, com porta e token lidos do
    /// registro e injetados no ambiente do processo filho. Só há UM daemon:
    /// `stop` antes, senão o `serve` recusa com "already running".
    /// Sobe o daemon do motor — a GUI é a DONA do processo, não uma
    /// intermediária que pede para o Agendador de Tarefas fazer.
    ///
    /// POR QUE NÃO `autostart kick` (era o que esta função fazia): o `kick`
    /// manda a Scheduled Task `cua-driver-serve` subir o daemon, e aí o
    /// processo é filho do AGENDADOR. Três consequências, todas observadas:
    ///   1. o stdout/stderr do motor pertence à task e se PERDE — a atividade
    ///      de clientes MCP externos (conector do Claude, Antigravity, Cursor)
    ///      nunca chegava ao console da GUI;
    ///   2. a task herda o ambiente do LOGON, então token/porta gravados depois
    ///      de logar não são vistos: com motor 0.16+ o daemon morre no ato
    ///      ("must be set to a host-generated bearer token") e a porta fica muda;
    ///   3. a GUI não sabe quando o daemon morreu — só descobre pela sonda.
    /// Um gerenciador tem de ser dono do que gerencia: lançamos `serve` como
    /// processo filho, com porta e token injetados e o stdout num arquivo que o
    /// console segue (`poll_engine_log`). O autostart do Windows continua
    /// existindo para o logon — este caminho é o da GUI.
    pub fn start_daemon(&mut self) {
        // JÁ ESTÁ NO AR? Então não encoste. "Iniciar" com o endpoint
        // respondendo derrubava um daemon saudável e, por causa do TIME_WAIT do
        // Windows (sockets da porta 8000 que já tiveram conexão ficam retidos
        // por minutos), o novo `serve` não conseguia mais o bind:
        // "MCP HTTP transport disabled (os error 10048)". Resultado prático:
        // clicar Iniciar QUEBRAVA o que estava funcionando.
        self.check_port_status();
        if self.port_active {
            self.daemon_running = true;
            self.log_debug(
                "[daemon] Ja esta no ar (endpoint respondendo): nada a fazer. Use Reiniciar para forcar uma troca de processo.",
            );
            self.status_msg = self.tr(
                "O motor ja esta em execucao — endpoint respondendo.",
                "The engine is already running — endpoint responding.",
            );
            return;
        }

        #[cfg(target_os = "windows")]
        {
            let port = Self::read_user_env("CUA_DRIVER_RS_MCP_HTTP_PORT")
                .unwrap_or_else(|| "8000".to_string());
            // Token: se não existir, a GUI GERA e persiste. O motor 0.16+ chama
            // o valor de "host-generated bearer token" — o host é esta GUI, e
            // exigir que o usuário invente e grave uma variável de ambiente à
            // mão para o produto simplesmente ligar é jogar o problema do
            // motor no colo de quem só quer usar. Sem isto, no 0.16+, o `serve`
            // sai com erro e NENHUMA porta abre: o app fica "PARADO" para sempre.
            let token = match Self::read_user_env("CUA_DRIVER_RS_MCP_HTTP_TOKEN") {
                Some(t) if !t.trim().is_empty() => Some(t),
                _ => self.generate_and_store_mcp_token(),
            };
            let exe = Self::engine_exe();

            // Só pode existir UM daemon (o segundo `serve` recusa com
            // "already running"): derruba o que estiver de pé — inclusive um
            // subido pela task, sem HTTP — antes de assumir o processo.
            let _ = self.run_logged(&exe, &["stop"]);

            // Espera o PROCESSO do motor anterior morrer — NUNCA testar a porta
            // com um bind próprio antes de lançar. MEDIDO nesta máquina: um
            // `TcpListener::bind` de "verificação", mesmo soltando o listener
            // em seguida, faz o motor lançado logo depois falhar com
            // "MCP HTTP transport disabled — bind 127.0.0.1:8000 failed
            // (os error 10048)" e virar daemon ZUMBI (pipe vivo, porta muda).
            // Sem esse teste, ele sobe com HTTP normalmente. Verificar o
            // processo é não-intrusivo e resolve o mesmo problema.
            for _ in 0..20 {
                let still_alive = quiet_cmd("tasklist")
                    .args(["/FI", "IMAGENAME eq cua-driver.exe", "/NH"])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).contains("cua-driver.exe"))
                    .unwrap_or(false);
                if !still_alive {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            // Folga curta para o socket do processo morto ser reciclado pelo SO.
            std::thread::sleep(std::time::Duration::from_millis(600));

            if token.is_none() {
                self.log_debug(
                    "[daemon] AVISO: CUA_DRIVER_RS_MCP_HTTP_TOKEN nao esta em HKCU\\Environment e nao consegui gerar um. Motores 0.16+ EXIGEM token e o daemon vai recusar subir.",
                );
            }
            let log_path = Self::engine_log_path();
            if let Some(dir) = log_path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }

            // RETRY com espera crescente. O socket do daemon anterior fica em
            // TIME_WAIT quando houve conexões (o caso normal: alguém usou o
            // MCP), e o motor sobe com "MCP HTTP transport disabled — bind
            // failed (os error 10048)" — daemon zumbi: pipe vivo, porta muda.
            // Esperar o processo morrer não basta; o SO precisa reciclar o
            // socket. Três tentativas cobrem o TIME_WAIT típico do Windows.
            //
            // O Command é montado DENTRO do laço de propósito: um `Stdio` é
            // CONSUMIDO no primeiro spawn, então reaproveitar o mesmo Command
            // faria a segunda tentativa subir sem o log redirecionado (e foi
            // exatamente o que aconteceu: o arquivo só continha a 1ª tentativa).
            let mut launched = false;
            for attempt in 1..=3 {
                let mut cmd = quiet_cmd(&exe);
                cmd.arg("serve").env("CUA_DRIVER_RS_MCP_HTTP_PORT", &port);
                if let Some(t) = &token {
                    cmd.env("CUA_DRIVER_RS_MCP_HTTP_TOKEN", t);
                }
                // Log REAL do motor num arquivo, para o console poder segui-lo
                // (tail). Sem isto o stdout do daemon destacado se perde e o que
                // um cliente MCP externo faz nao aparece em lugar nenhum.
                // Trunca a cada tentativa: o tail comeca do zero junto com ela.
                if let Ok(f) = std::fs::File::create(&log_path) {
                    let err_clone = f.try_clone().ok();
                    cmd.stdout(std::process::Stdio::from(f));
                    if let Some(e) = err_clone {
                        cmd.stderr(std::process::Stdio::from(e));
                    }
                    self.engine_log_pos = 0;
                }
                match cmd.spawn() {
                    Ok(_) => {
                        launched = true;
                        // O listener leva ~1-3s para abrir; sondar antes disso
                        // reportaria "parado" com o daemon subindo.
                        for _ in 0..10 {
                            std::thread::sleep(std::time::Duration::from_millis(400));
                            self.check_port_status();
                            if self.port_active {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        self.log_debug(&format!("[daemon] ERRO ao lancar 'serve': {}", e));
                        break;
                    }
                }
                if self.port_active {
                    break;
                }
                if attempt < 3 {
                    self.log_debug(&format!(
                        "[daemon] tentativa {}: a porta {} nao abriu (socket anterior ainda em TIME_WAIT). Parando e tentando de novo em {}s...",
                        attempt,
                        port,
                        attempt * 3
                    ));
                    let _ = quiet_cmd(&exe).arg("stop").output();
                    std::thread::sleep(std::time::Duration::from_secs((attempt * 3) as u64));
                }
            }
            if launched {
                self.log_debug(&format!(
                    "[daemon] 'serve' lancado (porta {}, token {}): porta ativa = {}",
                    port,
                    if token.is_some() { "presente" } else { "AUSENTE" },
                    self.port_active
                ));
            }

            // Último recurso: se o `serve` próprio não abriu a porta (motor
            // recusou por falta de token, exe inacessível), ainda tenta a task
            // do Windows — melhor um daemon sem logs do que nenhum daemon.
            if !self.port_active {
                self.log_debug(
                    "[daemon] 'serve' proprio nao abriu a porta; tentando a Scheduled Task (autostart kick) como ultimo recurso — sem logs no console, o processo nao sera nosso.",
                );
                let _ = self.run_logged(&exe, &["autostart", "kick"]);
                for _ in 0..6 {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    self.check_port_status();
                    if self.port_active {
                        break;
                    }
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = self.run_logged("cua-driver", &["autostart", "kick"]);
        }

        self.check_port_status();
        self.daemon_running = self.port_active;
    }

    /// Gera um bearer token e o persiste em `HKCU\Environment`, devolvendo-o.
    ///
    /// O motor 0.16+ recusa iniciar o endpoint HTTP sem
    /// `CUA_DRIVER_RS_MCP_HTTP_TOKEN` — ele o chama de *host-generated bearer
    /// token*, ou seja, quem hospeda é que decide o valor. Este app é o
    /// hospedeiro: gerar aqui é o que faz o produto funcionar na primeira
    /// execução, sem o usuário precisar saber que a variável existe.
    ///
    /// Aleatoriedade vem do `RNGCryptoServiceProvider` do Windows (via
    /// PowerShell, sem dependência nova): 32 bytes → 64 chars hex.
    /// Persistido com `setx` para a Scheduled Task do logon também enxergar.
    #[cfg(target_os = "windows")]
    fn generate_and_store_mcp_token(&mut self) -> Option<String> {
        let ps = "$b=New-Object byte[] 32; \
                  [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($b); \
                  ($b | ForEach-Object { $_.ToString('x2') }) -join ''";
        let out = quiet_cmd("powershell")
            .args(["-NoProfile", "-Command", ps])
            .output()
            .ok()?;
        let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if token.len() < 32 {
            self.log_debug(
                "[daemon] ERRO: nao consegui gerar o token do endpoint MCP (saida vazia do gerador).",
            );
            return None;
        }
        // setx persiste em HKCU\Environment (limite de 1024 chars — 64 cabe).
        let ok = quiet_cmd("setx")
            .args(["CUA_DRIVER_RS_MCP_HTTP_TOKEN", &token])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            self.mcp_token = token.clone();
            self.log_debug(
                "[daemon] Token do endpoint MCP GERADO e gravado em HKCU\\Environment (o motor 0.16+ recusa iniciar sem ele). O valor nao e exibido nem registrado em log — trate-o como senha; leia com: reg query HKCU\\Environment /v CUA_DRIVER_RS_MCP_HTTP_TOKEN",
            );
            Some(token)
        } else {
            self.log_debug(
                "[daemon] AVISO: token gerado mas NAO persistido (setx falhou). Ele vale so para este daemon; no proximo logon o autostart subira sem token.",
            );
            self.mcp_token = token.clone();
            Some(token)
        }
    }

    /// Arquivo onde o daemon do motor escreve stdout+stderr. Fica ao lado dos
    /// artefatos de update, em %TEMP%, para não exigir permissão especial.
    pub fn engine_log_path() -> std::path::PathBuf {
        Self::update_dir().join("cua-driver-serve.log")
    }

    /// `tail -f` do log do motor: lê apenas o que chegou desde a última leitura
    /// (posição guardada em `engine_log_pos`) e joga no console da GUI. É assim
    /// que a atividade de clientes MCP EXTERNOS (conector do Claude, Antigravity,
    /// Cursor…) aparece — ela nunca passa por `run_logged`, que só registra o
    /// que a própria GUI executa.
    pub fn poll_engine_log(&mut self) {
        let now = std::time::Instant::now();
        if let Some(last) = self.engine_log_poll {
            if now.duration_since(last).as_millis() < 700 {
                return;
            }
        }
        self.engine_log_poll = Some(now);

        let path = Self::engine_log_path();
        let Ok(meta) = std::fs::metadata(&path) else {
            return;
        };
        let len = meta.len();
        // Arquivo encolheu = daemon reiniciado e log truncado: recomeça do zero.
        if len < self.engine_log_pos {
            self.engine_log_pos = 0;
        }
        if len == self.engine_log_pos {
            return;
        }

        use std::io::{Read, Seek, SeekFrom};
        let Ok(mut f) = std::fs::File::open(&path) else {
            return;
        };
        if f.seek(SeekFrom::Start(self.engine_log_pos)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if f.read_to_end(&mut buf).is_err() {
            return;
        }
        self.engine_log_pos = len;
        let text = String::from_utf8_lossy(&buf);
        for line in text.lines() {
            let line = line.trim_end();
            if !line.is_empty() {
                self.log_debug(&format!("[motor] {}", line));
            }
        }
    }

    /// Valor de uma variável de ambiente do USUÁRIO (HKCU\Environment) — a
    /// fonte que o instalador semeia e que o processo já aberto não herda.
    #[cfg(target_os = "windows")]
    fn read_user_env(name: &str) -> Option<String> {
        let out = quiet_cmd("reg")
            .args(["query", "HKCU\\Environment", "/v", name])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        for line in text.lines() {
            if line.contains(name) {
                if let Some(pos) = line.find("REG_SZ") {
                    let v = line[pos + "REG_SZ".len()..].trim();
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
        None
    }

    pub fn stop_daemon(&mut self) {
        let exe = Self::engine_exe();
        let _ = self.run_logged(&exe, &["stop"]);
        self.check_port_status();
        self.daemon_running = self.port_active;
    }

    pub fn run_doctor(&mut self) {
        match self.run_logged("cua-driver", &["doctor"]) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.status_msg = if !stdout.is_empty() {
                    stdout
                } else if !stderr.is_empty() {
                    stderr
                } else {
                    format!("doctor: exit {:?} (sem saida)", out.status.code())
                };
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver doctor' (esta no PATH?)."
                        .to_string();
            }
        }
    }

    fn run_skills(&mut self, action: &str) {
        match self.run_logged("cua-driver", &["skills", action]) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.status_msg = if !stdout.is_empty() {
                    stdout
                } else if !stderr.is_empty() {
                    stderr
                } else {
                    format!("skills {}: exit {:?} (sem saida)", action, out.status.code())
                };
            }
            None => {
                self.status_msg =
                    "ERRO: nao foi possivel executar 'cua-driver skills' (esta no PATH?)."
                        .to_string();
            }
        }
    }

    pub fn install_skills(&mut self) {
        self.run_skills("install");
    }

    pub fn update_skills(&mut self) {
        self.run_skills("update");
    }

    pub fn uninstall_skills(&mut self) {
        self.run_skills("uninstall");
    }

    /// Reinicia a task de autostart do daemon.
    pub fn kick_autostart(&mut self) {
        let _ = self.run_logged("cua-driver", &["autostart", "kick"]);
        self.check_port_status();
        self.daemon_running = self.port_active;
    }

    /// Invoca qualquer tool MCP do CUA via CLI e captura o resultado.
    pub fn call_mcp_tool(&mut self, tool_name: &str, extra_args: &[&str]) {
        let mut args = vec!["call", tool_name];
        args.extend_from_slice(extra_args);
        match self.run_logged("cua-driver", &args) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.status_msg = if !stdout.is_empty() {
                    format!("[{}] OK:\n{}", tool_name, stdout)
                } else if !stderr.is_empty() {
                    format!("[{}] stderr:\n{}", tool_name, stderr)
                } else {
                    format!("[{}]: exit {:?} (sem saida)", tool_name, out.status.code())
                };
            }
            None => {
                self.status_msg = format!(
                    "ERRO: nao foi possivel executar 'cua-driver call {}' (esta no PATH?).",
                    tool_name
                );
            }
        }
    }

    /// Diretorio de staging do upgrade em %TEMP% (download + flag de pronto).
    fn update_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("fzcomputerai-update")
    }

    /// true somente se `remote` for ESTRITAMENTE mais nova que `local`
    /// (comparacao numerica major.minor.patch). Igual ou mais antiga =>
    /// false: /releases/latest e um ponteiro mutavel no GitHub e apontar
    /// para um tag antigo (rollback de release) NAO pode virar downgrade
    /// silencioso aqui.
    fn version_newer(remote: &str, local: &str) -> bool {
        fn parts(v: &str) -> [u64; 3] {
            let mut out = [0u64; 3];
            for (i, seg) in v.split('.').take(3).enumerate() {
                let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
                out[i] = digits.parse().unwrap_or(0);
            }
            out
        }
        parts(remote) > parts(local)
    }

    /// PASSO 1 do upgrade: verifica os DOIS componentes — esta GUI e o motor
    /// `cua-driver` — e já DISPARA a atualização do que estiver atrás. O botão
    /// atualizador é AÇÃO, não relatório:
    ///   - motor: automático de ponta a ponta — para o daemon antigo, aplica a
    ///     ÚLTIMA versão estável publicada (update --apply, com fallback para o
    ///     instalador oficial) e religa o autostart (daemon novo no ar);
    ///   - GUI: o instalador baixa e confere SHA256 em segundo plano; somente a
    ///     troca final pede confirmação, porque exige FECHAR este aplicativo.
    ///
    /// POR QUE O MOTOR ENTRA AQUI: a GUI é só a interface; quem faz o trabalho
    /// é o `cua-driver`. Enquanto este botão olhava apenas a GUI, o motor podia
    /// ficar dezenas de versões atrás sem ninguém perceber — e foi o que
    /// aconteceu (0.8.3 instalado contra 0.17.0 publicado). Versões novas do
    /// motor mudam contrato (por exemplo, passaram a EXIGIR token no endpoint
    /// HTTP), então saber a versão real não é cosmético: é o que evita a GUI
    /// reportar estado errado.
    pub fn check_for_updates(&mut self) {
        self.update_checked = true;
        self.check_driver_update();
        self.check_gui_update();
        if self.driver_update_available && !self.driver_updating {
            self.start_driver_update();
        }
        if self.update_available.is_some() && !self.update_downloading && !self.update_ready {
            self.start_update_download();
        }
    }

    /// Caminho RESOLVIDO do motor: `cua-driver` quando o PATH deste processo o
    /// enxerga; senão o caminho canônico do instalador oficial (junction
    /// estável entre versões). Um processo já aberto NÃO herda o PATH que o
    /// install.ps1 gravou depois — o nome puro falharia exatamente no momento
    /// que mais importa: logo após instalar/atualizar o motor.
    fn engine_exe() -> String {
        #[cfg(target_os = "windows")]
        {
            let on_path = quiet_cmd("where")
                .arg("cua-driver")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !on_path {
                if let Ok(la) = std::env::var("LOCALAPPDATA") {
                    let canon = std::path::Path::new(&la)
                        .join("Programs\\Cua\\cua-driver\\bin\\cua-driver.exe");
                    if canon.exists() {
                        return canon.display().to_string();
                    }
                }
            }
        }
        "cua-driver".to_string()
    }

    /// Escapa apóstrofos para interpolação em string single-quoted do
    /// PowerShell ('' = apóstrofo literal). Sem isso, um caminho com '
    /// (usuário "O'Neil") quebra o parse do -Command inteiro antes de
    /// qualquer linha executar — nenhuma flag é gravada e o poll espera
    /// para sempre.
    fn ps_quote(s: &str) -> String {
        s.replace('\'', "''")
    }

    /// Versão do motor + se há atualização, pela API OFICIAL do próprio
    /// `cua-driver` (`check-update --json`). Não reimplementamos a consulta de
    /// releases do motor: quem sabe onde ele publica é ele.
    ///
    /// `--no-cache` primeiro: o motor mantém cache de 20h em disco, e um botão
    /// que promete agir sobre "a última versão publicada" não pode decidir com
    /// dado de ontem. Se a versão do motor não aceitar o flag, repete sem ele.
    /// FALHA TOTAL LIMPA O ESTADO: sem resposta válida, os campos de
    /// atualização são zerados — o auto-disparo nunca age sobre dado obsoleto
    /// de uma consulta antiga (parar o daemon com base em informação velha).
    pub fn check_driver_update(&mut self) {
        let exe = Self::engine_exe();
        self.log_debug(&format!(
            "[upgrade] Consultando o motor: {} check-update --json --no-cache",
            exe
        ));
        let mut parsed: Option<serde_json::Value> = None;
        for (i, args) in [
            &["check-update", "--json", "--no-cache"][..],
            &["check-update", "--json"][..],
        ]
        .iter()
        .enumerate()
        {
            if let Some(out) = self.run_logged(&exe, args) {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(text.trim()) {
                    parsed = Some(v);
                    break;
                }
            }
            if i == 0 {
                self.log_debug(
                    "[upgrade] check-update --no-cache nao respondeu JSON valido; tentando com o cache de 20h...",
                );
            }
        }
        match parsed {
            Some(v) => {
                self.driver_version = v
                    .get("current_version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                self.driver_latest = v
                    .get("latest_version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                self.driver_update_available = v
                    .get("update_available")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                self.driver_notes_url = v
                    .get("release_notes_url")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
                    self.log_debug(&format!("[upgrade] motor reportou erro: {}", err));
                }
                self.log_debug(&format!(
                    "[upgrade] Motor cua-driver: instalado {} | ultimo {} | atualizacao disponivel: {}",
                    self.driver_version, self.driver_latest, self.driver_update_available
                ));
            }
            None => {
                // Estado DESCONHECIDO não é estado antigo: zera tudo para a UI
                // dizer "não foi possível consultar" em vez de afirmar versão
                // ou disponibilidade que não foram verificadas agora.
                self.driver_version.clear();
                self.driver_latest.clear();
                self.driver_notes_url.clear();
                self.driver_update_available = false;
                self.status_msg = self.tr(
                    "Nao foi possivel consultar o motor (cua-driver check-update). Instalado? Veja o console.",
                    "Could not query the engine (cua-driver check-update). Installed? See the console.",
                );
            }
        }
    }

    /// Versão desta GUI contra o GitHub Releases do projeto.
    pub fn check_gui_update(&mut self) {
        self.log_debug("[upgrade] Verificando novas versoes no GitHub Releases...");
        let ps_cmd = "$r = Invoke-RestMethod -Uri 'https://api.github.com/repos/RLuf/fzcomputerai/releases/latest' -Headers @{'User-Agent'='FzComputerAI'}; Write-Output $r.tag_name";
        match self.run_logged("powershell", &["-NoProfile", "-Command", ps_cmd]) {
            Some(out) if out.status.success() => {
                let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let current_ver = env!("CARGO_PKG_VERSION");
                self.log_debug(&format!(
                    "[upgrade] Versao atual: v{} | Ultima release: {}",
                    current_ver, tag
                ));
                let clean_tag = tag.trim_start_matches('v');
                if Self::version_newer(clean_tag, current_ver) {
                    self.log_debug(&format!(
                        "[upgrade] Nova versao disponivel: {}. Aguardando confirmacao para baixar em segundo plano.",
                        tag
                    ));
                    self.update_available = Some(tag);
                } else if !clean_tag.is_empty() && clean_tag != current_ver {
                    // Release "latest" mais ANTIGA que a instalada (rollback
                    // no GitHub). Informar, nunca fazer downgrade sozinho.
                    self.log_debug(&format!(
                        "[upgrade] A release marcada como latest ({}) NAO e mais nova que a versao instalada (v{}). Nenhum downgrade automatico sera feito.",
                        tag, current_ver
                    ));
                } else {
                    self.log_debug(&format!(
                        "[upgrade] Voce ja esta utilizando a versao mais recente (v{}).",
                        current_ver
                    ));
                }
            }
            Some(_) => {
                self.log_debug("[upgrade] Nao foi possivel consultar a API do GitHub Releases.");
            }
            None => {
                self.log_debug("[upgrade] Falha ao executar verificacao de atualizacoes.");
            }
        }
    }

    /// O motor está presente? Checagem pelo PATH (`where`/`which`) E pelo
    /// caminho canônico do instalador oficial — o mesmo critério de
    /// engine_exe(). Só olhar o PATH fazia o banner "motor NÃO encontrado"
    /// persistir logo após uma instalação bem-sucedida pela GUI (o processo
    /// aberto não herda o PATH novo), induzindo reinstalação em loop.
    pub fn check_driver_present(&mut self) {
        #[cfg(target_os = "windows")]
        let finder = "where";
        #[cfg(not(target_os = "windows"))]
        let finder = "which";
        let mut present = quiet_cmd(finder)
            .arg("cua-driver")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        #[cfg(target_os = "windows")]
        if !present {
            if let Ok(la) = std::env::var("LOCALAPPDATA") {
                present = std::path::Path::new(&la)
                    .join("Programs\\Cua\\cua-driver\\bin\\cua-driver.exe")
                    .exists();
            }
        }
        self.driver_present = present;
        if !self.driver_present {
            self.log_debug(
                "[motor] cua-driver NAO encontrado (nem no PATH nem no caminho canonico do instalador oficial) — nenhuma acao de automacao vai funcionar ate instalar o motor.",
            );
        }
    }

    /// Instala o MOTOR `cua-driver`, cumprindo o contrato que o instalador
    /// anuncia ao usuário ("instale o motor depois pelo próprio aplicativo").
    ///
    /// Ordem de preferência, igual à do instalador gráfico:
    ///   1. script OFICIAL embarcado em `<dir do exe>\cua-driver\install.ps1`
    ///      (auditável: veio junto no pacote e pode ser lido antes de rodar);
    ///   2. se não existir, o endpoint OFICIAL do projeto Cua
    ///      (`irm https://cua.ai/driver/install.ps1 | iex`).
    /// Nunca baixamos/instalamos o motor por conta própria — quem publica e
    /// instala o motor é o projeto Cua.
    /// Comando PowerShell do instalador OFICIAL do projeto Cua.
    ///
    /// ALVO EXPLÍCITO SEMPRE QUE CONHECIDO (`release` = `latest_version` do
    /// check-update): sem `-Release`, o script oficial NÃO consulta o GitHub —
    /// instala o `BAKED_VERSION` congelado dentro do próprio script
    /// (precedência documentada nele: env > -Release > baked > API; já
    /// flagramos um embarcado com baked 0.8.3 contra 0.17.0 publicado).
    /// Com alvo conhecido, prefere o script EMBARCADO (auditável) apontado
    /// para a versão exata; sem alvo, prefere o script do ENDPOINT oficial
    /// (baked atualizado pelo CD do Cua a cada release) com o embarcado como
    /// fallback offline.
    ///
    /// SEMPRE via ARQUIVO invocado com `&` — nunca `irm | iex`: no `iex` o
    /// script oficial roda no escopo do wrapper, e os `exit 1` dele MATAM o
    /// powershell inteiro antes de qualquer flag ser gravada (spinner eterno
    /// na UI), além do `$ErrorActionPreference='Stop'` dele vazar para o
    /// wrapper. Num script-arquivo, `exit` encerra só o script e vira
    /// $LASTEXITCODE — verificável.
    ///
    /// `no_autostart`: registrar a Scheduled Task exige admin (RunAs). No
    /// fallback da ATUALIZAÇÃO (janela oculta), um UAC "do nada" é hostil e a
    /// task normalmente já existe — o `autostart kick` religa o daemon.
    fn engine_installer_cmd(&mut self, no_autostart: bool, release: &str) -> String {
        let flag = if no_autostart { " -NoAutoStart" } else { "" };
        // Só [0-9.] chega à linha de comando — o valor vem do JSON do motor.
        let rel = if !release.is_empty()
            && release.len() <= 32
            && release.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            format!(" -Release {}", release)
        } else {
            String::new()
        };
        let embedded = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("cua-driver").join("install.ps1")))
            .filter(|p| p.exists());
        match &embedded {
            Some(p) if !rel.is_empty() => {
                self.log_debug(&format!(
                    "[motor] Instalador oficial: script embarcado {} (alvo{})",
                    p.display(),
                    rel
                ));
                format!(
                    "& '{}'{}{}",
                    Self::ps_quote(&p.display().to_string()),
                    rel,
                    flag
                )
            }
            _ => {
                let fallback = match &embedded {
                    Some(p) => format!(
                        " catch {{ & '{}'{}{} }}",
                        Self::ps_quote(&p.display().to_string()),
                        rel,
                        flag
                    ),
                    // `throw`, nunca `exit`: propaga para o try/catch do
                    // wrapper (que grava a flag de erro) sem matar o processo.
                    None => " catch { throw }".to_string(),
                };
                self.log_debug(&format!(
                    "[motor] Instalador oficial: endpoint cua.ai/driver/install.ps1 (alvo{}; fallback offline embarcado={}).",
                    if rel.is_empty() { " = baked do CD" } else { rel.as_str() },
                    embedded.is_some()
                ));
                format!(
                    "& {{ try {{ $s = Join-Path $env:TEMP 'cua-driver-install.ps1'; irm https://cua.ai/driver/install.ps1 -OutFile $s; & $s{rel}{flag} }}{fb} }}",
                    rel = rel,
                    flag = flag,
                    fb = fallback
                )
            }
        }
    }

    pub fn install_driver_engine(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            let _ = std::fs::create_dir_all(&dir);
            // Flags obsoletas são removidas AQUI, sincronamente, ANTES do
            // spawn: o poll roda no frame seguinte (~16ms) e o Remove-Item do
            // PowerShell só executa após o cold-start dele (centenas de ms) —
            // sem isso, o poll consome a flag da tentativa ANTERIOR.
            let _ = std::fs::remove_file(dir.join("drv-ready.flag"));
            let _ = std::fs::remove_file(dir.join("drv-error.flag"));
            // Alvo quando conhecido (raro aqui — normalmente o motor nem
            // existe ainda); vazio => script do endpoint oficial, baked fresco.
            let latest = self.driver_latest.clone();
            let installer_cmd = self.engine_installer_cmd(false, &latest);

            // Depois de instalar, o PATH desta sessão do PowerShell continua o
            // ANTIGO (sem o bin do motor) — por isso o `$cua` é resolvido para
            // o caminho canônico antes do kick e do check-update finais.
            // Veredito ($ok) vem do exit code do instalador; o kick roda em
            // try/catch para o erro reportado nunca ser o CommandNotFound do
            // kick mascarando a causa real ($out vai junto na flag).
            // Lock orfao: o install.ps1 oficial espera o install.lock PARA
            // SEMPRE — um lock de instalacao morta (>30 min sem escrita) e
            // removido AQUI antes de invocar o instalador, senao ele pendura.
            // Flag de erro NUNCA vazia: $out + causa, com texto minimo de
            // resgate se ambos vierem vazios (a GUI ja mostrou falha sem causa).
            let ps = format!(
                "$ErrorActionPreference='Continue'; $d='{dir}'; $out=''; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 try {{ \
                   $lk = Join-Path $env:USERPROFILE '.cua-driver/install.lock'; \
                   if ((Test-Path $lk) -and (((Get-Date) - (Get-Item $lk).LastWriteTime).TotalMinutes -gt 30)) {{ Remove-Item $lk -Force }}; \
                   $out = ({cmd} 2>&1 | Out-String); \
                   $ok = ($LASTEXITCODE -eq 0); \
                   $cua = 'cua-driver'; \
                   if (-not (Get-Command $cua -ErrorAction SilentlyContinue)) {{ \
                     $cc = Join-Path $env:LOCALAPPDATA 'Programs\\Cua\\cua-driver\\bin\\cua-driver.exe'; \
                     if (Test-Path $cc) {{ $cua = $cc }} \
                   }}; \
                   try {{ & $cua autostart kick 2>&1 | Out-Null }} catch {{}}; \
                   $ver = (& $cua check-update --json 2>&1 | Out-String); \
                   if ($ok) {{ Set-Content -Path (Join-Path $d 'drv-ready.flag') -Value ($out + \"`n\" + $ver) }} \
                   else {{ \
                     $msg = ($out + \"`n\" + $ver); \
                     if (-not $msg.Trim()) {{ $msg = 'FALHA sem saida capturada do instalador oficial (exit != 0).' }}; \
                     Set-Content -Path (Join-Path $d 'drv-error.flag') -Value $msg \
                   }} \
                 }} catch {{ \
                   $msg = ($out + \"`n\" + $_.Exception.Message); \
                   if (-not $msg.Trim()) {{ $msg = 'FALHA sem saida capturada (excecao sem mensagem).' }}; \
                   Set-Content -Path (Join-Path $d 'drv-error.flag') -Value $msg \
                 }}",
                dir = Self::ps_quote(&dir.display().to_string()),
                cmd = installer_cmd
            );
            match quiet_cmd("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    ps.as_str(),
                ])
                .spawn()
            {
                Ok(_) => {
                    self.driver_updating = true;
                    self.status_msg = self.tr(
                        "Instalando o motor cua-driver em segundo plano (instalador oficial do projeto Cua)...",
                        "Installing the cua-driver engine in the background (Cua project's official installer)...",
                    );
                }
                Err(e) => {
                    self.log_debug(&format!("[motor] ERRO ao disparar a instalacao: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.status_msg =
                "No Linux/macOS instale o motor com: curl -fsSL https://cua.ai/driver/install.sh | bash"
                    .to_string();
        }
    }

    /// Atualiza o MOTOR pelo caminho oficial dele, de ponta a ponta e sem mais
    /// cliques: PARA o daemon antigo, aplica a ÚLTIMA versão estável publicada
    /// e RELIGA o autostart (daemon novo no ar). Roda em processo DESTACADO
    /// (o download/instalação pode levar dezenas de segundos e travaria a UI).
    ///
    /// Sequência: stop -> `update --apply` -> se o subcomando não existir ou
    /// falhar (motores antigos como o 0.8.3 não têm `update`), FALLBACK para o
    /// instalador OFICIAL do projeto Cua (que resolve e instala a latest do
    /// GitHub) -> autostart kick -> check-update para registrar a versão REAL.
    /// NUNCA baixamos binário do motor por conta própria — quem publica e
    /// instala o motor é o projeto Cua.
    pub fn start_driver_update(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            let _ = std::fs::create_dir_all(&dir);
            // Pré-limpeza síncrona das flags (ver install_driver_engine).
            let _ = std::fs::remove_file(dir.join("drv-ready.flag"));
            let _ = std::fs::remove_file(dir.join("drv-error.flag"));
            let cua = Self::engine_exe();
            // -NoAutoStart no fallback: a task do daemon ja existe no cenario
            // de atualizacao e o kick a religa; sem isso o install.ps1 dispara
            // um UAC (RunAs) "do nada", sob janela oculta, sem contexto algum.
            // Alvo explicito: driver_latest acabou de vir do check-update que
            // disparou esta atualizacao (sem -Release o script instalaria o
            // baked congelado dele, nao a latest).
            let latest = self.driver_latest.clone();
            let installer_cmd = self.engine_installer_cmd(true, &latest);
            // O daemon NUNCA fica parado no caminho de falha: kick roda no
            // sucesso, no erro E no catch. O veredito ready/error vem do exit
            // code real do apply/instalador — nunca "ready" por ter chegado ao
            // fim do script (falso sucesso reportando a versao antiga).
            // Motor AUSENTE (ex.: desinstalado por migracao de layout): o
            // `& $cua stop` explodiria com CommandNotFound e o fluxo morreria
            // no catch — stop/update so rodam se o binario RESOLVER
            // (Get-Command); senao $out registra a ausencia e o fluxo vai
            // DIRETO ao instalador oficial (mesmo bloco de fallback).
            // Lock orfao: o install.ps1 oficial espera o install.lock PARA
            // SEMPRE — lock morto (>30 min) e removido antes do instalador.
            // Flag de erro NUNCA vazia: $out + causa, com texto minimo de
            // resgate se ambos vierem vazios.
            let ps = format!(
                "$ErrorActionPreference='Continue'; $d='{dir}'; $cua='{cua}'; $out=''; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 try {{ \
                   $has = [bool](Get-Command $cua -ErrorAction SilentlyContinue); \
                   if ($has) {{ \
                     & $cua stop 2>&1 | Out-Null; \
                     $out = (& $cua update --apply 2>&1 | Out-String); \
                     $ok = ($LASTEXITCODE -eq 0) \
                   }} else {{ \
                     $out = \"[motor ausente] cua-driver nao encontrado - indo direto ao instalador oficial (latest)...`n\"; \
                     $ok = $false \
                   }}; \
                   if (-not $ok) {{ \
                     if ($has) {{ $out += \"`n[fallback] update --apply indisponivel/falhou (exit=$LASTEXITCODE) - executando o instalador oficial (latest)...`n\" }}; \
                     $lk = Join-Path $env:USERPROFILE '.cua-driver/install.lock'; \
                     if ((Test-Path $lk) -and (((Get-Date) - (Get-Item $lk).LastWriteTime).TotalMinutes -gt 30)) {{ Remove-Item $lk -Force }}; \
                     $out += ({inst} 2>&1 | Out-String); \
                     $ok = ($LASTEXITCODE -eq 0); \
                     $cc = Join-Path $env:LOCALAPPDATA 'Programs\\Cua\\cua-driver\\bin\\cua-driver.exe'; \
                     if (Test-Path $cc) {{ $cua = $cc }} \
                   }}; \
                   try {{ & $cua autostart kick 2>&1 | Out-Null }} catch {{}}; \
                   $ver = (& $cua check-update --json 2>&1 | Out-String); \
                   if ($ok) {{ Set-Content -Path (Join-Path $d 'drv-ready.flag') -Value ($out + \"`n\" + $ver) }} \
                   else {{ \
                     $msg = ($out + \"`n\" + $ver); \
                     if (-not $msg.Trim()) {{ $msg = 'FALHA sem saida capturada do update/instalador oficial (exit != 0).' }}; \
                     Set-Content -Path (Join-Path $d 'drv-error.flag') -Value $msg \
                   }} \
                 }} catch {{ \
                   try {{ & $cua autostart kick 2>&1 | Out-Null }} catch {{}}; \
                   $msg = ($out + \"`n\" + $_.Exception.Message); \
                   if (-not $msg.Trim()) {{ $msg = 'FALHA sem saida capturada (excecao sem mensagem).' }}; \
                   Set-Content -Path (Join-Path $d 'drv-error.flag') -Value $msg \
                 }}",
                dir = Self::ps_quote(&dir.display().to_string()),
                cua = Self::ps_quote(&cua),
                inst = installer_cmd
            );
            match quiet_cmd("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    ps.as_str(),
                ])
                .spawn()
            {
                Ok(_) => {
                    self.driver_updating = true;
                    self.status_msg = self.tr(
                        "Atualizando o motor cua-driver em segundo plano (daemon antigo parado; o novo sobe ao final)...",
                        "Updating the cua-driver engine in the background (old daemon stopped; the new one starts when done)...",
                    );
                    self.log_debug(
                        "[upgrade] Atualizacao do motor disparada em segundo plano (stop -> update --apply, fallback instalador oficial -> autostart kick).",
                    );
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[upgrade] ERRO ao disparar a atualizacao do motor: {}",
                        e
                    ));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Fora do Windows o motor tambem tem atualizador proprio; aqui a
            // chamada e direta (sem o wrapper de autostart, que e Windows-only).
            let _ = self.run_logged("cua-driver", &["update", "--apply"]);
            self.check_driver_update();
        }
    }

    /// Observa a atualização do motor (flags em %TEMP%) e, ao terminar, relê a
    /// versão REAL — o estado exibido nunca é presumido a partir do "mandei
    /// atualizar".
    pub fn poll_driver_update(&mut self) {
        if !self.driver_updating {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.driver_update_poll {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.driver_update_poll = Some(now);

        let dir = Self::update_dir();
        let ready = dir.join("drv-ready.flag");
        let err = dir.join("drv-error.flag");
        if err.exists() {
            let msg = std::fs::read_to_string(&err).unwrap_or_default();
            let _ = std::fs::remove_file(&err);
            self.driver_updating = false;
            self.log_debug(&format!(
                "[upgrade] FALHA ao atualizar o motor: {}",
                msg.trim()
            ));
            // Também no ERRO o estado real é relido: o script tenta religar o
            // daemon (kick) mesmo falhando, e a UI precisa refletir se o motor
            // sobreviveu — não congelar no estado de antes da tentativa.
            self.check_driver_present();
            self.check_driver_update();
            self.check_port_status();
            self.daemon_running = self.port_active;
            self.status_msg = format!("Falha ao atualizar o motor: {}", tail_str(msg.trim(), 300));
        } else if ready.exists() {
            let info = std::fs::read_to_string(&ready).unwrap_or_default();
            let _ = std::fs::remove_file(&ready);
            self.driver_updating = false;
            self.log_debug(&format!(
                "[upgrade] Atualizacao do motor concluida. Saida:\n{}",
                tail_str(info.trim(), 1500)
            ));
            // Verdade final: reler presenca, versao e estado do endpoint —
            // nada e presumido a partir de "mandei instalar/atualizar".
            self.check_driver_present();
            self.check_driver_update();
            self.check_port_status();
            self.daemon_running = self.port_active;
            let v = self.driver_version.clone();
            // "Atualizado" só se a releitura CONFIRMAR que não há mais versão
            // nova — a flag ready diz que o processo terminou, não que deu certo.
            if self.driver_update_available && !self.driver_latest.is_empty() {
                let l = self.driver_latest.clone();
                self.status_msg = format!(
                    "{} {} -> {} ({})",
                    self.tr(
                        "Atualizacao do motor NAO foi aplicada:",
                        "Engine update was NOT applied:"
                    ),
                    v,
                    l,
                    self.tr("detalhes no console", "details in the console")
                );
            } else {
                self.status_msg = format!(
                    "{} {}",
                    self.tr("Motor atualizado. Versao agora:", "Engine updated. Version now:"),
                    v
                );
            }
        }
    }

    /// PASSO 2: download do instalador em PROCESSO SEPARADO (a UI nao trava).
    /// O processo grava ready.flag ao terminar; poll_update_download observa.
    pub fn start_update_download(&mut self) {
        let Some(tag) = self.update_available.clone() else {
            return;
        };
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            let _ = std::fs::create_dir_all(&dir);
            // Pré-limpeza SÍNCRONA das flags da tentativa anterior: o poll
            // roda no frame seguinte e o Remove-Item do PowerShell só executa
            // centenas de ms depois — sem isso, um error.flag/ready.flag velho
            // e consumido na hora e o retry nasce morto (ou dispara o dialogo
            // de instalar com download ainda em andamento).
            let _ = std::fs::remove_file(dir.join("ready.flag"));
            let _ = std::fs::remove_file(dir.join("error.flag"));
            // Baixa o instalador E o .sha256 publicado pelo CI, confere o
            // hash (Get-FileHash) e SO grava ready.flag com hash conferido.
            // Divergencia => error.flag + instalador apagado: executavel
            // baixado sem integridade conferida nunca roda (SIGNING.md §3).
            let ps = format!(
                "$ErrorActionPreference='Stop'; \
                 $d = '{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'ready.flag'),(Join-Path $d 'error.flag') -Force -ErrorAction SilentlyContinue; \
                 $t = Join-Path $d 'fzcomputerai-setup-windows-x64.exe'; \
                 try {{ \
                   Invoke-WebRequest -Uri 'https://github.com/RLuf/fzcomputerai/releases/download/{tag}/fzcomputerai-setup-windows-x64.exe' -OutFile $t -UseBasicParsing; \
                   Invoke-WebRequest -Uri 'https://github.com/RLuf/fzcomputerai/releases/download/{tag}/fzcomputerai-setup-windows-x64.exe.sha256' -OutFile \"$t.sha256\" -UseBasicParsing; \
                   $expected = ((Get-Content \"$t.sha256\" -TotalCount 1) -split '\\s+')[0].ToLower(); \
                   $actual = (Get-FileHash -Path $t -Algorithm SHA256).Hash.ToLower(); \
                   if ($expected -ne $actual) {{ throw \"SHA256 nao confere: esperado $expected, obtido $actual\" }}; \
                   New-Item -ItemType File -Force -Path (Join-Path $d 'ready.flag') | Out-Null \
                 }} catch {{ \
                   Set-Content -Path (Join-Path $d 'error.flag') -Value $_.Exception.Message; \
                   Remove-Item $t -Force -ErrorAction SilentlyContinue \
                 }}",
                dir = Self::ps_quote(&dir.display().to_string()),
                tag = Self::ps_quote(&tag)
            );
            match quiet_cmd("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    ps.as_str(),
                ])
                .spawn()
            {
                Ok(_) => {
                    self.update_downloading = true;
                    self.log_debug(&format!(
                        "[upgrade] Download do instalador {} iniciado em SEGUNDO PLANO para {}.",
                        tag,
                        dir.display()
                    ));
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[upgrade] ERRO ao iniciar o download em background: {}",
                        e
                    ));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.log_debug("[upgrade] Auto-upgrade disponivel apenas no Windows.");
        }
    }

    /// PASSO 3: observado a cada ~1s pelo loop da UI enquanto ha download em
    /// andamento. Quando ready.flag existe E o .exe esta la, marca pronto —
    /// dai o dialogo pede para FECHAR e instalar.
    pub fn poll_update_download(&mut self) {
        if !self.update_downloading || self.update_ready {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.last_update_poll {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.last_update_poll = Some(now);

        let dir = Self::update_dir();
        let flag = dir.join("ready.flag");
        let err_flag = dir.join("error.flag");
        let setup = dir.join("fzcomputerai-setup-windows-x64.exe");

        if err_flag.exists() {
            let msg = std::fs::read_to_string(&err_flag).unwrap_or_default();
            let _ = std::fs::remove_file(&err_flag);
            self.update_downloading = false;
            // update_available fica: a verificacao ACABOU de encontrar update;
            // so o download falhou. Zerar aqui fazia a Central exibir
            // "atualizada" (verde) depois de uma falha — afirmacao falsa — e
            // escondia o botao de tentar de novo.
            self.log_debug(&format!(
                "[upgrade] FALHA no download/verificacao do instalador: {}",
                msg.trim()
            ));
            self.status_msg = format!(
                "{} {}",
                self.tr(
                    "Falha no download/verificacao do instalador:",
                    "Installer download/verification failed:"
                ),
                tail_str(msg.trim(), 300)
            );
            return;
        }

        if flag.exists() && setup.exists() {
            let _ = std::fs::remove_file(&flag);
            self.update_downloading = false;
            self.update_ready = true;
            self.log_debug(&format!(
                "[upgrade] Download concluido e SHA256 conferido: {}. Aguardando confirmacao para FECHAR e instalar.",
                setup.display()
            ));
        }
    }

    /// PASSO 4: dispara o processo de instalacao em background e retorna.
    /// O chamador (UI) fecha o aplicativo em seguida. O processo externo:
    /// espera este exe sair (ate 30s), GARANTE encerrado (Stop-Process),
    /// encerra o cua-driver, instala /VERYSILENT e reabre a GUI + motor
    /// (autostart kick). Nada de "instalar por cima" com o app aberto.
    pub fn install_update_and_restart(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
            let current_exe = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let ps = format!(
                "$d = '{dir}'; \
                 $t = Join-Path $d 'fzcomputerai-setup-windows-x64.exe'; \
                 if (-not (Test-Path $t)) {{ exit 1 }}; \
                 $deadline = (Get-Date).AddSeconds(30); \
                 while ((Get-Process fzcomputerai -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {{ Start-Sleep -Milliseconds 500 }}; \
                 Stop-Process -Name fzcomputerai -Force -ErrorAction SilentlyContinue; \
                 Stop-Process -Name cua-driver -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Seconds 1; \
                 Start-Process -FilePath $t -ArgumentList '/VERYSILENT /NORESTART' -Wait; \
                 $exe = Join-Path $env:LOCALAPPDATA 'Programs\\FzComputerAI\\fzcomputerai.exe'; \
                 if (-not (Test-Path $exe)) {{ $exe = '{cur}' }}; \
                 if (Test-Path $exe) {{ Start-Process -FilePath $exe }}; \
                 & '{cua}' autostart kick",
                dir = Self::ps_quote(&dir.display().to_string()),
                cur = Self::ps_quote(&current_exe),
                cua = Self::ps_quote(&Self::engine_exe())
            );
            match quiet_cmd("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    ps.as_str(),
                ])
                .spawn()
            {
                Ok(_) => {
                    self.log_debug(
                        "[upgrade] Instalador disparado em background — o aplicativo sera fechado para concluir a atualizacao.",
                    );
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[upgrade] ERRO ao disparar a instalacao: {}",
                        e
                    ));
                }
            }
        }
    }

    /// Lê no registro se o autostart (HKCU\...\Run\FzComputerAI) está ativo.
    #[cfg(target_os = "windows")]
    pub fn read_autostart(&mut self) {
        let enabled = self
            .run_logged(
                "reg",
                &[
                    "query",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "FzComputerAI",
                ],
            )
            .map(|o| o.status.success())
            .unwrap_or(false);
        self.autostart_enabled = enabled;
    }

    /// Ativa/desativa "Iniciar com o Windows" via reg add/delete e relê o
    /// estado real do registro para o checkbox refletir a verdade.
    #[cfg(target_os = "windows")]
    pub fn set_autostart(&mut self, enable: bool) {
        if enable {
            match std::env::current_exe() {
                Ok(exe) => {
                    let value = format!("\"{}\"", exe.display());
                    let _ = self.run_logged(
                        "reg",
                        &[
                            "add",
                            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                            "/v",
                            "FzComputerAI",
                            "/t",
                            "REG_SZ",
                            "/d",
                            value.as_str(),
                            "/f",
                        ],
                    );
                }
                Err(e) => {
                    self.log_debug(&format!(
                        "[autostart] ERRO: current_exe() falhou: {}",
                        e
                    ));
                }
            }
        } else {
            let _ = self.run_logged(
                "reg",
                &[
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "FzComputerAI",
                    "/f",
                ],
            );
        }
        self.read_autostart();
        let status = if self.autostart_enabled { "ATIVADO" } else { "DESATIVADO" };
        self.log_debug(&format!("[autostart] Iniciar com o Windows: {}", status));
    }

    // ═══════════════════════════════════════════════════════════════════
    //  ABA TÚNEL — expõe o MCP HTTP local (127.0.0.1:porta) na internet.
    //  Objetivo: URL HTTPS pública -> HTTP local. Um túnel por vez.
    // ═══════════════════════════════════════════════════════════════════

    /// Diretório de staging dos túneis em %TEMP%. O caminho é usado como
    /// arquivo de log dos CLIs (--logfile/--log/-E) e por isso vira o
    /// MARCADOR de identidade na command line do processo (usado para matar
    /// apenas os túneis NOSSOS, nunca um cloudflared/ssh alheio do usuário).
    fn tunnel_dir() -> std::path::PathBuf {
        std::env::temp_dir().join("fzcomputerai-tunnel")
    }

    fn provider_slug(p: TunnelProvider) -> &'static str {
        match p {
            TunnelProvider::Cloudflare => "cloudflare",
            TunnelProvider::Ngrok => "ngrok",
            TunnelProvider::Ssh => "ssh",
        }
    }

    fn provider_image(p: TunnelProvider) -> &'static str {
        match p {
            TunnelProvider::Cloudflare => "cloudflared.exe",
            TunnelProvider::Ngrok => "ngrok.exe",
            TunnelProvider::Ssh => "ssh.exe",
        }
    }

    /// Caminho do log combinado (stdout+stderr) do túnel atual.
    fn tunnel_log_path(&self) -> std::path::PathBuf {
        let name = format!(
            "{}-{}.log",
            Self::provider_slug(self.tunnel_provider),
            self.tunnel_run_id
        );
        Self::tunnel_dir().join(name)
    }

    /// Diretório onde a GUI guarda binários baixados (cloudflared/ngrok):
    /// ao lado do executável ({app}\tunnel) e, se não for gravável,
    /// %LOCALAPPDATA%\FzComputerAI\tunnel.
    fn tunnel_bin_dir() -> std::path::PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                return parent.join("tunnel");
            }
        }
        std::env::temp_dir().join("fzcomputerai-tunnel-bin")
    }

    /// Preferência de gravação: {app}\tunnel se gravável, senão
    /// %LOCALAPPDATA%\FzComputerAI\tunnel.
    fn tunnel_download_dir() -> std::path::PathBuf {
        let primary = Self::tunnel_bin_dir();
        if std::fs::create_dir_all(&primary).is_ok() {
            // Testa gravabilidade real com um arquivo temporário.
            let probe = primary.join(".w");
            if std::fs::write(&probe, b"x").is_ok() {
                let _ = std::fs::remove_file(&probe);
                return primary;
            }
        }
        let fallback = std::env::var("LOCALAPPDATA")
            .map(|p| std::path::PathBuf::from(p).join("FzComputerAI").join("tunnel"))
            .unwrap_or_else(|_| std::env::temp_dir().join("fzcomputerai-tunnel-bin"));
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    }

    /// Detecta cloudflared/ngrok/ssh: primeiro no PATH (where/which), depois
    /// em {app}\tunnel, e o ssh também no OpenSSH do Windows. LAZY: roda na
    /// 1ª abertura da aba, nunca no startup.
    pub fn detect_tunnel_bins(&mut self) {
        self.tunnel_cf_bin = self.resolve_bin("cloudflared", "cloudflared.exe");
        self.tunnel_ngrok_bin = self.resolve_bin("ngrok", "ngrok.exe");
        let mut ssh = self.resolve_bin("ssh", "ssh.exe");
        #[cfg(target_os = "windows")]
        if ssh.is_empty() {
            let sys = r"C:\Windows\System32\OpenSSH\ssh.exe";
            if std::path::Path::new(sys).exists() {
                ssh = sys.to_string();
            }
        }
        self.tunnel_ssh_bin = ssh;
        self.tunnel_bins_checked = true;
        self.log_debug(&format!(
            "[tunnel] Binarios detectados -> cloudflared: {} | ngrok: {} | ssh: {}",
            if self.tunnel_cf_bin.is_empty() { "NAO ENCONTRADO" } else { &self.tunnel_cf_bin },
            if self.tunnel_ngrok_bin.is_empty() { "NAO ENCONTRADO" } else { &self.tunnel_ngrok_bin },
            if self.tunnel_ssh_bin.is_empty() { "NAO ENCONTRADO" } else { &self.tunnel_ssh_bin },
        ));
    }

    /// Resolve um binário: PATH (where/which) e depois {app}\tunnel\<exe>.
    fn resolve_bin(&mut self, cmd: &str, exe: &str) -> String {
        #[cfg(target_os = "windows")]
        let finder = "where";
        #[cfg(not(target_os = "windows"))]
        let finder = "which";

        if let Ok(out) = quiet_cmd(finder).arg(cmd).output() {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(first) = text.lines().map(|l| l.trim()).find(|l| !l.is_empty()) {
                    return first.to_string();
                }
            }
        }
        let local = Self::tunnel_bin_dir().join(exe);
        if local.exists() {
            return local.display().to_string();
        }
        String::new()
    }

    /// Lê a config da aba Túnel de HKCU\Software\FzComputerAI (valores
    /// "tunnelcfg:*"). Parser corta em "REG_SZ" — caminhos têm espaço, então
    /// split_whitespace().last() estragaria o valor.
    pub fn read_tunnel_cfg(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let out = match self
                .run_logged("reg", &["query", r"HKCU\Software\FzComputerAI"])
            {
                Some(o) if o.status.success() => o,
                _ => return,
            };
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            for line in text.lines() {
                let trimmed = line.trim();
                let Some(rest) = trimmed.strip_prefix("tunnelcfg:") else {
                    continue;
                };
                // rest = "<chave>    REG_SZ    <valor>"
                let Some(pos) = rest.find("REG_SZ") else { continue };
                let key = rest[..pos].trim();
                let value = rest[pos + "REG_SZ".len()..].trim().to_string();
                match key {
                    "provider" => {
                        self.tunnel_provider = match value.as_str() {
                            "ngrok" => TunnelProvider::Ngrok,
                            "ssh" => TunnelProvider::Ssh,
                            _ => TunnelProvider::Cloudflare,
                        }
                    }
                    "cf_token_file" => self.tunnel_cf_token_file = value,
                    "public_url" => self.tunnel_public_url = value,
                    "ngrok_extra" => self.tunnel_ngrok_extra = value,
                    "ngrok_use_policy" => self.tunnel_ngrok_use_policy = value != "0",
                    "ssh_target" => {
                        if !value.is_empty() {
                            self.tunnel_ssh_target = value
                        }
                    }
                    "ssh_remote_port" => {
                        if !value.is_empty() {
                            self.tunnel_ssh_remote_port = value
                        }
                    }
                    "ssh_key" => self.tunnel_ssh_key = value,
                    "ssh_extra" => self.tunnel_ssh_extra = value,
                    _ => {}
                }
            }
        }
    }

    /// Grava a config da aba (só no clique em "Salvar configuração" — gravar
    /// por keystroke faria um reg.exe por frame). NUNCA grava segredo: só o
    /// CAMINHO do token-file do Cloudflare, jamais o token.
    pub fn save_tunnel_cfg(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let provider = Self::provider_slug(self.tunnel_provider).to_string();
            let pairs: Vec<(&str, String)> = vec![
                ("provider", provider),
                ("cf_token_file", self.tunnel_cf_token_file.clone()),
                ("public_url", self.tunnel_public_url.clone()),
                ("ngrok_extra", self.tunnel_ngrok_extra.clone()),
                (
                    "ngrok_use_policy",
                    if self.tunnel_ngrok_use_policy { "1" } else { "0" }.to_string(),
                ),
                ("ssh_target", self.tunnel_ssh_target.clone()),
                ("ssh_remote_port", self.tunnel_ssh_remote_port.clone()),
                ("ssh_key", self.tunnel_ssh_key.clone()),
                ("ssh_extra", self.tunnel_ssh_extra.clone()),
            ];
            for (k, v) in pairs {
                let name = format!("tunnelcfg:{}", k);
                let _ = self.run_logged(
                    "reg",
                    &[
                        "add",
                        r"HKCU\Software\FzComputerAI",
                        "/v",
                        name.as_str(),
                        "/t",
                        "REG_SZ",
                        "/d",
                        v.as_str(),
                        "/f",
                    ],
                );
            }
            self.log_debug("[tunnel] Configuracao salva em HKCU\\Software\\FzComputerAI (tunnelcfg:*).");
        }
    }

    /// Registra em HKCU (auditoria) que os termos do ngrok foram aceitos.
    pub fn save_ngrok_tos_accepted(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|_| "1".to_string());
            let _ = self.run_logged(
                "reg",
                &[
                    "add",
                    r"HKCU\Software\FzComputerAI",
                    "/v",
                    "tunnelcfg:ngrok_tos_accepted",
                    "/t",
                    "REG_SZ",
                    "/d",
                    stamp.as_str(),
                    "/f",
                ],
            );
        }
    }

    /// Gera uma string alfanumérica pseudoaleatória sem crate `rand`
    /// (xorshift semeado pelo relógio). Boa o bastante para senha-de-URL e
    /// para o run_id — não é material criptográfico de produção.
    fn gen_token(len: usize) -> String {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            ^ (std::process::id() as u64).wrapping_mul(0x2545F4914F6CDD1D);
        if seed == 0 {
            seed = 0x9E3779B97F4A7C15;
        }
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            // xorshift64
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            out.push(CHARS[(seed % CHARS.len() as u64) as usize] as char);
        }
        out
    }

    /// Gera a senha do gate (16 chars) no campo da UI.
    pub fn tunnel_generate_password(&mut self) {
        self.tunnel_gate_password = Self::gen_token(16);
    }

    // ─── Gate local (nível 1 de auth): senha na URL ─────────────────────
    // O driver MCP executa POST em QUALQUER path (não valida caminho), e o
    // quick tunnel do Cloudflare / SSH público não têm auth de borda. Logo,
    // "senha na URL" só é real com um porteiro no meio: este gate é um
    // mini reverse-proxy em 127.0.0.1 que exige /s/<senha>/ no path antes de
    // encaminhar ao MCP. Sem senha, o túnel aponta direto no MCP.
    //
    // EXCEÇÃO CONSCIENTE à convenção "sem threads" do crate: um servidor não
    // é implementável por poll de arquivo. A thread morre com o app (o
    // AtomicBool + uma conexão dummy destravam o accept), então o gate nunca
    // sobrevive à GUI — coerente com o ciclo de vida dos túneis.

    /// Sobe o gate em 127.0.0.1:porta-efêmera e retorna a porta. O túnel
    /// deve então apontar para ela em vez da porta do MCP.
    fn start_gate(&mut self, mcp_port: u16, password: &str) -> Option<u16> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let listener = match std::net::TcpListener::bind(("127.0.0.1", 0u16)) {
            Ok(l) => l,
            Err(e) => {
                self.log_debug(&format!("[tunnel][gate] Falha ao abrir o porteiro local: {}", e));
                return None;
            }
        };
        let gate_port = listener.local_addr().ok()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let password = password.to_string();

        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }
                match conn {
                    Ok(client) => {
                        let pw = password.clone();
                        std::thread::spawn(move || {
                            gate_handle_conn(client, mcp_port, &pw);
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        self.tunnel_gate_stop = Some(stop);
        self.tunnel_gate_port = Some(gate_port);
        self.log_debug(&format!(
            "[tunnel][gate] Porteiro de senha ativo em 127.0.0.1:{} -> MCP 127.0.0.1:{} (exige /s/<senha>/).",
            gate_port, mcp_port
        ));
        Some(gate_port)
    }

    /// Encaminhamento LAN **dentro do próprio app** — substitui o
    /// `netsh interface portproxy`.
    ///
    /// POR QUE ISTO EXISTE (não volte para o netsh): o portproxy é uma regra
    /// ESTÁTICA do serviço IP Helper. Ela (a) exige admin/UAC para criar e
    /// remover, (b) continua "LISTENING" na LAN mesmo com o motor morto —
    /// aceitando conexões que morrem no destino, dando falso positivo de
    /// serviço no ar — e (c) SOBREVIVE ao fechamento do app e ao reboot, o que
    /// obrigava uma rotina de limpeza que o Defender flagrou como malware.
    /// Um forwarder em thread resolve os três: sem elevação, some junto com o
    /// processo e só escuta enquanto o app está vivo.
    ///
    /// Escuta em `<ip_lan>:porta` e copia bytes nos dois sentidos contra
    /// `127.0.0.1:porta` (onde o motor escuta, com bind fixo no upstream).
    fn start_lan_forward(&mut self, ip: &str, port: u16) -> bool {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        self.stop_lan_forward();

        let listener = match std::net::TcpListener::bind((ip, port)) {
            Ok(l) => l,
            Err(e) => {
                self.log_debug(&format!(
                    "[lan] Falha ao escutar em {}:{} — {}. (Porta ocupada? Regra portproxy antiga ainda ativa nesse endereco?)",
                    ip, port, e
                ));
                return false;
            }
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();

        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }
                match conn {
                    Ok(client) => {
                        std::thread::spawn(move || {
                            // Destino: o motor, sempre em loopback.
                            let Ok(upstream) = std::net::TcpStream::connect(("127.0.0.1", port))
                            else {
                                return;
                            };
                            let (Ok(mut c_in), Ok(mut u_out)) =
                                (client.try_clone(), upstream.try_clone())
                            else {
                                return;
                            };
                            let mut c_out = client;
                            let mut u_in = upstream;
                            // Um sentido em cada thread; ao fechar um lado, a
                            // cópia do outro termina com EOF/erro e sai.
                            let t = std::thread::spawn(move || {
                                let _ = std::io::copy(&mut c_in, &mut u_out);
                            });
                            let _ = std::io::copy(&mut u_in, &mut c_out);
                            let _ = t.join();
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        self.lan_forward_stop = Some(stop);
        self.lan_forward_addr = Some((ip.to_string(), port));
        self.log_debug(&format!(
            "[lan] Encaminhamento ATIVO dentro do app: {}:{} -> 127.0.0.1:{}. Sem netsh, sem admin, e some quando o app fecha.",
            ip, port, port
        ));
        true
    }

    /// Derruba o encaminhamento LAN (destrava o accept com conexão dummy).
    fn stop_lan_forward(&mut self) {
        use std::sync::atomic::Ordering;
        if let Some(stop) = self.lan_forward_stop.take() {
            stop.store(true, Ordering::SeqCst);
            if let Some((ip, port)) = self.lan_forward_addr.clone() {
                let _ = std::net::TcpStream::connect((ip.as_str(), port));
            }
            self.log_debug("[lan] Encaminhamento encerrado.");
        }
        self.lan_forward_addr = None;
    }

    /// Encerra o gate: sinaliza a thread e destrava o accept com uma conexão
    /// dummy no próprio porteiro.
    fn stop_gate(&mut self) {
        use std::sync::atomic::Ordering;
        if let Some(stop) = self.tunnel_gate_stop.take() {
            stop.store(true, Ordering::SeqCst);
            if let Some(port) = self.tunnel_gate_port {
                let _ = std::net::TcpStream::connect(("127.0.0.1", port));
            }
            self.log_debug("[tunnel][gate] Porteiro de senha encerrado.");
        }
        self.tunnel_gate_port = None;
    }

    /// Monta (programa, args) do túnel para a porta LOCAL alvo (MCP direto ou
    /// gate). Retorna None se faltar binário/config essencial (já logando).
    fn tunnel_cmdline(&mut self, local_port: u16) -> Option<(String, Vec<String>)> {
        let dir = Self::tunnel_dir();
        let _ = std::fs::create_dir_all(&dir);
        let log = self.tunnel_log_path();
        let log_s = log.display().to_string();
        let target = format!("http://127.0.0.1:{}", local_port);

        match self.tunnel_provider {
            TunnelProvider::Cloudflare => {
                if self.tunnel_cf_bin.is_empty() {
                    self.status_msg = self.tr(
                        "cloudflared nao encontrado. Use 'Baixar cloudflared' ou instale e clique em 'Detectar binarios'.",
                        "cloudflared not found. Use 'Download cloudflared' or install it and click 'Detect binaries'.",
                    );
                    return None;
                }
                let bin = self.tunnel_cf_bin.clone();
                if self.tunnel_cf_token_file.trim().is_empty() {
                    // Quick tunnel (sem conta).
                    Some((
                        bin,
                        vec![
                            "--no-autoupdate".into(),
                            "--loglevel".into(),
                            "info".into(),
                            "--logfile".into(),
                            log_s,
                            "tunnel".into(),
                            "--url".into(),
                            target,
                        ],
                    ))
                } else {
                    // Túnel nomeado via token-file (o segredo nunca vai no argv logado).
                    Some((
                        bin,
                        vec![
                            "--no-autoupdate".into(),
                            "--logfile".into(),
                            log_s,
                            "tunnel".into(),
                            "run".into(),
                            "--token-file".into(),
                            self.tunnel_cf_token_file.clone(),
                        ],
                    ))
                }
            }
            TunnelProvider::Ngrok => {
                if self.tunnel_ngrok_bin.is_empty() {
                    self.status_msg = self.tr(
                        "ngrok nao encontrado. Use 'Baixar ngrok' ou instale e clique em 'Detectar binarios'.",
                        "ngrok not found. Use 'Download ngrok' or install it and click 'Detect binaries'.",
                    );
                    return None;
                }
                let bin = self.tunnel_ngrok_bin.clone();
                let mut args: Vec<String> = vec![
                    "http".into(),
                    format!("127.0.0.1:{}", local_port),
                    "--log".into(),
                    log_s,
                    "--log-format".into(),
                    "logfmt".into(),
                    "--log-level".into(),
                    "info".into(),
                ];
                // basic-auth opcional via traffic policy (o segredo vai no
                // ARQUIVO da policy, nunca no argv logado).
                if self.tunnel_ngrok_use_policy {
                    if self.tunnel_ngrok_password.trim().is_empty() {
                        self.tunnel_ngrok_password = Self::gen_token(24);
                    }
                    let pw = self.tunnel_ngrok_password.clone();
                    let dir = Self::tunnel_download_dir();
                    let policy = dir.join("ngrok-policy.yml");
                    let yaml = format!(
                        "on_http_request:\n  - actions:\n      - type: basic-auth\n        config:\n          realm: FzComputerAI MCP\n          credentials:\n            - \"fz:{}\"\n",
                        pw
                    );
                    if std::fs::write(&policy, yaml).is_ok() {
                        #[cfg(target_os = "windows")]
                        {
                            let user = std::env::var("USERNAME").unwrap_or_default();
                            if !user.is_empty() {
                                let p = policy.display().to_string();
                                let grant = format!("{}:R", user);
                                let _ = self.run_logged(
                                    "icacls",
                                    &[p.as_str(), "/inheritance:r", "/grant:r", grant.as_str()],
                                );
                            }
                        }
                        args.push("--traffic-policy-file".into());
                        args.push(policy.display().to_string());
                    }
                }
                for tok in self.tunnel_ngrok_extra.split_whitespace() {
                    args.push(tok.to_string());
                }
                Some((bin, args))
            }
            TunnelProvider::Ssh => {
                if self.tunnel_ssh_bin.is_empty() {
                    self.status_msg = self.tr(
                        "ssh nao encontrado (Cliente OpenSSH do Windows).",
                        "ssh not found (Windows OpenSSH Client).",
                    );
                    return None;
                }
                let target_host = self.tunnel_ssh_target.trim().to_string();
                if target_host.is_empty() {
                    self.status_msg = self.tr(
                        "Informe o destino SSH (ex.: usuario@servidor ou nokey@localhost.run).",
                        "Enter the SSH target (e.g. user@server or nokey@localhost.run).",
                    );
                    return None;
                }
                let rport = self.tunnel_ssh_remote_port.trim();
                let rport = if rport.is_empty() { "80" } else { rport };
                let bin = self.tunnel_ssh_bin.clone();
                let mut args: Vec<String> = vec![
                    "-N".into(),
                    "-T".into(),
                    "-E".into(),
                    log_s,
                    "-o".into(),
                    "BatchMode=yes".into(),
                    "-o".into(),
                    "StrictHostKeyChecking=accept-new".into(),
                    "-o".into(),
                    "ExitOnForwardFailure=yes".into(),
                    "-o".into(),
                    "ConnectTimeout=10".into(),
                    "-o".into(),
                    "ServerAliveInterval=30".into(),
                    "-o".into(),
                    "ServerAliveCountMax=3".into(),
                ];
                if !self.tunnel_ssh_key.trim().is_empty() {
                    args.push("-i".into());
                    args.push(self.tunnel_ssh_key.trim().to_string());
                }
                args.push("-R".into());
                args.push(format!("{}:127.0.0.1:{}", rport, local_port));
                args.push(target_host);
                for tok in self.tunnel_ssh_extra.split_whitespace() {
                    args.push(tok.to_string());
                }
                Some((bin, args))
            }
        }
    }

    /// Inicia o túnel selecionado. Pré-checagens honestas antes de qualquer
    /// spawn; se houver senha, sobe o gate e o túnel aponta para ele.
    pub fn start_tunnel(&mut self) {
        if matches!(self.tunnel_status, TunnelStatus::Starting | TunnelStatus::Running) {
            self.status_msg = self.tr(
                "Ja existe um tunel ativo. Pare-o antes de iniciar outro.",
                "A tunnel is already active. Stop it before starting another.",
            );
            return;
        }

        // 1) Porta MCP confirmada de verdade (nunca publica porta morta).
        let Some(mcp_port) = self.detect_confirmed_cua_port() else {
            self.tunnel_status = TunnelStatus::Error;
            self.status_msg = self.tr(
                "O MCP local nao respondeu em 127.0.0.1. Inicie o motor na aba MCP & Rede antes de abrir o tunel.",
                "The local MCP did not answer on 127.0.0.1. Start the engine in the MCP & Network tab before opening the tunnel.",
            );
            return;
        };

        // 2) Pré-checagens específicas do provedor.
        if self.tunnel_provider == TunnelProvider::Cloudflare
            && self.tunnel_cf_token_file.trim().is_empty()
        {
            if let Ok(home) = std::env::var("USERPROFILE") {
                let cfg = std::path::PathBuf::from(home).join(".cloudflared").join("config.yaml");
                if cfg.exists() {
                    self.tunnel_status = TunnelStatus::Error;
                    self.status_msg = self.tr(
                        "O quick tunnel do Cloudflare falha quando existe ~/.cloudflared/config.yaml. Renomeie esse arquivo ou use o tunel nomeado (token-file).",
                        "Cloudflare quick tunnel fails when ~/.cloudflared/config.yaml exists. Rename that file or use the named tunnel (token-file).",
                    );
                    return;
                }
            }
        }
        if self.tunnel_provider == TunnelProvider::Ngrok {
            let bin = self.tunnel_ngrok_bin.clone();
            if !bin.is_empty() {
                let check = self.run_logged(&bin, &["config", "check"]);
                let ok = check.map(|o| o.status.success()).unwrap_or(false);
                if !ok {
                    self.tunnel_status = TunnelStatus::Error;
                    self.status_msg = self.tr(
                        "ngrok sem authtoken configurado. Rode no seu terminal: ngrok config add-authtoken <SEU_TOKEN> (crie a conta em ngrok.com).",
                        "ngrok has no authtoken configured. Run in your terminal: ngrok config add-authtoken <YOUR_TOKEN> (create the account at ngrok.com).",
                    );
                    return;
                }
            }
        }

        // 3) run_id novo + (se houver senha) gate local.
        self.tunnel_run_id = Self::gen_token(8);
        self.tunnel_gate_port = None;

        // A senha do gate DEVE ser URL-safe (unreserved: letras, dígitos e
        // - . _ ~). Se tiver espaço/#/%/acento, o cliente percent-encoda o path
        // e o gate (que compara o path cru) responderia 404 em TODA requisição,
        // com a UI ainda exibindo uma URL de aparência válida. Sanitiza aqui,
        // antes de usar — gate e tunnel_full_url leem o mesmo campo, então
        // continuam batendo.
        if !self.tunnel_gate_password.trim().is_empty() {
            let raw = self.tunnel_gate_password.trim().to_string();
            let safe: String = raw
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
                .collect();
            if safe != raw {
                self.log_debug(
                    "[tunnel][gate] Senha continha caracteres nao URL-safe; foram removidos.",
                );
                self.status_msg = self.tr(
                    "Senha ajustada para caracteres URL-safe (letras, numeros, - . _ ~).",
                    "Password reduced to URL-safe characters (letters, digits, - . _ ~).",
                );
            }
            self.tunnel_gate_password = safe;
        }

        let local_port = if self.tunnel_gate_password.trim().is_empty() {
            mcp_port
        } else {
            let pw = self.tunnel_gate_password.trim().to_string();
            match self.start_gate(mcp_port, &pw) {
                Some(gp) => gp,
                None => {
                    self.tunnel_status = TunnelStatus::Error;
                    self.status_msg = self.tr(
                        "Nao foi possivel abrir o porteiro de senha local.",
                        "Could not open the local password gate.",
                    );
                    return;
                }
            }
        };

        // 4) staging + logs zerados.
        let dir = Self::tunnel_dir();
        let _ = std::fs::create_dir_all(&dir);
        let log = self.tunnel_log_path();
        let _ = std::fs::remove_file(&log);

        let Some((bin, args)) = self.tunnel_cmdline(local_port) else {
            self.stop_gate();
            self.tunnel_status = TunnelStatus::Error;
            return;
        };

        let file = match std::fs::File::create(&log) {
            Ok(f) => f,
            Err(e) => {
                self.stop_gate();
                self.tunnel_status = TunnelStatus::Error;
                self.status_msg = format!("Falha ao criar log do tunel: {}", e);
                return;
            }
        };
        let file2 = match file.try_clone() {
            Ok(f) => f,
            Err(e) => {
                self.stop_gate();
                self.tunnel_status = TunnelStatus::Error;
                self.status_msg = format!("Falha ao preparar log do tunel: {}", e);
                return;
            }
        };

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        match quiet_cmd(&bin)
            .args(&arg_refs)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(file))
            .stderr(std::process::Stdio::from(file2))
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                self.tunnel_child = Some(child);
                self.tunnel_pid = Some(pid);
                self.tunnel_status = TunnelStatus::Starting;
                self.tunnel_exposure = None;
                if self.tunnel_cf_token_file.trim().is_empty() {
                    self.tunnel_public_url.clear();
                }
                // Log SEM o argv completo (pode conter caminho de token/chave).
                self.log_debug(&format!(
                    "[tunnel] {} iniciado (pid {}) para 127.0.0.1:{} [log: {}]",
                    Self::provider_slug(self.tunnel_provider),
                    pid,
                    local_port,
                    self.tunnel_log_path().display()
                ));
                self.status_msg = self.tr(
                    "Tunel iniciando... aguardando a URL publica aparecer no log.",
                    "Tunnel starting... waiting for the public URL to appear in the log.",
                );
                self.register_tunnel(pid, local_port);
                self.spawn_tunnel_guard(pid);
            }
            Err(e) => {
                self.stop_gate();
                self.tunnel_status = TunnelStatus::Error;
                self.status_msg = format!("Falha ao executar {}: {}", bin, e);
                self.log_debug(&format!("[tunnel] ERRO ao spawnar {}: {}", bin, e));
            }
        }
    }

    /// Registra o túnel como PROPRIEDADE nossa em HKCU (identidade forte:
    /// imagem|CreationDate|porta|run_id|modo), para reconciliação e limpeza.
    fn register_tunnel(&mut self, pid: u32, local_port: u16) {
        #[cfg(target_os = "windows")]
        {
            let image = Self::provider_image(self.tunnel_provider);
            let creation = self.pid_creation_date(pid).unwrap_or_default();
            let mode = if self.tunnel_gate_password.trim().is_empty() {
                "direct"
            } else {
                "gated"
            };
            let value = format!(
                "{}|{}|{}|{}|{}",
                image, creation, local_port, self.tunnel_run_id, mode
            );
            let name = format!("tunnel:{}:{}", Self::provider_slug(self.tunnel_provider), pid);
            let _ = self.run_logged(
                "reg",
                &[
                    "add",
                    r"HKCU\Software\FzComputerAI",
                    "/v",
                    name.as_str(),
                    "/t",
                    "REG_SZ",
                    "/d",
                    value.as_str(),
                    "/f",
                ],
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (pid, local_port);
        }
    }

    /// CreationDate (CIM) de um PID — elimina risco de PID reciclado.
    #[cfg(target_os = "windows")]
    fn pid_creation_date(&mut self, pid: u32) -> Option<String> {
        let ps = format!(
            "$p = Get-CimInstance Win32_Process -Filter \"ProcessId={}\"; if ($p) {{ $p.CreationDate.ToString('yyyyMMddHHmmss') }}",
            pid
        );
        let out = self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    /// Guarda-PS disparada NO START: vive por conta própria e mata o túnel
    /// quando ESTE processo da GUI desaparecer (cobre taskkill /F, crash,
    /// queda de energia — casos em que o on_exit NÃO roda). spawn(), NUNCA
    /// wait(). Mata só com imagem + CreationDate + marcador na cmdline.
    fn spawn_tunnel_guard(&mut self, tunnel_pid: u32) {
        #[cfg(target_os = "windows")]
        {
            let gui_pid = std::process::id();
            let slug = Self::provider_slug(self.tunnel_provider);
            let image = Self::provider_image(self.tunnel_provider);
            let creation = self.pid_creation_date(tunnel_pid).unwrap_or_default();
            // Marcador de identidade = o run_id (8 chars aleatorios), que
            // aparece no path do --logfile/--log/-E na command line. Unico o
            // bastante para, junto de imagem + CreationDate, garantir que so
            // matamos o NOSSO processo (nunca cloudflared/ssh alheio).
            let marker = self.tunnel_run_id.clone();
            let name = format!("tunnel:{}:{}", slug, tunnel_pid);
            // $tp em vez de $pid ($pid e variavel automatica reservada no PS).
            let ps = format!(
                "$ErrorActionPreference='SilentlyContinue'; $g={gui}; $tp={tp}; $img='{img}'; $ct='{ct}'; $mark='{mark}'; \
                 $key='HKCU:\\Software\\FzComputerAI'; $name='{name}'; \
                 while ($true) {{ \
                   $t = Get-CimInstance Win32_Process -Filter \"ProcessId=$tp\"; \
                   if (-not $t) {{ break }}; \
                   if ($t.Name -ne $img) {{ break }}; \
                   if ($ct -and $t.CreationDate.ToString('yyyyMMddHHmmss') -ne $ct) {{ break }}; \
                   if ($t.CommandLine -notlike ('*' + $mark + '*')) {{ break }}; \
                   if (-not (Get-Process -Id $g -ErrorAction SilentlyContinue)) {{ Stop-Process -Id $tp -Force; break }}; \
                   Start-Sleep -Seconds 2 \
                 }}; \
                 $cur = (Get-ItemProperty -Path $key -Name $name -ErrorAction SilentlyContinue).$name; \
                 if ($cur -and (($cur -split '\\|')[3]) -eq $mark) {{ Remove-ItemProperty -Path $key -Name $name -Force -ErrorAction SilentlyContinue }}",
                gui = gui_pid,
                tp = tunnel_pid,
                img = image,
                ct = creation,
                mark = marker,
                name = name
            );
            let _ = quiet_cmd("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps.as_str()])
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = tunnel_pid;
        }
    }

    /// Para o túnel: mata o filho, confirma ausência (relê o tasklist —
    /// nunca presume), encerra o gate e limpa o registro em HKCU.
    pub fn stop_tunnel(&mut self) {
        let Some(pid) = self.tunnel_pid else {
            self.tunnel_status = TunnelStatus::Stopped;
            self.stop_gate();
            return;
        };
        // Mata a ÁRVORE enquanto o handle do Child ainda reserva o PID (sem
        // janela de reuso): taskkill /T ANTES do wait()/drop. Depois reap.
        if let Some(mut child) = self.tunnel_child.take() {
            #[cfg(target_os = "windows")]
            {
                let pid_s = pid.to_string();
                let _ = self.run_logged("taskkill", &["/PID", pid_s.as_str(), "/T", "/F"]);
            }
            let _ = child.kill();
            let _ = child.wait();
        }

        // Confirma ausência por IDENTIDADE (imagem + marcador run_id), nunca por
        // PID nu — um PID reciclado leria como "vivo" e vazaria o registro HKCU
        // ou mataria árvore alheia. Mesma disciplina do watchdog/reconcile.
        let still = self.tunnel_pid_is_ours_alive(pid);
        if still {
            self.tunnel_status = TunnelStatus::Error;
            self.status_msg = self.tr(
                "AVISO: o processo do tunel AINDA esta vivo apos o encerramento.",
                "WARNING: the tunnel process is STILL alive after termination.",
            );
            self.log_debug("[tunnel] AVISO: processo ainda vivo apos kill+taskkill.");
        } else {
            self.tunnel_status = TunnelStatus::Stopped;
            self.tunnel_exposure = None;
            // URL de tunel morto e mentirosa — limpa (exceto tunel nomeado
            // com hostname fixo informado pelo usuario).
            if self.tunnel_cf_token_file.trim().is_empty() {
                self.tunnel_public_url.clear();
            }
            self.log_debug("[tunnel] Tunel encerrado e confirmado ausente.");
            self.clear_tunnel_registration(pid);
        }
        self.stop_gate();
        self.tunnel_pid = None;
    }

    /// O PID ainda é o NOSSO túnel vivo? Checa identidade (imagem + marcador
    /// run_id na cmdline), não o PID nu — assim um PID reciclado por outro
    /// processo nunca é lido como "nosso túnel vivo". Fora do Windows,
    /// best-effort false.
    fn tunnel_pid_is_ours_alive(&mut self, pid: u32) -> bool {
        #[cfg(target_os = "windows")]
        {
            let image = Self::provider_image(self.tunnel_provider);
            let mark = self.tunnel_run_id.clone();
            let ps = format!(
                "$p = Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\"; \
                 if ($p -and $p.Name -eq '{img}' -and '{mark}' -ne '' -and $p.CommandLine -like ('*' + '{mark}' + '*')) {{ 'ALIVE' }} else {{ 'GONE' }}",
                pid = pid,
                img = image,
                mark = mark
            );
            self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("ALIVE"))
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = pid;
            false
        }
    }

    /// Remove o valor tunnel:<slug>:<pid> do HKCU.
    fn clear_tunnel_registration(&mut self, pid: u32) {
        #[cfg(target_os = "windows")]
        {
            let name = format!("tunnel:{}:{}", Self::provider_slug(self.tunnel_provider), pid);
            let _ = self.run_logged(
                "reg",
                &["delete", r"HKCU\Software\FzComputerAI", "/v", name.as_str(), "/f"],
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = pid;
        }
    }

    /// Extrai a URL pública do texto do log, por sufixo do provedor. Sem
    /// crate `regex`: busca "https://", consome o host e aceita se terminar
    /// num sufixo conhecido. Devolve o ÚLTIMO match (o banner final é o que
    /// vale).
    fn extract_public_url(text: &str, p: TunnelProvider) -> Option<String> {
        let suffixes: &[&str] = match p {
            TunnelProvider::Cloudflare => &[".trycloudflare.com"],
            TunnelProvider::Ngrok => &[".ngrok-free.app", ".ngrok-free.dev", ".ngrok.app", ".ngrok.io"],
            // localhost.run -> *.lhr.life ; serveo.net -> a URL publica vem em
            // *.serveousercontent.com (nao em serveo.net). O sufixo antigo
            // ".serveo.net" nunca casava a URL real do serveo.
            TunnelProvider::Ssh => &[".lhr.life", ".serveousercontent.com", ".serveo.net"],
        };
        let bytes = text.as_bytes();
        let mut found: Option<String> = None;
        let mut i = 0usize;
        while let Some(rel) = text[i..].find("https://") {
            let start = i + rel;
            let host_start = start + "https://".len();
            let mut end = host_start;
            while end < bytes.len() {
                let c = bytes[end] as char;
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    end += 1;
                } else {
                    break;
                }
            }
            let host = &text[host_start..end];
            for suf in suffixes {
                if host.len() > suf.len() && host.ends_with(suf) {
                    found = Some(format!("https://{}", host));
                    break;
                }
            }
            i = end.max(start + "https://".len());
        }
        found
    }

    /// Observado a cada ~1s pelo loop da UI enquanto o túnel está ativo:
    /// detecta morte do processo e captura a URL pública do log.
    pub fn poll_tunnel(&mut self) {
        if matches!(self.tunnel_status, TunnelStatus::Stopped | TunnelStatus::Error) {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.tunnel_last_poll {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.tunnel_last_poll = Some(now);

        // (A) o processo morreu sozinho?
        let exited = self
            .tunnel_child
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten());
        if let Some(status) = exited {
            let tail = std::fs::read_to_string(self.tunnel_log_path()).unwrap_or_default();
            let tail = tail_str(&tail, 1200);
            self.tunnel_status = TunnelStatus::Error;
            self.status_msg = format!(
                "{} (exit {:?})\n\n{}",
                self.tr("O processo do tunel SAIU", "The tunnel process EXITED"),
                status.code(),
                tail
            );
            self.log_debug(&format!("[tunnel] processo saiu (exit {:?}).", status.code()));
            if let Some(pid) = self.tunnel_pid.take() {
                self.clear_tunnel_registration(pid);
            }
            self.tunnel_child = None;
            self.stop_gate();
            return;
        }

        // (B) URL ainda não capturada? Lê o log e tenta extrair.
        if self.tunnel_public_url.trim().is_empty() {
            let txt = std::fs::read_to_string(self.tunnel_log_path()).unwrap_or_default();
            if let Some(url) = Self::extract_public_url(&txt, self.tunnel_provider) {
                self.tunnel_public_url = url.clone();
                self.tunnel_status = TunnelStatus::Running;
                self.status_msg = format!(
                    "{}: {}",
                    self.tr("URL publica capturada", "Public URL captured"),
                    url
                );
                self.log_debug(&format!("[tunnel] URL publica capturada: {}", url));
            }
        } else if self.tunnel_status == TunnelStatus::Starting {
            // URL informada à mão (túnel nomeado): promove a Running.
            self.tunnel_status = TunnelStatus::Running;
        }
    }

    /// URL pública COMPLETA (com /s/<senha> quando houver gate) + /mcp.
    pub fn tunnel_full_url(&self) -> String {
        let base = self.tunnel_public_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return String::new();
        }
        if self.tunnel_gate_password.trim().is_empty() {
            format!("{}/mcp", base)
        } else {
            format!("{}/s/{}/mcp", base, self.tunnel_gate_password.trim())
        }
    }

    /// Snippet mcpServers pronto para colar num cliente MCP. Inclui o header
    /// Authorization quando o motor exige token (0.16+) — sem ele o cliente
    /// tomaria 401 e o usuario ficaria sem saber por que.
    pub fn tunnel_mcp_snippet(&self) -> String {
        let url = self.tunnel_full_url();
        if self.mcp_token.trim().is_empty() {
            format!(
                "{{\n  \"mcpServers\": {{\n    \"fzcomputerai\": {{\n      \"type\": \"http\",\n      \"url\": \"{}\"\n    }}\n  }}\n}}",
                url
            )
        } else {
            format!(
                "{{\n  \"mcpServers\": {{\n    \"fzcomputerai\": {{\n      \"type\": \"http\",\n      \"url\": \"{}\",\n      \"headers\": {{\n        \"Authorization\": \"Bearer {}\"\n      }}\n    }}\n  }}\n}}",
                url,
                self.mcp_token.trim()
            )
        }
    }

    /// SONDA DE EXPOSIÇÃO: POST initialize na URL PÚBLICA via curl.exe
    /// (TcpStream não faz TLS). Se há senha, envia com ela — espera jsonrpc;
    /// resultado registrado honestamente em tunnel_exposure.
    pub fn verify_tunnel(&mut self) {
        let url = self.tunnel_full_url();
        if url.is_empty() {
            self.status_msg = self.tr("Sem URL publica para testar.", "No public URL to test.");
            return;
        }
        let dir = Self::tunnel_dir();
        let _ = std::fs::create_dir_all(&dir);
        let body_path = dir.join(format!("probe-{}.json", self.tunnel_run_id));
        let body = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"fzcomputerai-gui","version":"{}"}}}}}}"#,
            env!("CARGO_PKG_VERSION")
        );
        if std::fs::write(&body_path, &body).is_err() {
            self.status_msg = self.tr("Falha ao preparar o corpo do teste.", "Failed to prepare the test body.");
            return;
        }
        let data_arg = format!("@{}", body_path.display());
        // A URL contém a senha do gate (/s/<senha>/mcp). NÃO pode ir pelo argv
        // (legível por qualquer processo via Win32_Process.CommandLine) nem por
        // run_logged (que loga o argv no Console Debug). Vai num arquivo de
        // config do curl (`--config`), e o log é MASCARADO à mão.
        let cfg_path = dir.join(format!("probe-{}.cfg", self.tunnel_run_id));
        if std::fs::write(&cfg_path, format!("url = \"{}\"\n", url)).is_err() {
            self.status_msg = self.tr("Falha ao preparar o teste.", "Failed to prepare the test.");
            return;
        }
        let cfg_arg = cfg_path.display().to_string();
        self.log_debug(&format!(
            "> curl [probe] -X POST {} (URL via --config; senha omitida)",
            self.mask_gate_url(&url)
        ));
        let out = quiet_cmd("curl")
            .args([
                "-sS",
                "-m",
                "20",
                "-o",
                "-",
                "-w",
                "\nHTTP_CODE=%{http_code}",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-H",
                "Accept: application/json, text/event-stream",
                "-H",
                "ngrok-skip-browser-warning: 1",
                "--data-binary",
                data_arg.as_str(),
                "--config",
                cfg_arg.as_str(),
            ])
            .output()
            .ok();
        // Não deixa o segredo em disco depois do teste.
        let _ = std::fs::remove_file(&cfg_path);
        let _ = std::fs::remove_file(&body_path);
        match out {
            Some(o) => {
                let text = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                let code = text
                    .rsplit("HTTP_CODE=")
                    .next()
                    .and_then(|s| s.trim().parse::<u16>().ok())
                    .unwrap_or(0);
                if text.contains("jsonrpc") {
                    self.tunnel_exposure = Some(TunnelExposure::Exposed);
                } else if matches!(code, 401 | 403 | 302 | 407) {
                    self.tunnel_exposure = Some(TunnelExposure::EdgeAuth(code));
                } else {
                    self.tunnel_exposure = Some(TunnelExposure::Unknown);
                }
                let combined = format!("{}{}", text, stderr);
                self.status_msg = format!(
                    "{} ({}):\n{}",
                    self.tr("Teste pela internet", "Internet test"),
                    self.mask_gate_url(&url),
                    tail_str(self.mask_gate_url(combined.trim()).as_str(), 1200)
                );
            }
            None => {
                self.tunnel_exposure = Some(TunnelExposure::Unknown);
                self.status_msg = self.tr(
                    "Falha ao executar curl para o teste.",
                    "Failed to run curl for the test.",
                );
            }
        }
    }

    /// Mascara a senha do gate em qualquer texto destinado a log/UI —
    /// substitui o segmento /s/<senha>/ por /s/***/. Nunca deixa a senha do
    /// gate (única credencial da URL pública) aparecer no Console Debug.
    fn mask_gate_url(&self, s: &str) -> String {
        let pw = self.tunnel_gate_password.trim();
        if pw.is_empty() {
            s.to_string()
        } else {
            s.replace(&format!("/s/{}/", pw), "/s/***/")
        }
    }

    /// ngrok: consulta a API local (loopback, HTTP puro) para descobrir a URL.
    pub fn ngrok_query_local_api(&mut self) {
        let body = self.http_get_local(4040, "/api/tunnels");
        let body = match body {
            Some(b) if !b.is_empty() => b,
            _ => {
                self.status_msg = self.tr(
                    "Nao consegui falar com a API local do ngrok (127.0.0.1:4040). Se houver outro agente ngrok, abra http://127.0.0.1:4040.",
                    "Could not reach the local ngrok API (127.0.0.1:4040). If another ngrok agent is running, open http://127.0.0.1:4040.",
                );
                return;
            }
        };
        if let Some(url) = Self::extract_public_url(&body, TunnelProvider::Ngrok) {
            self.tunnel_public_url = url.clone();
            if self.tunnel_status == TunnelStatus::Starting {
                self.tunnel_status = TunnelStatus::Running;
            }
            self.status_msg = format!("ngrok API 4040 -> {}", url);
            self.log_debug(&format!("[tunnel] URL do ngrok pela API 4040: {}", url));
        } else {
            self.status_msg = self.tr(
                "A API 4040 respondeu mas nao encontrei uma URL publica.",
                "The 4040 API answered but no public URL was found.",
            );
        }
    }

    /// GET simples em 127.0.0.1:porta/path (HTTP puro, sem TLS) — para a API
    /// local do ngrok. Espelho enxuto do mcp_probe.
    fn http_get_local(&mut self, port: u16, path: &str) -> Option<String> {
        use std::io::{Read, Write};
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().ok()?;
        let timeout = std::time::Duration::from_millis(1500);
        let mut stream = std::net::TcpStream::connect_timeout(&addr, timeout).ok()?;
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            path, port
        );
        stream.write_all(req.as_bytes()).ok()?;
        let mut collected = Vec::new();
        let mut buf = [0u8; 2048];
        while collected.len() < 65536 {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => collected.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        let text = String::from_utf8_lossy(&collected).to_string();
        // Retorna só o corpo (após o cabeçalho).
        Some(match text.split_once("\r\n\r\n") {
            Some((_, body)) => body.to_string(),
            None => text,
        })
    }

    /// Login OAuth do Cloudflare (abre o navegador). Processo destacado.
    pub fn cloudflared_login(&mut self) {
        if self.tunnel_cf_bin.is_empty() {
            self.status_msg = self.tr(
                "cloudflared nao encontrado. Baixe-o antes de fazer login.",
                "cloudflared not found. Download it before logging in.",
            );
            return;
        }
        let bin = self.tunnel_cf_bin.clone();
        match quiet_cmd(&bin).args(["tunnel", "login"]).spawn() {
            Ok(_) => {
                self.status_msg = self.tr(
                    "Login Cloudflare aberto no navegador. Conclua a autorizacao e depois crie/route o tunel nomeado.",
                    "Cloudflare login opened in the browser. Complete the authorization, then create/route the named tunnel.",
                );
                self.log_debug("[tunnel] cloudflared tunnel login disparado (navegador).");
            }
            Err(e) => {
                self.status_msg = format!("Falha ao iniciar login do cloudflared: {}", e);
            }
        }
    }

    /// Salva o token do Cloudflare colado num arquivo com ACL restrita e usa
    /// só o CAMINHO daqui em diante (o token nunca vai para argv/log/HKCU).
    pub fn save_cf_token(&mut self) {
        let token = self.tunnel_cf_token_input.trim().to_string();
        if token.is_empty() {
            self.status_msg = self.tr("Cole o token do Cloudflare primeiro.", "Paste the Cloudflare token first.");
            return;
        }
        let dir = Self::tunnel_download_dir();
        let path = dir.join("cf-token.txt");
        if std::fs::write(&path, &token).is_err() {
            self.status_msg = self.tr("Falha ao gravar o token-file.", "Failed to write the token-file.");
            return;
        }
        #[cfg(target_os = "windows")]
        {
            let user = std::env::var("USERNAME").unwrap_or_default();
            if !user.is_empty() {
                let p = path.display().to_string();
                let grant = format!("{}:R", user);
                let _ = self.run_logged("icacls", &[p.as_str(), "/inheritance:r", "/grant:r", grant.as_str()]);
            }
        }
        self.tunnel_cf_token_file = path.display().to_string();
        self.tunnel_cf_token_input.clear();
        self.log_debug("[tunnel] Token do Cloudflare salvo em token-file com ACL restrita (nao logado).");
        self.status_msg = self.tr(
            "Token salvo. O tunel Cloudflare passa a rodar em modo NOMEADO (token-file).",
            "Token saved. The Cloudflare tunnel now runs in NAMED mode (token-file).",
        );
    }

    /// Esquece o token do Cloudflare (apaga o arquivo e volta ao quick tunnel).
    pub fn forget_cf_token(&mut self) {
        if !self.tunnel_cf_token_file.is_empty() {
            let _ = std::fs::remove_file(&self.tunnel_cf_token_file);
            self.tunnel_cf_token_file.clear();
            self.log_debug("[tunnel] Token-file do Cloudflare removido — de volta ao quick tunnel.");
        }
    }

    /// Baixa o cloudflared (Apache-2.0) via winget (valida hash sozinho) com
    /// fallback direto do release + comparação de SHA256 com o manifesto
    /// winget. Processo destacado + flags (padrão start_update_download).
    pub fn download_cloudflared(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::tunnel_download_dir();
            let dest = dir.join("cloudflared.exe");
            let ps = format!(
                "$ErrorActionPreference='Stop'; $d='{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'cf-ready.flag'),(Join-Path $d 'cf-error.flag') -Force -ErrorAction SilentlyContinue; \
                 try {{ \
                   winget install --exact --id Cloudflare.cloudflared --source winget --installer-type portable --accept-source-agreements --accept-package-agreements --disable-interactivity --location $d 2>&1 | Out-Null; \
                   if (-not (Test-Path (Join-Path $d 'cloudflared.exe'))) {{ \
                     $u='https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe'; \
                     Invoke-WebRequest -Uri $u -OutFile '{dest}' -UseBasicParsing; \
                   }}; \
                   $h=(Get-FileHash -Path '{dest}' -Algorithm SHA256).Hash.ToLower(); \
                   $sig=(Get-AuthenticodeSignature '{dest}').Status; \
                   Set-Content -Path (Join-Path $d 'cf-ready.flag') -Value (\"SHA256=$h`nAuthenticode=$sig\") \
                 }} catch {{ Set-Content -Path (Join-Path $d 'cf-error.flag') -Value $_.Exception.Message }}",
                dir = dir.display(),
                dest = dest.display()
            );
            match quiet_cmd("powershell").args(["-NoProfile", "-Command", ps.as_str()]).spawn() {
                Ok(_) => {
                    self.tunnel_downloading = true;
                    self.status_msg = self.tr(
                        "Baixando cloudflared em segundo plano...",
                        "Downloading cloudflared in the background...",
                    );
                    self.log_debug("[tunnel] Download do cloudflared iniciado (background).");
                }
                Err(e) => {
                    self.log_debug(&format!("[tunnel] ERRO ao iniciar download do cloudflared: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.status_msg = "Download automatico disponivel apenas no Windows.".to_string();
        }
    }

    /// Baixa o ngrok — SOMENTE após o usuário aceitar os termos (modal). O
    /// download é da fonte oficial, na máquina do usuário, por clique dele.
    pub fn download_ngrok(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let dir = Self::tunnel_download_dir();
            let ps = format!(
                "$ErrorActionPreference='Stop'; $d='{dir}'; \
                 New-Item -ItemType Directory -Force -Path $d | Out-Null; \
                 Remove-Item (Join-Path $d 'ngrok-ready.flag'),(Join-Path $d 'ngrok-error.flag') -Force -ErrorAction SilentlyContinue; \
                 try {{ \
                   winget install --exact --id Ngrok.Ngrok --source winget --accept-source-agreements --accept-package-agreements --disable-interactivity 2>&1 | Out-Null; \
                   $z=Join-Path $d 'ngrok.zip'; \
                   if (-not (Get-Command ngrok -ErrorAction SilentlyContinue) -and -not (Test-Path (Join-Path $d 'ngrok.exe'))) {{ \
                     Invoke-WebRequest -Uri 'https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-windows-amd64.zip' -OutFile $z -UseBasicParsing; \
                     Expand-Archive -Path $z -DestinationPath $d -Force; \
                   }}; \
                   $exe=Join-Path $d 'ngrok.exe'; \
                   if (Test-Path $exe) {{ $h=(Get-FileHash $exe -Algorithm SHA256).Hash.ToLower(); $sig=(Get-AuthenticodeSignature $exe).Status }} else {{ $h='(via winget)'; $sig='(via winget)' }}; \
                   Set-Content -Path (Join-Path $d 'ngrok-ready.flag') -Value (\"SHA256=$h`nAuthenticode=$sig\") \
                 }} catch {{ Set-Content -Path (Join-Path $d 'ngrok-error.flag') -Value $_.Exception.Message }}",
                dir = dir.display()
            );
            match quiet_cmd("powershell").args(["-NoProfile", "-Command", ps.as_str()]).spawn() {
                Ok(_) => {
                    self.tunnel_downloading = true;
                    self.status_msg = self.tr(
                        "Baixando ngrok em segundo plano (fonte oficial)...",
                        "Downloading ngrok in the background (official source)...",
                    );
                    self.log_debug("[tunnel] Download do ngrok iniciado (background, apos aceite dos termos).");
                }
                Err(e) => {
                    self.log_debug(&format!("[tunnel] ERRO ao iniciar download do ngrok: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.status_msg = "Download automatico disponivel apenas no Windows.".to_string();
        }
    }

    /// Observa os downloads de binário de túnel (flags em %TEMP%).
    pub fn poll_tunnel_download(&mut self) {
        if !self.tunnel_downloading {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.tunnel_last_probe {
            if now.duration_since(last).as_millis() < 1000 {
                return;
            }
        }
        self.tunnel_last_probe = Some(now);

        let dir = Self::tunnel_download_dir();
        for (ready, error, label) in [
            ("cf-ready.flag", "cf-error.flag", "cloudflared"),
            ("ngrok-ready.flag", "ngrok-error.flag", "ngrok"),
        ] {
            let rp = dir.join(ready);
            let ep = dir.join(error);
            if ep.exists() {
                let msg = std::fs::read_to_string(&ep).unwrap_or_default();
                let _ = std::fs::remove_file(&ep);
                self.tunnel_downloading = false;
                self.status_msg = format!("Falha ao baixar {}: {}", label, msg.trim());
                self.log_debug(&format!("[tunnel] FALHA no download de {}: {}", label, msg.trim()));
                self.detect_tunnel_bins();
            } else if rp.exists() {
                let info = std::fs::read_to_string(&rp).unwrap_or_default();
                let _ = std::fs::remove_file(&rp);
                self.tunnel_downloading = false;
                self.log_debug(&format!(
                    "[tunnel] {} baixado e verificado:\n{}",
                    label,
                    info.trim()
                ));
                self.status_msg = format!(
                    "{} {}:\n{}",
                    label,
                    self.tr("baixado e verificado", "downloaded and verified"),
                    info.trim()
                );
                self.detect_tunnel_bins();
            }
        }
    }

    /// FASE 2 da reconciliação na abertura: túneis NOSSOS (tunnel:*) que
    /// sobraram de uma sessão anterior. Mata só com identidade de 4 fatores;
    /// nunca taskkill /IM.
    #[cfg(target_os = "windows")]
    pub fn startup_reconcile_tracked_tunnels(&mut self) {
        let out = match self.run_logged("reg", &["query", r"HKCU\Software\FzComputerAI"]) {
            Some(o) if o.status.success() => o,
            _ => return,
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut entries: Vec<(String, String)> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("tunnel:") {
                continue;
            }
            let Some(pos) = trimmed.find("REG_SZ") else { continue };
            let name = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + "REG_SZ".len()..].trim().to_string();
            entries.push((name, value));
        }
        if entries.is_empty() {
            return;
        }
        for (name, value) in entries {
            // name = tunnel:<slug>:<pid> ; value = imagem|creation|porta|id|modo
            let pid: u32 = match name.rsplit(':').next().and_then(|s| s.parse().ok()) {
                Some(p) => p,
                None => continue,
            };
            let parts: Vec<&str> = value.split('|').collect();
            let image = parts.first().copied().unwrap_or("");
            let creation = parts.get(1).copied().unwrap_or("");
            let run_id = parts.get(3).copied().unwrap_or("");
            let marker = run_id.to_string();
            let ps = format!(
                "$ErrorActionPreference='SilentlyContinue'; $tp={tp}; \
                 $p=Get-CimInstance Win32_Process -Filter \"ProcessId=$tp\"; \
                 if ($p -and $p.Name -eq '{img}' -and ('{ct}' -eq '' -or $p.CreationDate.ToString('yyyyMMddHHmmss') -eq '{ct}') -and '{mark}' -ne '' -and $p.CommandLine -like ('*{mark}*')) {{ Stop-Process -Id $tp -Force; Write-Output 'KILLED' }} else {{ Write-Output 'SKIP' }}",
                tp = pid,
                img = image,
                ct = creation,
                mark = marker
            );
            let killed = self
                .run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("KILLED"))
                .unwrap_or(false);
            if killed {
                self.log_debug(&format!(
                    "[startup] TUNEL ORFAO encerrado (pid {}, id {}) — a maquina esteve exposta ate agora.",
                    pid, run_id
                ));
            } else {
                self.log_debug(&format!(
                    "[startup] Rastro de tunel (pid {}) nao corresponde a um processo nosso vivo — apenas desregistrando.",
                    pid
                ));
            }
            let _ = self.run_logged(
                "reg",
                &["delete", r"HKCU\Software\FzComputerAI", "/v", name.as_str(), "/f"],
            );
        }
    }

    /// Helper i18n curto: PT ou EN conforme o idioma ativo.
    fn tr(&self, pt: &str, en: &str) -> String {
        match self.language {
            Language::PtBr => pt.to_string(),
            Language::English => en.to_string(),
        }
    }
}

/// Relay de UMA conexão do gate: valida /s/<senha>/ e encaminha ao MCP.
/// Fora do impl porque roda em thread própria (sem &self).
fn gate_handle_conn(mut client: std::net::TcpStream, mcp_port: u16, password: &str) {
    use std::io::{Read, Write};
    let timeout = std::time::Duration::from_secs(30);
    let _ = client.set_read_timeout(Some(timeout));
    let _ = client.set_write_timeout(Some(timeout));

    // Lê até o fim do cabeçalho (\r\n\r\n).
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    let header_end;
    loop {
        match client.read(&mut tmp) {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
                    header_end = pos + 4;
                    break;
                }
                if buf.len() > 65536 {
                    return;
                }
            }
            Err(_) => return,
        }
    }

    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let first_line_end = match head.find("\r\n") {
        Some(p) => p,
        None => return,
    };
    let req_line = &head[..first_line_end];
    let mut parts = req_line.split(' ');
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let version = parts.next().unwrap_or("HTTP/1.1");

    // Content-Length (para ler o corpo por inteiro).
    let mut content_len = 0usize;
    for l in head.lines() {
        let low = l.to_ascii_lowercase();
        if let Some(v) = low.strip_prefix("content-length:") {
            content_len = v.trim().parse().unwrap_or(0);
        }
    }

    // Confere a senha no path: /s/<senha> ou /s/<senha>/...
    let prefix = format!("/s/{}", password);
    let with_slash = format!("{}/", prefix);
    if !(path == prefix || path.starts_with(&with_slash)) {
        let _ = client
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let new_path = {
        let stripped = &path[prefix.len()..];
        if stripped.is_empty() {
            "/".to_string()
        } else {
            stripped.to_string()
        }
    };

    // Conecta ao MCP.
    let mut upstream = match std::net::TcpStream::connect(("127.0.0.1", mcp_port)) {
        Ok(s) => s,
        Err(_) => {
            let _ = client.write_all(
                b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        }
    };
    let _ = upstream.set_read_timeout(Some(timeout));

    // Reescreve a 1a linha + cabeçalhos: descarta o Connection do cliente e
    // FORÇA "Connection: close". Sem isso, o MCP (HTTP/1.1 keep-alive por
    // padrão) NÃO fecha após responder — o relay abaixo travaria até o timeout
    // de 30s e o cliente reusaria a conexão e perderia a próxima requisição.
    // Com close, o MCP fecha após 1 resposta (o relay recebe EOF na hora) e o
    // cliente não reusa — coerente com este gate tratar 1 requisição por
    // conexão. (Mesmo padrão de Connection: close já usado no 404/502/probe.)
    let rest_headers = &head[first_line_end + 2..];
    let mut hdrs = String::new();
    for line in rest_headers.split("\r\n") {
        if line.is_empty() {
            continue;
        }
        if line.to_ascii_lowercase().starts_with("connection:") {
            continue;
        }
        hdrs.push_str(line);
        hdrs.push_str("\r\n");
    }
    hdrs.push_str("Connection: close\r\n");
    let new_head = format!("{} {} {}\r\n{}\r\n", method, new_path, version, hdrs);
    if upstream.write_all(new_head.as_bytes()).is_err() {
        return;
    }
    // Corpo já bufferizado junto ao cabeçalho.
    let _ = upstream.write_all(&buf[header_end..]);

    // Encaminha o RESTANTE do corpo do cliente -> upstream numa thread, SEM
    // depender de Content-Length (funciona também com Transfer-Encoding:
    // chunked). Termina no EOF do cliente ou no read_timeout de 30s. O
    // content_len parseado acima fica só como referência/diagnóstico.
    let _ = content_len;
    if let (Ok(mut c2), Ok(mut u2)) = (client.try_clone(), upstream.try_clone()) {
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut c2, &mut u2);
            let _ = u2.shutdown(std::net::Shutdown::Write);
        });
    }

    // Relay da resposta de volta ao cliente (termina quando o upstream fecha,
    // graças ao Connection: close acima).
    let mut resp = [0u8; 8192];
    loop {
        match upstream.read(&mut resp) {
            Ok(0) => break,
            Ok(n) => {
                if client.write_all(&resp[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Acha a primeira ocorrência de `needle` em `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// Últimos `max` chars de um texto (corta em char boundary), para logs.
fn tail_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut start = s.len() - max;
    while !s.is_char_boundary(start) {
        start += 1;
    }
    format!("...{}", &s[start..])
}

#[derive(Default)]
pub struct FzComputerApp {
    pub state: AppState,
    /// Bandeja do sistema. Criada sob demanda (só quando o usuário liga
    /// "minimizar para a bandeja"), para não deixar ícone na área de
    /// notificação de quem não pediu.
    tray: Option<crate::tray::Tray>,
}

// ─── PALETA TERMINAL (preto + verde) ────────────────────────────────────
// Fundo preto, texto predominantemente verde, secundário branco/cinza, tudo
// monoespaçado — visual de TUI. As cores SEMÂNTICAS de status (amarelo/
// vermelho) sobrevivem de propósito: elas carregam informação de segurança
// ("EXPOSTO SEM AUTENTICAÇÃO", "REGRA SEM EFEITO") e num tema só-verde essa
// informação se perderia. Decoração é monocromática; estado, não.
// ─── LIMIAR DE LEITURA POR CÂMERA (não baixe o brilho destas cores) ─────
// O Roger lê a tela com template matching em OpenCV, que binariza assim:
//     cv::threshold(cinza, bin, 180, 255, cv::THRESH_BINARY)
// e o cinza vem de COLOR_BGR2GRAY:  Y = 0.299*R + 0.587*G + 0.114*B
// Ou seja: TEXTO COM Y <= 180 SIMPLESMENTE DESAPARECE para ele.
//
// A paleta anterior falhava exatamente nisso: o verde normal (64,224,132)
// dava Y=166 e o "brilhante" (0,255,128) dava Y=164 — ambos abaixo do corte.
// Os valores abaixo foram escolhidos para passar com folga; o Y de cada um
// está no comentário. Se mexer nas cores, RECALCULE o Y e mantenha > 190
// para texto legível.
pub const TERM_BG: Color32 = Color32::from_rgb(6, 8, 6); // fundo raiz (Y~7)
pub const TERM_BG_PANEL: Color32 = Color32::from_rgb(12, 16, 12); // cartões (Y~15)
pub const TERM_BG_INPUT: Color32 = Color32::from_rgb(0, 0, 0); // campos/console
pub const TERM_GREEN: Color32 = Color32::from_rgb(120, 255, 175); // texto normal (Y=206)
pub const TERM_GREEN_BRIGHT: Color32 = Color32::from_rgb(170, 255, 205); // destaque (Y=224)
pub const TERM_GREEN_DIM: Color32 = Color32::from_rgb(46, 110, 70); // bordas (Y~85, não é texto)
pub const TERM_WHITE: Color32 = Color32::from_rgb(235, 240, 235); // secundário (Y=238)
pub const TERM_GRAY: Color32 = Color32::from_rgb(170, 185, 170); // apoio (Y=180 no limite)

// Status (semânticos — NÃO monocromáticos). Também acima do limiar:
pub const ST_OK: Color32 = Color32::from_rgb(110, 255, 165); // Y=203
pub const ST_WARN: Color32 = Color32::from_rgb(255, 214, 90); // Y=212
pub const ST_ERR: Color32 = Color32::from_rgb(255, 140, 135); // Y=174 -> ver nota
// NOTA sobre o vermelho: vermelho puro tem Y baixo por natureza (o olho humano
// pesa pouco o canal R). Para o vermelho de ERRO passar de 180 ele viraria
// rosa-claro e perderia a leitura de "perigo" para quem vê a cor. Mantemos o
// vermelho legível a olho e garantimos que TODO estado crítico também esteja
// escrito em texto (PARADO, ERRO, SEM REGRA, EXPOSTO), que é o que o leitor
// por câmera captura.

/// Botão no estilo terminal: sem preenchimento, contorno e texto verdes sobre
/// preto. O realce no hover vem do `visuals.widgets.hovered` do tema.
pub fn term_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.to_string()).color(TERM_GREEN))
        .fill(TERM_BG_INPUT)
        .stroke(egui::Stroke::new(1.0_f32, TERM_GREEN_DIM))
        .rounding(egui::Rounding::same(2.0))
}

/// Variante para ações destrutivas/de parada (Parar, Remover): mesmo desenho,
/// contorno e texto avermelhados — o vermelho aqui é semântico, não enfeite.
pub fn term_button_danger(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.to_string()).color(ST_ERR))
        .fill(TERM_BG_INPUT)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(120, 45, 45)))
        .rounding(egui::Rounding::same(2.0))
}

fn setup_fazai_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = TERM_BG;
    visuals.window_fill = TERM_BG;
    visuals.faint_bg_color = TERM_BG_PANEL;
    visuals.extreme_bg_color = TERM_BG_INPUT;
    visuals.window_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_DIM);
    visuals.hyperlink_color = TERM_GREEN_BRIGHT;

    let sharp = egui::Rounding::same(2.0);

    visuals.widgets.noninteractive.bg_fill = TERM_BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = TERM_BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(24, 40, 28));
    visuals.widgets.noninteractive.rounding = sharp;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN);

    visuals.widgets.inactive.bg_fill = TERM_BG_INPUT;
    visuals.widgets.inactive.weak_bg_fill = TERM_BG_INPUT;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_DIM);
    visuals.widgets.inactive.rounding = sharp;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(16, 40, 24);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(16, 40, 24);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);
    visuals.widgets.hovered.rounding = sharp;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);

    visuals.widgets.active.bg_fill = Color32::from_rgb(22, 58, 34);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(22, 58, 34);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);
    visuals.widgets.active.rounding = sharp;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);

    visuals.selection.bg_fill = Color32::from_rgb(22, 58, 34);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, TERM_GREEN_BRIGHT);

    ctx.set_visuals(visuals);

    // TUDO monoespaçado: a fonte mono (Hack) já vem embutida no eframe pela
    // feature "default_fonts" — nenhuma dependência ou arquivo novo.
    //
    // A família mono é ~15% MAIS LARGA que a proporcional por caractere. Se
    // mantivéssemos os tamanhos originais, toda linha "título à esquerda +
    // status à direita" passaria a colidir e as fileiras de botões sairiam da
    // tela. Por isso reduzimos um pouco cada tamanho junto com a troca de
    // família — o layout volta a caber sem mexer em cada tela.
    let mut style = (*ctx.style()).clone();
    for (_ts, font_id) in style.text_styles.iter_mut() {
        font_id.family = egui::FontFamily::Monospace;
        font_id.size = (font_id.size * 0.86).max(9.0);
    }
    // Espaçamento um pouco menor: em fonte mono os widgets já ficam largos.
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.visuals.widgets.noninteractive.fg_stroke.color = TERM_GREEN;
    ctx.set_style(style);
}

impl eframe::App for FzComputerApp {
    // Fechar a GUI = desligar o sistema por inteiro: para o daemon
    // cua-driver e remove as regras portproxy LAN -> localhost (inclusive
    // orfas de testes antigos). Ver AppState::shutdown_cleanup.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.shutdown_cleanup();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        setup_fazai_theme(ctx);

        // Observa o download do upgrade em background (throttle interno de 1s).
        self.state.poll_update_download();
        // tail -f do log REAL do motor: é o que traz para o console a atividade
        // de clientes MCP externos (conector do Claude, Antigravity, Cursor…),
        // que nunca passa por run_logged. Throttle interno de ~0,7s.
        self.state.poll_engine_log();
        // Observa o túnel (captura de URL / morte do processo) e downloads de
        // binários de túnel (throttle interno de 1s cada).
        self.state.poll_tunnel();
        self.state.poll_tunnel_download();
        // HTTPS: progresso da emissão Let's Encrypt e renovação periódica.
        self.state.poll_tls();
        // ─── BANDEJA (tray) ─────────────────────────────────────────────
        // Sobe o ícone só quando o usuário liga a opção, e derruba quando ele
        // desliga — quem não pediu não ganha ícone na área de notificação.
        #[cfg(target_os = "windows")]
        {
            if self.state.minimize_to_tray && self.tray.is_none() {
                let labels = [
                    match self.state.language {
                        Language::PtBr => "Abrir FzComputerAI",
                        Language::English => "Open FzComputerAI",
                    }
                    .to_string(),
                    match self.state.language {
                        Language::PtBr => "Iniciar / Parar motor",
                        Language::English => "Start / Stop engine",
                    }
                    .to_string(),
                    match self.state.language {
                        Language::PtBr => "Sair",
                        Language::English => "Quit",
                    }
                    .to_string(),
                    "FzComputerAI".to_string(),
                ];
                self.tray = crate::tray::spawn_tray_with_labels(labels);
                if self.tray.is_some() {
                    self.state.log_debug("[tray] Icone criado na area de notificacao.");
                } else {
                    self.state
                        .log_debug("[tray] FALHA ao criar o icone na area de notificacao.");
                    self.state.minimize_to_tray = false;
                }
            } else if !self.state.minimize_to_tray && self.tray.is_some() {
                self.tray = None; // Drop remove o icone
                self.state.window_hidden = false;
                self.state.log_debug("[tray] Icone removido da area de notificacao.");
            }

            // Comandos vindos do menu da bandeja (a thread da bandeja só
            // publica; quem mexe na janela é aqui, a thread da UI).
            if let Some(tray) = &self.tray {
                match tray.take_command() {
                    crate::tray::TRAY_SHOW => {
                        self.state.window_hidden = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    crate::tray::TRAY_TOGGLE_ENGINE => {
                        if self.state.port_active {
                            self.state.stop_daemon();
                        } else {
                            self.state.start_daemon();
                        }
                    }
                    crate::tray::TRAY_QUIT => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    _ => {}
                }
            }

            // Minimizou com a opção ligada? Esconde a janela (é isso que faz
            // "minimizar para a bandeja" em vez de ficar na barra de tarefas).
            if self.state.minimize_to_tray && !self.state.window_hidden {
                let minimized = ctx.input(|i| i.viewport().minimized).unwrap_or(false);
                if minimized {
                    self.state.window_hidden = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.state
                        .log_debug("[tray] Janela minimizada para a bandeja (o app continua rodando).");
                }
            }
        }

        self.state.poll_driver_update();
        if self.state.update_downloading
            || self.state.driver_updating
            || self.state.tunnel_downloading
            || self.state.tls_acme_busy
            || matches!(
                self.state.tunnel_status,
                TunnelStatus::Starting | TunnelStatus::Running
            )
        {
            // Garante novos frames mesmo sem input, para o poll acontecer.
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }

        // ─── POR QUE A BANDEJA PRECISA DE REPAINT PERIODICO ─────────────
        // Quando a janela e minimizada, o egui PARA de desenhar (sem input,
        // sem frame). Sem frame, o codigo que detecta "minimizou" nunca roda —
        // e a janela ficava na barra de tarefas em vez de esconder. Foi um bug
        // real: funcionou uma vez so porque um repaint casual caiu no momento
        // certo. Enquanto a opcao estiver ligada, pedimos um frame a cada
        // 300ms para conseguir observar a minimizacao e ler os comandos que a
        // thread da bandeja publica.
        if self.state.minimize_to_tray {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }

        // Header Principal
        // ─── BARRA LATERAL (navegação) ───────────────────────────────────
        // Substitui a fileira de abas "pill". Declarada ANTES dos painéis de
        // baixo para ocupar a ALTURA TOTAL da janela (fica ao lado do console
        // também). No pé dela: status real do MCP, chip de túnel, idioma e
        // Sobre — o que antes vivia no header de cima.
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(196.0)
            .frame(
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(10.0, 12.0))
                    .fill(Color32::from_rgb(3, 5, 3)),
            )
            .show(ctx, |ui| {
                // Marca + versão (versão SEMPRE do Cargo.toml).
                ui.label(
                    egui::RichText::new("FZComputerAI")
                        .size(17.0)
                        .strong()
                        .color(TERM_GREEN_BRIGHT),
                );
                ui.label(
                    egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .size(11.0)
                        .color(TERM_GRAY),
                );
                ui.label(
                    egui::RichText::new(match self.state.language {
                        Language::PtBr => "livre, poderoso e seguro",
                        Language::English => "free, powerful and safe",
                    })
                    .size(10.0)
                    .color(TERM_GREEN_DIM),
                );
                // Modo portátil precisa ser VISÍVEL: muda onde a config mora e
                // desabilita o autostart. Esconder isso confundiria o usuário.
                if self.state.portable_mode {
                    ui.label(
                        egui::RichText::new(match self.state.language {
                            Language::PtBr => "MODO PORTATIL",
                            Language::English => "PORTABLE MODE",
                        })
                        .size(10.0)
                        .strong()
                        .color(ST_WARN),
                    );
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                let items = [
                    (Tab::Network, match self.state.language {
                        Language::PtBr => "MCP & Rede",
                        Language::English => "MCP & Network",
                    }),
                    (Tab::Tunnel, match self.state.language {
                        Language::PtBr => "Túnel",
                        Language::English => "Tunnel",
                    }),
                    (Tab::McpTools, "MCP Tools"),
                    (Tab::Calibration, match self.state.language {
                        Language::PtBr => "Calibração",
                        Language::English => "Calibration",
                    }),
                    (Tab::Windows, match self.state.language {
                        Language::PtBr => "Janelas",
                        Language::English => "Windows",
                    }),
                    (Tab::Recording, match self.state.language {
                        Language::PtBr => "Gravação",
                        Language::English => "Recording",
                    }),
                    (Tab::DoctorSkills, "Doctor & Skills"),
                ];

                for (tab, label) in items {
                    let selected = self.state.active_tab == tab;
                    // Item de menu: barra verde à esquerda quando ativo.
                    let (rect, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 28.0),
                        egui::Sense::click(),
                    );
                    let hovered = resp.hovered();
                    let p = ui.painter();
                    if selected {
                        p.rect_filled(rect, 2.0, Color32::from_rgb(16, 40, 24));
                        p.rect_filled(
                            egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                            0.0,
                            TERM_GREEN_BRIGHT,
                        );
                    } else if hovered {
                        p.rect_filled(rect, 2.0, Color32::from_rgb(10, 22, 14));
                    }
                    let txt_color = if selected {
                        TERM_GREEN_BRIGHT
                    } else if hovered {
                        TERM_GREEN
                    } else {
                        TERM_WHITE
                    };
                    p.text(
                        egui::pos2(rect.min.x + 12.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::monospace(13.0),
                        txt_color,
                    );
                    if resp.clicked() {
                        self.state.active_tab = tab;
                    }
                    ui.add_space(2.0);
                }

                // Pé da sidebar: status REAL + idioma + Sobre.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    if ui.add(term_button(match self.state.language {
                        Language::PtBr => "Ajuda & Sobre",
                        Language::English => "Help & About",
                    })).clicked() {
                        self.state.show_about = true;
                    }
                    if ui.add(term_button(match self.state.language {
                        Language::PtBr => "EN | English",
                        Language::English => "PT | Português",
                    })).clicked() {
                        self.state.language = match self.state.language {
                            Language::PtBr => Language::English,
                            Language::English => Language::PtBr,
                        };
                    }

                    ui.add_space(8.0);

                    // Chip de túnel ativo: a máquina está exposta à internet —
                    // visível de QUALQUER aba, não só na aba Túnel.
                    if matches!(
                        self.state.tunnel_status,
                        TunnelStatus::Starting | TunnelStatus::Running
                    ) {
                        let tunnel_color = Color32::from_rgb(255, 112, 67);
                        ui.horizontal(|ui| {
                            crate::app::status_dot(ui, tunnel_color);
                            ui.label(
                                egui::RichText::new(match self.state.language {
                                    Language::PtBr => "TUNEL ATIVO",
                                    Language::English => "TUNNEL ACTIVE",
                                })
                                .color(tunnel_color)
                                .strong()
                                .size(11.0),
                            );
                        });
                    }

                    // Status do MCP: mesmo critério honesto de sempre — verde
                    // só com LAN confirmada por netstat + POST initialize real.
                    let (status_txt, status_color) = match self.state.port_status {
                        crate::app::PortStatus::LanListening => (
                            match self.state.language {
                                Language::PtBr => format!("MCP local+LAN :{}", self.state.http_port),
                                Language::English => format!("MCP local+LAN :{}", self.state.http_port),
                            },
                            ST_OK,
                        ),
                        crate::app::PortStatus::LocalOnly => (
                            match self.state.language {
                                Language::PtBr => format!("MCP local :{}", self.state.http_port),
                                Language::English => format!("MCP local :{}", self.state.http_port),
                            },
                            ST_WARN,
                        ),
                        crate::app::PortStatus::Stopped => (
                            match self.state.language {
                                Language::PtBr => "MCP parado".to_string(),
                                Language::English => "MCP stopped".to_string(),
                            },
                            ST_ERR,
                        ),
                    };
                    ui.horizontal(|ui| {
                        crate::app::status_dot(ui, status_color);
                        ui.label(
                            egui::RichText::new(status_txt)
                                .color(status_color)
                                .strong()
                                .size(11.0),
                        );
                    });
                });
            });

        // Rodapé
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(12.0, 7.0))
                    .fill(Color32::from_rgb(3, 5, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Adaptativo: em janela estreita o texto da esquerda
                    // colidia com o bloco da direita — some quando nao ha
                    // largura para os dois.
                    if ui.available_width() > 720.0 {
                        ui.label(
                            egui::RichText::new(
                                "\"im not antisocial, im just not user friendly\"",
                            )
                            .size(11.0)
                            .italics()
                            .color(TERM_GREEN_DIM),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Roger Luft (VeilWalker)")
                                .size(11.0)
                                .color(TERM_GRAY),
                        );
                    });
                });
            });

        // ─── CONSOLE GLOBAL ÚNICO (faixa fixa acima do rodapé) ───
        // Um só console para TODAS as abas. Antes cada aba tinha a sua caixa de
        // saída e a aba MCP & Rede tinha o Console Debug — dois consoles com a
        // mesma informação na mesma tela. Comportamento de `tail -f`: acompanha
        // o fim do log sozinho, mas PARA de acompanhar quando o usuário rola
        // para cima (para poder ler) e volta a acompanhar ao retornar ao fim.
        egui::TopBottomPanel::bottom("global_console")
            .resizable(true)
            .default_height(170.0)
            .min_height(96.0)
            .frame(
                egui::Frame::none()
                    .inner_margin(10.0)
                    .fill(Color32::from_rgb(18, 18, 18)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(match self.state.language {
                            Language::PtBr => "Console",
                            Language::English => "Console",
                        })
                        .size(13.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                    // Indicador do modo de rolagem — estado real, não presumido.
                    let (follow_txt, follow_color) = if self.state.console_follow {
                        (
                            match self.state.language {
                                Language::PtBr => "seguindo",
                                Language::English => "following",
                            },
                            Color32::from_rgb(76, 175, 80),
                        )
                    } else {
                        (
                            match self.state.language {
                                Language::PtBr => "pausado (rolagem manual)",
                                Language::English => "paused (manual scroll)",
                            },
                            Color32::from_rgb(255, 193, 7),
                        )
                    };
                    crate::app::status_dot(ui, follow_color);
                    ui.label(
                        egui::RichText::new(follow_txt)
                            .size(10.0)
                            .color(follow_color),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(match self.state.language {
                                Language::PtBr => "Limpar",
                                Language::English => "Clear",
                            })
                            .clicked()
                        {
                            self.state.debug_log.clear();
                            self.state.status_msg.clear();
                        }
                        if ui
                            .button(match self.state.language {
                                Language::PtBr => "Copiar",
                                Language::English => "Copy",
                            })
                            .clicked()
                        {
                            let log = self.state.debug_log.clone();
                            ui.output_mut(|o| o.copied_text = log);
                        }
                        if ui
                            .button(match self.state.language {
                                Language::PtBr => "Ir ao fim",
                                Language::English => "Jump to end",
                            })
                            .clicked()
                        {
                            // console_jump (e não só console_follow): a detecção
                            // de posição no fim deste mesmo frame sobrescrevia o
                            // follow com a posição ANTIGA do scroll e anulava o
                            // clique — o botão não fazia nada.
                            self.state.console_jump = true;
                            self.state.console_follow = true;
                        }
                    });
                });

                // Faixa da ÚLTIMA mensagem (o que antes era a caixa de saída de
                // cada aba). Fica sempre à vista, sem duplicar o histórico.
                if !self.state.status_msg.trim().is_empty() {
                    ui.add_space(4.0);
                    let msg = self.state.status_msg.trim();
                    // Só a primeira linha na faixa; o corpo inteiro está no log.
                    let first = msg.lines().next().unwrap_or("");
                    let extra = msg.lines().count().saturating_sub(1);
                    let shown = if extra > 0 {
                        format!("{}  (+{} linhas no log)", first, extra)
                    } else {
                        first.to_string()
                    };
                    ui.label(
                        egui::RichText::new(shown)
                            .size(11.0)
                            .color(Color32::from_rgb(255, 213, 79)),
                    );
                }

                ui.add_space(4.0);

                let jumping = self.state.console_jump;
                let out = egui::ScrollArea::vertical()
                    .id_salt("global_console_scroll")
                    .auto_shrink([false, false])
                    .stick_to_bottom(self.state.console_follow || jumping)
                    .show(ui, |ui| {
                        if self.state.debug_log.is_empty() {
                            ui.monospace(match self.state.language {
                                Language::PtBr => "(nenhum comando executado ainda)",
                                Language::English => "(no command executed yet)",
                            });
                        } else {
                            ui.monospace(&self.state.debug_log);
                        }
                    });

                // Detecta se o usuário está no fim: se sim, segue; se rolou
                // para cima, pausa. É o comportamento de um tail de log.
                // No frame do "Ir ao fim" a posição lida ainda é a ANTIGA — por
                // isso o salto tem precedência e só o frame seguinte volta a
                // decidir pela posição real.
                if jumping {
                    self.state.console_jump = false;
                    self.state.console_follow = true;
                } else {
                    let max_offset = (out.content_size.y - out.inner_rect.height()).max(0.0);
                    self.state.console_follow = out.state.offset.y >= max_offset - 8.0;
                }
            });

        // ─── PAINEL CENTRAL: só o conteúdo da seção ativa ───
        // A navegação virou a barra lateral (SidePanel "nav"). A antiga fileira
        // de abas "pill" foi removida: com 7 itens ela quebrava em duas
        // fileiras e comia altura útil da tela.
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(14.0).fill(TERM_BG))
            .show(ctx, |ui| {
                match self.state.active_tab {
                    Tab::Network => crate::tabs::network::render(ui, &mut self.state),
                    Tab::Calibration => crate::tabs::calibration::render(ui, &mut self.state),
                    Tab::Windows => crate::tabs::windows::render(ui, &mut self.state),
                    Tab::Recording => crate::tabs::recording::render(ui, &mut self.state),
                    Tab::DoctorSkills => crate::tabs::doctor_skills::render(ui, &mut self.state),
                    Tab::McpTools => crate::tabs::mcp_tools::render(ui, &mut self.state),
                    Tab::Tunnel => crate::tabs::tunnel::render(ui, &mut self.state),
                }
            });

        if self.state.show_about {
            let lang = self.state.language;
            let mut open = true;
            let mut close_clicked = false;

            egui::Window::new(match lang {
                Language::PtBr => "Sobre o FzComputerAI & Suporte",
                Language::English => "About FzComputerAI & Support",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .open(&mut open)
            .show(ctx, |ui| {
                ui.heading("FZComputerAI — Grupo FazAI");
                ui.label(concat!("Versão / Version: v", env!("CARGO_PKG_VERSION")));
                ui.add_space(4.0);
                ui.label(match lang {
                    Language::PtBr => "Servidor Nativo de Visão Computacional, MCP & Hub CLI",
                    Language::English => "Native Computer Vision Server, MCP & CLI Hub",
                });
                ui.label(match lang {
                    Language::PtBr => "Desenvolvido por: Roger Luft (VeilWalker) <roger@webstorage.com.br>",
                    Language::English => "Developed by: Roger Luft (VeilWalker) <roger@webstorage.com.br>",
                });

                // ─── DONATE: apoio ao projeto via GitHub Sponsors ───
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 34, 22))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Coracao DESENHADO (a fonte padrao nao tem glifo de
                            // coracao/emoji — viraria caixa quebrada, igual ao "●").
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(18.0, 18.0),
                                egui::Sense::hover(),
                            );
                            let p = ui.painter();
                            let c = rect.center();
                            let r = 4.2;
                            let pink = Color32::from_rgb(233, 30, 99);
                            p.circle_filled(c + Vec2::new(-r * 0.55, -r * 0.35), r * 0.75, pink);
                            p.circle_filled(c + Vec2::new(r * 0.55, -r * 0.35), r * 0.75, pink);
                            p.add(egui::Shape::convex_polygon(
                                vec![
                                    c + Vec2::new(-r * 1.25, -r * 0.10),
                                    c + Vec2::new(r * 1.25, -r * 0.10),
                                    c + Vec2::new(0.0, r * 1.45),
                                ],
                                pink,
                                egui::Stroke::NONE,
                            ));
                            ui.label(
                                egui::RichText::new(match lang {
                                    Language::PtBr => "APOIE O PROJETO / DONATE",
                                    Language::English => "SUPPORT THE PROJECT / DONATE",
                                })
                                .size(14.0)
                                .strong()
                                .color(Color32::from_rgb(129, 199, 132)),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(match lang {
                                Language::PtBr => "O FzComputerAI é software livre (MIT). Se ele te ajuda, considere apoiar o desenvolvimento — qualquer valor mantém o projeto vivo.",
                                Language::English => "FzComputerAI is free software (MIT). If it helps you, consider supporting development — any amount keeps the project alive.",
                            })
                            .size(11.0)
                            .color(Color32::from_rgb(170, 190, 170)),
                        );
                        ui.add_space(8.0);
                        const SPONSOR_URL: &str = "https://github.com/sponsors/RLuf";
                        ui.horizontal(|ui| {
                            let donate_btn = egui::Button::new(
                                egui::RichText::new(match lang {
                                    Language::PtBr => "Doar no GitHub Sponsors",
                                    Language::English => "Donate on GitHub Sponsors",
                                })
                                .color(Color32::WHITE)
                                .strong(),
                            )
                            .fill(Color32::from_rgb(233, 30, 99))
                            .min_size(Vec2::new(190.0, 30.0))
                            .rounding(egui::Rounding::same(6.0));
                            if ui.add(donate_btn).clicked() {
                                let _ = open::that(SPONSOR_URL);
                                self.state.log_debug(&format!(
                                    "[donate] Abrindo {} no navegador.",
                                    SPONSOR_URL
                                ));
                            }
                            if ui
                                .button(match lang {
                                    Language::PtBr => "Copiar link",
                                    Language::English => "Copy link",
                                })
                                .clicked()
                            {
                                ui.output_mut(|o| o.copied_text = SPONSOR_URL.to_string());
                            }
                        });
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(SPONSOR_URL)
                                .size(10.0)
                                .color(Color32::from_rgb(120, 140, 120)),
                        );
                    });

                ui.add_space(10.0);
                ui.label(match lang {
                    Language::PtBr => "Patrocinadores Oficiais:",
                    Language::English => "Official Sponsors:",
                });
                ui.hyperlink_to("Webstorage Tecnologia", "https://www.webstorage.com.br");
                ui.hyperlink_to("Imóvel Site", "https://www.imovelsite.com.br");

                // ─── Credito ao projeto Cua (motor cua-driver, MIT) ───
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "Motor de automação: projeto Cua (cua-driver)",
                        Language::English => "Automation engine: Cua project (cua-driver)",
                    })
                    .size(12.0)
                    .strong(),
                );
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "MIT License — Copyright (c) 2025 Cua AI, Inc. Obrigado à Cua AI e à comunidade do Cua: o cua-driver é a base deste projeto.",
                        Language::English => "MIT License — Copyright (c) 2025 Cua AI, Inc. Thanks to Cua AI and the Cua community: cua-driver is the foundation of this project.",
                    })
                    .size(10.0)
                    .color(Color32::from_rgb(150, 150, 150)),
                );
                ui.hyperlink_to("github.com/trycua/cua", "https://github.com/trycua/cua");
                ui.add_space(12.0);
                if ui.button(match lang {
                    Language::PtBr => "Fechar",
                    Language::English => "Close",
                }).clicked() {
                    close_clicked = true;
                }
            });

            if !open || close_clicked {
                self.state.show_about = false;
            }
        }

        // ─── CENTRAL DE ATUALIZAÇÕES: GUI + MOTOR num só lugar ───
        // Abre após "Verificar Atualizações" e mostra o estado REAL dos dois
        // componentes (versão instalada x publicada), com o botão de ação de
        // cada um. Antes este diálogo só falava da GUI e o motor ficava
        // invisível — foi assim que o motor chegou a 9 versões de atraso.
        if self.state.update_checked && !self.state.update_ready {
            let lang = self.state.language;
            let tag = self.state.update_available.clone().unwrap_or_default();
            let gui_has = self.state.update_available.is_some();
            let drv_has = self.state.driver_update_available;
            let drv_cur = self.state.driver_version.clone();
            let drv_new = self.state.driver_latest.clone();
            let drv_notes = self.state.driver_notes_url.clone();
            let downloading = self.state.update_downloading;
            let drv_updating = self.state.driver_updating;
            let mut do_download = false;
            let mut do_driver = false;
            let mut do_dismiss = false;

            egui::Window::new(match lang {
                Language::PtBr => "Central de Atualizações",
                Language::English => "Update Center",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                // ── Componente 1: a interface ──
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "Interface (FzComputerAI)",
                        Language::English => "Interface (FzComputerAI)",
                    })
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
                );
                if gui_has {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!(
                                "instalada v{}  ->  disponivel {}",
                                env!("CARGO_PKG_VERSION"),
                                tag
                            ),
                            Language::English => format!(
                                "installed v{}  ->  available {}",
                                env!("CARGO_PKG_VERSION"),
                                tag
                            ),
                        })
                        .color(ST_WARN),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!("v{} — atualizada", env!("CARGO_PKG_VERSION")),
                            Language::English => format!("v{} — up to date", env!("CARGO_PKG_VERSION")),
                        })
                        .color(ST_OK),
                    );
                }
                if gui_has {
                    ui.add_space(4.0);
                    if downloading {
                        ui.label(
                            egui::RichText::new(match lang {
                                Language::PtBr => "baixando o instalador em segundo plano...",
                                Language::English => "downloading the installer in the background...",
                            })
                            .color(TERM_GRAY),
                        );
                    } else if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Baixar instalador",
                            Language::English => "Download installer",
                        }))
                        .clicked()
                    {
                        do_download = true;
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // ── Componente 2: o motor ──
                ui.label(
                    egui::RichText::new(match lang {
                        Language::PtBr => "Motor (cua-driver)",
                        Language::English => "Engine (cua-driver)",
                    })
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
                );
                if drv_cur.is_empty() {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => "nao foi possivel consultar o motor (instalado? no PATH?)",
                            Language::English => "could not query the engine (installed? on PATH?)",
                        })
                        .color(ST_ERR),
                    );
                } else if drv_has {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!("instalado {}  ->  disponivel {}", drv_cur, drv_new),
                            Language::English => format!("installed {}  ->  available {}", drv_cur, drv_new),
                        })
                        .color(ST_WARN),
                    );
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => "Versoes novas do motor podem mudar o contrato do endpoint HTTP (ex.: exigir token). Apos atualizar, confira o estado na aba MCP & Rede.",
                            Language::English => "Newer engine versions may change the HTTP endpoint contract (e.g. require a token). After updating, check the status in the MCP & Network tab.",
                        })
                        .size(11.0)
                        .color(TERM_GRAY),
                    );
                    if !drv_notes.is_empty() {
                        ui.hyperlink_to(
                            match lang {
                                Language::PtBr => "notas da versao",
                                Language::English => "release notes",
                            },
                            &drv_notes,
                        );
                    }
                    ui.add_space(4.0);
                    if drv_updating {
                        ui.label(
                            egui::RichText::new(match lang {
                                Language::PtBr => "atualizando o motor em segundo plano...",
                                Language::English => "updating the engine in the background...",
                            })
                            .color(TERM_GRAY),
                        );
                    } else if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Atualizar motor",
                            Language::English => "Update engine",
                        }))
                        .clicked()
                    {
                        do_driver = true;
                    }
                } else {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => format!("{} — atualizado", drv_cur),
                            Language::English => format!("{} — up to date", drv_cur),
                        })
                        .color(ST_OK),
                    );
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Verificar de novo",
                            Language::English => "Check again",
                        }))
                        .clicked()
                    {
                        do_dismiss = false; // mantem aberto; recheca abaixo
                        self.state.check_for_updates();
                    }
                    if ui
                        .add(term_button(match lang {
                            Language::PtBr => "Fechar",
                            Language::English => "Close",
                        }))
                        .clicked()
                    {
                        do_dismiss = true;
                    }
                });
            });

            if do_download {
                self.state.start_update_download();
            }
            if do_driver {
                self.state.start_driver_update();
            }
            if do_dismiss {
                self.state.update_checked = false;
                self.state.update_available = None;
            }
        }

        // ─── Dialogo 2 do upgrade: download pronto — fechar e instalar? ───
        if self.state.update_ready {
            let lang = self.state.language;
            // NUNCA oferecer a troca da GUI com o motor NO MEIO de uma
            // atualizacao: install_update_and_restart mata cua-driver.exe
            // (derrubaria o update --apply na troca de junction) e o setup
            // /VERYSILENT rodaria um segundo instalador do motor em corrida
            // com o fallback ainda ativo — motor corrompido/indefinido.
            let engine_busy = self.state.driver_updating;
            let mut do_install = false;
            let mut do_dismiss = false;

            egui::Window::new(match lang {
                Language::PtBr => "Pronto para atualizar",
                Language::English => "Ready to update",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.label(match lang {
                    Language::PtBr => "Download concluído em diretório temporário.\nPara instalar, o FzComputerAI precisa ser FECHADO.\nApós a instalação, o aplicativo e o motor cua-driver serão reabertos automaticamente.",
                    Language::English => "Download finished in a temporary directory.\nTo install, FzComputerAI must be CLOSED.\nAfter installation, the app and the cua-driver engine will be reopened automatically.",
                });
                ui.add_space(8.0);
                if engine_busy {
                    ui.label(
                        egui::RichText::new(match lang {
                            Language::PtBr => "Aguardando a atualizacao do motor terminar antes de trocar a GUI (evita corromper o motor no meio da troca)...",
                            Language::English => "Waiting for the engine update to finish before swapping the GUI (prevents corrupting the engine mid-swap)...",
                        })
                        .color(ST_WARN),
                    );
                    ui.add_space(4.0);
                }
                ui.horizontal(|ui| {
                    if !engine_busy
                        && ui
                            .button(match lang {
                                Language::PtBr => "Fechar e instalar agora",
                                Language::English => "Close and install now",
                            })
                            .clicked()
                    {
                        do_install = true;
                    }
                    if ui
                        .button(match lang {
                            Language::PtBr => "Depois",
                            Language::English => "Later",
                        })
                        .clicked()
                    {
                        do_dismiss = true;
                    }
                });
            });

            if do_install {
                self.state.install_update_and_restart();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if do_dismiss {
                // "Depois" e DEPOIS de verdade: zera tambem update_checked,
                // senao a Central reabria no frame seguinte dizendo
                // "atualizada" (falso — o instalador verificado esta no TEMP)
                // e voltava a importunar a cada verificacao.
                self.state.update_ready = false;
                self.state.update_available = None;
                self.state.update_checked = false;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// HTTPS do endpoint MCP — orquestração (a mecânica TLS/ACME vive em tls.rs)
// ═══════════════════════════════════════════════════════════════════════
impl AppState {
    fn tls_mode_str(m: TlsMode) -> &'static str {
        match m {
            TlsMode::SelfSigned => "selfsigned",
            TlsMode::LetsEncrypt => "letsencrypt",
            TlsMode::Custom => "custom",
        }
    }
    fn tls_bind_str(b: TlsBind) -> &'static str {
        match b {
            TlsBind::Loopback => "loopback",
            TlsBind::Lan => "lan",
            TlsBind::All => "all",
        }
    }

    /// Lê as preferências tlscfg:* (registro ou .ini portátil).
    pub fn tls_read_cfg(&mut self) {
        if let Some(v) = self.cfg_get("tlscfg:enabled") {
            self.tls_enabled = v == "1";
        }
        if let Some(v) = self.cfg_get("tlscfg:port") {
            if v.parse::<u16>().is_ok() {
                self.tls_port = v;
            }
        }
        if let Some(v) = self.cfg_get("tlscfg:bind") {
            self.tls_bind = match v.as_str() {
                "loopback" => TlsBind::Loopback,
                "all" => TlsBind::All,
                _ => TlsBind::Lan,
            };
        }
        if let Some(v) = self.cfg_get("tlscfg:mode") {
            self.tls_mode = match v.as_str() {
                "letsencrypt" => TlsMode::LetsEncrypt,
                "custom" => TlsMode::Custom,
                _ => TlsMode::SelfSigned,
            };
        }
        if let Some(v) = self.cfg_get("tlscfg:domain") {
            self.tls_domain = v;
        }
        if let Some(v) = self.cfg_get("tlscfg:email") {
            self.tls_email = v;
        }
        if let Some(v) = self.cfg_get("tlscfg:staging") {
            self.tls_staging = v == "1";
        }
        if let Some(v) = self.cfg_get("tlscfg:custom_cert") {
            self.tls_custom_cert = v;
        }
        if let Some(v) = self.cfg_get("tlscfg:custom_key") {
            self.tls_custom_key = v;
        }
    }

    /// Persiste as preferências tlscfg:* (nunca chave privada — só caminhos).
    pub fn tls_save_cfg(&mut self) {
        let enabled = if self.tls_enabled { "1" } else { "0" }.to_string();
        let port = self.tls_port.trim().to_string();
        let bind = Self::tls_bind_str(self.tls_bind).to_string();
        let mode = Self::tls_mode_str(self.tls_mode).to_string();
        let domain = self.tls_domain.trim().to_string();
        let email = self.tls_email.trim().to_string();
        let staging = if self.tls_staging { "1" } else { "0" }.to_string();
        let cc = self.tls_custom_cert.trim().to_string();
        let ck = self.tls_custom_key.trim().to_string();
        self.cfg_set("tlscfg:enabled", &enabled);
        self.cfg_set("tlscfg:port", &port);
        self.cfg_set("tlscfg:bind", &bind);
        self.cfg_set("tlscfg:mode", &mode);
        self.cfg_set("tlscfg:domain", &domain);
        self.cfg_set("tlscfg:email", &email);
        self.cfg_set("tlscfg:staging", &staging);
        self.cfg_set("tlscfg:custom_cert", &cc);
        self.cfg_set("tlscfg:custom_key", &ck);
        self.log_debug("[https] Configuracao salva (tlscfg:*).");
    }

    /// Startup: preferências + auto-assinado garantido + listener se ligado.
    pub fn tls_startup(&mut self) {
        self.tls_cert_dir = crate::tls::cert_dir(self.portable_mode);
        self.tls_read_cfg();
        // "Na instalação ou no primeiro run, o que vier primeiro": o instalador
        // chama `--tls-init`; se ele não rodou (portátil, Linux, upgrade
        // antigo), o primeiro run gera aqui. Idempotente: cert válido que já
        // cobre os SANs é mantido (o fingerprint não muda à toa).
        let sans = crate::tls::default_sans(&self.lan_ip, &self.tls_domain);
        match crate::tls::ensure_self_signed(&self.tls_cert_dir, &sans, false) {
            Ok((crt, _key, generated)) => {
                let dir = self.tls_cert_dir.display().to_string();
                if generated {
                    self.log_debug(&format!(
                        "[https] Certificado auto-assinado GERADO em {} (SANs: {}). Ele NAO e instalado em nenhuma store de confianca — o cliente confia pelo fingerprint SHA-256 ou pelo arquivo .crt.",
                        dir,
                        sans.join(", ")
                    ));
                } else {
                    self.log_debug(&format!("[https] Certificado auto-assinado presente e valido em {}.", dir));
                }
                if let Ok(info) = crate::tls::inspect_cert_file(&crt) {
                    self.log_debug(&format!(
                        "[https] auto-assinado: valido ate {} ({} dias) — SHA-256 {}",
                        info.not_after, info.days_left, info.sha256_fingerprint
                    ));
                }
            }
            Err(e) => {
                self.tls_last_error = format!("{e:#}");
                self.log_debug(&format!("[https] FALHA ao gerar o auto-assinado: {e:#}"));
            }
        }
        self.tls_refresh_cert_info();
        if self.tls_enabled {
            self.start_tls();
        } else {
            self.log_debug("[https] Listener HTTPS desligado (preferencia). Ligue em MCP & Rede > HTTPS.");
        }
    }

    /// Resolve (crt, key) conforme o modo. Não gera nada aqui além do
    /// auto-assinado (o Let's Encrypt exige emissão explícita).
    fn tls_resolve_cert(&mut self) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
        let dir = self.tls_cert_dir.clone();
        match self.tls_mode {
            TlsMode::SelfSigned => {
                let sans = crate::tls::default_sans(&self.lan_ip, &self.tls_domain);
                crate::tls::ensure_self_signed(&dir, &sans, false)
                    .map(|(c, k, _)| (c, k))
                    .map_err(|e| format!("{e:#}"))
            }
            TlsMode::LetsEncrypt => {
                let crt = dir.join(crate::tls::ACME_CERT);
                let key = dir.join(crate::tls::ACME_KEY);
                if !crt.exists() || !key.exists() {
                    return Err(self.tr(
                        "Nenhum certificado Let's Encrypt emitido ainda — clique em \"Emitir Let's Encrypt\".",
                        "No Let's Encrypt certificate issued yet — click \"Issue Let's Encrypt\".",
                    ));
                }
                match crate::tls::inspect_cert_file(&crt) {
                    Ok(i) if i.expired() => Err(self.tr(
                        "O certificado Let's Encrypt EXPIROU — emita de novo.",
                        "The Let's Encrypt certificate has EXPIRED — issue it again.",
                    )),
                    Ok(_) => Ok((crt, key)),
                    Err(e) => Err(format!("{e:#}")),
                }
            }
            TlsMode::Custom => {
                let crt = std::path::PathBuf::from(self.tls_custom_cert.trim());
                let key = std::path::PathBuf::from(self.tls_custom_key.trim());
                if !crt.is_file() || !key.is_file() {
                    return Err(self.tr(
                        "Informe caminhos validos para o .crt e a .key (PEM).",
                        "Provide valid paths for the .crt and .key (PEM).",
                    ));
                }
                Ok((crt, key))
            }
        }
    }

    fn tls_bind_ip(&self) -> String {
        match self.tls_bind {
            TlsBind::Loopback => "127.0.0.1".to_string(),
            TlsBind::All => "0.0.0.0".to_string(),
            TlsBind::Lan => {
                let ip = self.lan_ip.trim();
                if ip.is_empty() { "0.0.0.0".to_string() } else { ip.to_string() }
            }
        }
    }

    /// Sobe o listener HTTPS. Não mexe no motor nem no LAN forward.
    pub fn start_tls(&mut self) {
        self.stop_tls();
        let tls_port: u16 = match self.tls_port.trim().parse() {
            Ok(p) => p,
            Err(_) => {
                self.tls_status = TlsStatus::Error;
                self.tls_last_error = self.tr("Porta HTTPS invalida.", "Invalid HTTPS port.");
                self.log_debug("[https] Porta HTTPS invalida.");
                return;
            }
        };
        let http_port: u16 = self.http_port.trim().parse().unwrap_or(8000);
        if tls_port == http_port {
            self.tls_status = TlsStatus::Error;
            self.tls_last_error = self.tr(
                "A porta HTTPS nao pode ser a mesma do HTTP do motor.",
                "The HTTPS port cannot be the same as the engine's HTTP port.",
            );
            self.log_debug("[https] Porta HTTPS igual a porta HTTP do motor — recusado.");
            return;
        }
        let (crt, key) = match self.tls_resolve_cert() {
            Ok(p) => p,
            Err(e) => {
                self.tls_status = TlsStatus::Error;
                self.log_debug(&format!("[https] Certificado indisponivel: {e}"));
                self.tls_last_error = e;
                return;
            }
        };
        let bind = self.tls_bind_ip();
        match crate::tls::start_tls_proxy(&bind, tls_port, http_port, &crt, &key) {
            Ok(h) => {
                self.tls_cert_path = crt.display().to_string();
                self.tls_proxy = Some(h);
                self.tls_last_error.clear();
                self.log_debug(&format!(
                    "[https] Listener HTTPS ATIVO dentro do app: https://{}:{}/mcp -> http://127.0.0.1:{}/mcp (cert: {}). O bearer token continua obrigatorio.",
                    bind, tls_port, http_port, crt.display()
                ));
                self.tls_refresh_cert_info();
                self.check_tls_status();
            }
            Err(e) => {
                self.tls_status = TlsStatus::Error;
                self.tls_last_error = format!("{e:#}");
                self.log_debug(&format!("[https] FALHA ao subir o listener HTTPS: {e:#}"));
            }
        }
    }

    pub fn stop_tls(&mut self) {
        if let Some(h) = self.tls_proxy.take() {
            h.stop();
            self.log_debug("[https] Listener HTTPS encerrado.");
        }
        self.tls_status = TlsStatus::Stopped;
        self.tls_probe = None;
        self.tls_probe_lan_ok = false;
    }

    /// Lê o certificado ATIVO do disco para a tela (SANs, validade, SHA-256).
    pub fn tls_refresh_cert_info(&mut self) {
        let path = match self.tls_mode {
            TlsMode::SelfSigned => self.tls_cert_dir.join(crate::tls::SELF_SIGNED_CERT),
            TlsMode::LetsEncrypt => self.tls_cert_dir.join(crate::tls::ACME_CERT),
            TlsMode::Custom => std::path::PathBuf::from(self.tls_custom_cert.trim()),
        };
        self.tls_cert_path = path.display().to_string();
        self.tls_cert_info = crate::tls::inspect_cert_file(&path).ok();
    }

    /// Sonda REAL do HTTPS: handshake + POST initialize (com o token) em
    /// 127.0.0.1 e, quando o bind não é loopback, também no IP da LAN.
    pub fn check_tls_status(&mut self) {
        let Some(h) = self.tls_proxy.as_ref() else {
            self.tls_status = TlsStatus::Stopped;
            self.tls_probe = None;
            return;
        };
        let port = h.port;
        let bind = h.bind_ip.clone();
        self.tls_accepted = h.accepted_count();
        let err = h.take_last_error();
        if !err.is_empty() {
            self.log_debug(&format!("[https] Ultimo erro de conexao no listener: {err}"));
        }
        let token = self.mcp_token.clone();
        let sni = if self.tls_domain.trim().is_empty() { "localhost".to_string() } else { self.tls_domain.trim().to_string() };

        // 1) loopback (sempre, quando o bind cobre 127.0.0.1) — senão o IP do bind
        let local_target = if bind == "127.0.0.1" || bind == "0.0.0.0" { "127.0.0.1".to_string() } else { bind.clone() };
        let p = crate::tls::probe_https(&local_target, port, &sni, &token);
        self.log_debug(&format!(
            "> https-probe {}:{} (SNI {})\n  tls: {}  proto: {}  http: {}  jsonrpc: {}  {}",
            local_target, port, sni,
            if p.tls_ok { "OK" } else { "FALHOU" },
            p.protocol, p.http_status, p.jsonrpc, p.detail
        ));
        // 2) LAN, quando o bind é 0.0.0.0 (o IP da LAN é outro endereço).
        let lan_ip = self.lan_ip.trim().to_string();
        self.tls_probe_lan_ok = if bind == "0.0.0.0" && lan_ip != "127.0.0.1" && !lan_ip.is_empty() {
            let pl = crate::tls::probe_https(&lan_ip, port, &sni, &token);
            self.log_debug(&format!(
                "> https-probe {}:{}\n  tls: {}  http: {}  jsonrpc: {}",
                lan_ip, port, if pl.tls_ok { "OK" } else { "FALHOU" }, pl.http_status, pl.jsonrpc
            ));
            pl.tls_ok && pl.jsonrpc
        } else {
            p.tls_ok && p.jsonrpc && bind != "127.0.0.1"
        };
        self.tls_status = if p.tls_ok && p.jsonrpc {
            TlsStatus::Listening
        } else if p.tls_ok {
            TlsStatus::ListeningNoMcp
        } else {
            TlsStatus::Error
        };
        if p.tls_ok && !p.jsonrpc {
            self.log_debug(&format!(
                "[https] TLS OK mas o motor nao respondeu JSON-RPC atras do listener (HTTP {}). Motor parado? Token? O HTTPS so encaminha — ele nao substitui o motor.",
                p.http_status
            ));
        }
        if let Some(c) = &p.cert {
            if c.needs_renewal() {
                self.log_debug(&format!(
                    "[https] AVISO: o certificado servido expira em {} dias ({}).",
                    c.days_left, c.not_after
                ));
            }
        }
        self.tls_probe = Some(p);
    }

    /// Liga/desliga pelo checkbox: persiste e aplica.
    pub fn set_tls_enabled(&mut self, on: bool) {
        self.tls_enabled = on;
        self.tls_save_cfg();
        if on { self.start_tls(); } else { self.stop_tls(); }
    }

    /// Regenera o auto-assinado (novo fingerprint) e recarrega o listener.
    pub fn tls_regenerate_self_signed(&mut self) {
        let sans = crate::tls::default_sans(&self.lan_ip, &self.tls_domain);
        match crate::tls::ensure_self_signed(&self.tls_cert_dir, &sans, true) {
            Ok((crt, _, _)) => {
                let fp = crate::tls::inspect_cert_file(&crt).map(|i| i.sha256_fingerprint).unwrap_or_default();
                self.log_debug(&format!(
                    "[https] Auto-assinado REGENERADO (SANs: {}). Novo SHA-256: {}. Clientes com pin do fingerprint antigo precisam atualizar. O par anterior ficou em selfsigned.prev.*",
                    sans.join(", "), fp
                ));
                self.status_msg = self.tr("Certificado auto-assinado regenerado.", "Self-signed certificate regenerated.");
            }
            Err(e) => {
                self.tls_last_error = format!("{e:#}");
                self.log_debug(&format!("[https] FALHA ao regenerar: {e:#}"));
            }
        }
        self.tls_refresh_cert_info();
        if self.tls_proxy.is_some() && self.tls_mode == TlsMode::SelfSigned {
            self.start_tls();
        }
    }

    /// Dispara a emissão Let's Encrypt (thread + runtime tokio próprio).
    pub fn tls_issue_letsencrypt(&mut self) {
        if self.tls_acme_busy {
            return;
        }
        let domain = self.tls_domain.trim().to_string();
        if domain.is_empty() {
            self.status_msg = self.tr("Informe o dominio publico antes de emitir.", "Enter the public domain before issuing.");
            self.log_debug("[acme] Dominio vazio — nada a emitir.");
            return;
        }
        self.tls_save_cfg();
        self.log_debug(&format!(
            "[acme] Emissao Let's Encrypt para {} — pre-requisitos: DNS do dominio apontando para o IP publico desta maquina e porta 80 alcancavel da internet (o app abre um respondedor HTTP-01 temporario em 0.0.0.0:80).{}",
            domain,
            if self.tls_staging { " MODO STAGING: certificado de TESTE, nao confiavel." } else { "" }
        ));
        let rx = crate::tls::acme_issue_async(crate::tls::AcmeRequest {
            domain,
            email: self.tls_email.trim().to_string(),
            staging: self.tls_staging,
            dir: self.tls_cert_dir.clone(),
        });
        self.tls_acme_rx = Some(rx);
        self.tls_acme_busy = true;
    }

    /// update(): drena eventos ACME e faz a checagem de renovação (6 em 6 h).
    pub fn poll_tls(&mut self) {
        // Eventos da emissão
        let mut done: Option<anyhow::Result<(std::path::PathBuf, std::path::PathBuf)>> = None;
        let mut logs = Vec::new();
        if let Some(rx) = self.tls_acme_rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(crate::tls::AcmeEvent::Log(m)) => logs.push(m),
                    Ok(crate::tls::AcmeEvent::Done(r)) => { done = Some(r); break; }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        done = Some(Err(anyhow::anyhow!("thread ACME encerrou sem resultado")));
                        break;
                    }
                }
            }
        }
        for m in logs { self.log_debug(&m); }
        if let Some(r) = done {
            self.tls_acme_rx = None;
            self.tls_acme_busy = false;
            match r {
                Ok((crt, _)) => {
                    self.tls_mode = TlsMode::LetsEncrypt;
                    self.tls_save_cfg();
                    self.log_debug(&format!("[acme] OK — certificado em {}. Modo trocado para Let's Encrypt.", crt.display()));
                    self.status_msg = self.tr("Certificado Let's Encrypt emitido.", "Let's Encrypt certificate issued.");
                    self.tls_refresh_cert_info();
                    if self.tls_proxy.is_some() || self.tls_enabled {
                        self.start_tls();
                    }
                }
                Err(e) => {
                    self.tls_last_error = format!("{e:#}");
                    self.log_debug(&format!("[acme] FALHOU: {e:#}"));
                    self.status_msg = self.tr("Emissao Let's Encrypt falhou — veja o console.", "Let's Encrypt issuance failed — see the console.");
                }
            }
        }

        // Renovação automática
        let due = match self.tls_renew_check {
            None => true,
            Some(t) => t.elapsed() > std::time::Duration::from_secs(6 * 3600),
        };
        if due {
            self.tls_renew_check = Some(std::time::Instant::now());
            if self.tls_proxy.is_some() {
                self.tls_refresh_cert_info();
                let needs = self.tls_cert_info.as_ref().map(|i| i.needs_renewal()).unwrap_or(false);
                if needs {
                    match self.tls_mode {
                        TlsMode::SelfSigned => {
                            self.log_debug("[https] Auto-assinado perto de expirar — regenerando.");
                            self.tls_regenerate_self_signed();
                        }
                        TlsMode::LetsEncrypt => {
                            self.log_debug("[acme] Certificado Let's Encrypt com menos de 30 dias — renovando.");
                            self.tls_issue_letsencrypt();
                        }
                        TlsMode::Custom => {
                            self.log_debug("[https] AVISO: o certificado proprio esta perto de expirar — substitua os arquivos.");
                        }
                    }
                }
            }
        }
    }

    /// URL HTTPS que FUNCIONA AGORA (mesma doutrina da URL HTTP da tela).
    pub fn tls_url(&self) -> String {
        let port = self.tls_port.trim();
        let host = match self.tls_status {
            TlsStatus::Listening | TlsStatus::ListeningNoMcp => {
                if !self.tls_domain.trim().is_empty() && self.tls_mode == TlsMode::LetsEncrypt {
                    self.tls_domain.trim().to_string()
                } else if self.tls_probe_lan_ok {
                    self.lan_ip.trim().to_string()
                } else {
                    "127.0.0.1".to_string()
                }
            }
            _ => "127.0.0.1".to_string(),
        };
        format!("https://{}:{}/mcp", host, port)
    }
}
