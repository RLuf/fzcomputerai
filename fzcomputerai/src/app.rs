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
fn detect_lan_ip() -> String {
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
    pub calibration_log: String,

    // Janelas & Processos
    pub windows_list: Vec<WindowItem>,
    pub launch_input: String,
    pub windows_log: String,

    // Gravação & Trajetórias
    pub is_recording: bool,
    pub recording_path: String,
    pub recording_log: String,

    // Doctor & Skills
    pub doctor_output: String,
    pub skills_output: String,

    pub show_about: bool,

    // Console Debug (todos os comandos executados + stdout/stderr/erros)
    pub debug_log: String,

    // Iniciar com o Windows (HKCU\...\Run)
    pub autostart_enabled: bool,

    // MCP Tools Catalog
    pub mcp_tools_output: String,
    pub mcp_tools_filter: String,

    // Fluxo de upgrade (GitHub Releases):
    //   check -> update_available(tag) -> download em BACKGROUND (%TEMP%)
    //   -> ready.flag -> pedir para FECHAR -> instalar -> reabrir GUI + motor.
    pub update_available: Option<String>,
    pub update_downloading: bool,
    pub update_ready: bool,
    last_update_poll: Option<std::time::Instant>,
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
            calibration_log: String::new(),

            windows_list: Vec::new(),
            launch_input: "notepad".to_string(),
            windows_log: String::new(),

            is_recording: false,
            recording_path: "./recordings".to_string(),
            recording_log: String::new(),

            doctor_output: String::new(),
            skills_output: String::new(),

            show_about: false,

            debug_log: String::new(),
            autostart_enabled: false,

            mcp_tools_output: String::new(),
            mcp_tools_filter: String::new(),

            update_available: None,
            update_downloading: false,
            update_ready: false,
            last_update_poll: None,
        };
        state.log_debug(&format!("[startup] IP LAN autodetectado: {}", state.lan_ip));
        state.startup_reconcile_tracked_rules();
        state.check_port_status();
        state.daemon_running = state.port_active;
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
                let request = format!(
                    "POST /mcp HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    ip,
                    port,
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
            let exists = if lan_is_loopback {
                false
            } else {
                self.portproxy_rule_exists(&lan_ip, &port.to_string())
            };
            self.portproxy_active = exists;
            self.portproxy_effective = exists && netstat_lan;
            if exists && !netstat_lan {
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

            let bind_ok = self.set_user_env_confirmed("CUA_DRIVER_RS_MCP_HTTP_BIND", "0.0.0.0");
            if bind_ok {
                self.log_debug(
                    "[env] OK: CUA_DRIVER_RS_MCP_HTTP_BIND = 0.0.0.0 confirmado (todas as interfaces).",
                );
            } else {
                self.log_debug(
                    "[env] FALHA: CUA_DRIVER_RS_MCP_HTTP_BIND NAO confirmado — o daemon pode continuar so em 127.0.0.1.",
                );
            }

            if port_ok || bind_ok {
                self.log_debug("[env] Reiniciando daemon cua-driver para aplicar a configuracao...");
                self.run_logged("cua-driver", &["stop"]);
                self.run_logged("cua-driver", &["autostart", "kick"]);
            }
        }
        self.check_port_status();
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
        #[cfg(target_os = "windows")]
        {
            let my_pid = std::process::id();
            let ps = format!(
                "$ErrorActionPreference='SilentlyContinue'; \
                 $deadline=(Get-Date).AddSeconds(20); \
                 while ((Get-Process -Id {pid} -ErrorAction SilentlyContinue) -and ((Get-Date) -lt $deadline)) {{ Start-Sleep -Milliseconds 300 }}; \
                 Stop-Process -Id {pid} -Force -ErrorAction SilentlyContinue; \
                 cua-driver stop | Out-Null; \
                 taskkill /F /IM cua-driver.exe | Out-Null; \
                 $key='HKCU:\\Software\\FzComputerAI'; \
                 $props = (Get-ItemProperty -Path $key -ErrorAction SilentlyContinue); \
                 if (-not $props) {{ exit 0 }}; \
                 $rules = @($props.PSObject.Properties | Where-Object {{ $_.Name -like 'portproxy:*' }} | ForEach-Object {{ $_.Name.Substring(10) }}); \
                 if ($rules.Count -eq 0) {{ exit 0 }}; \
                 $pend=@(); \
                 foreach ($r in $rules) {{ \
                   $i=$r.LastIndexOf(':'); $addr=$r.Substring(0,$i); $prt=$r.Substring($i+1); \
                   netsh interface portproxy delete v4tov4 listenport=$prt listenaddress=$addr | Out-Null; \
                   $left = (netsh interface portproxy show v4tov4 | Select-String (\"^\\s*\" + [regex]::Escape($addr) + \"\\s+\" + $prt + \"\\s\")); \
                   if ($left) {{ $pend += \"netsh interface portproxy delete v4tov4 listenport=$prt listenaddress=$addr\" }} \
                   else {{ Remove-ItemProperty -Path $key -Name (\"portproxy:\" + $r) -Force -ErrorAction SilentlyContinue }} \
                 }}; \
                 if ($pend.Count -gt 0) {{ \
                   Start-Process -FilePath powershell -ArgumentList ('-NoProfile -WindowStyle Hidden -Command \"' + ($pend -join '; ') + '\"') -Verb RunAs -Wait; \
                   foreach ($r in $rules) {{ \
                     $i=$r.LastIndexOf(':'); $addr=$r.Substring(0,$i); $prt=$r.Substring($i+1); \
                     $left = (netsh interface portproxy show v4tov4 | Select-String (\"^\\s*\" + [regex]::Escape($addr) + \"\\s+\" + $prt + \"\\s\")); \
                     if (-not $left) {{ Remove-ItemProperty -Path $key -Name (\"portproxy:\" + $r) -Force -ErrorAction SilentlyContinue }} \
                   }} \
                 }}",
                pid = my_pid
            );

            // spawn(), NUNCA output()/wait(): o auxiliar vive por conta
            // propria e o processo da GUI pode terminar imediatamente.
            let _ = quiet_cmd("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps.as_str()])
                .spawn();
        }
    }

    /// Remove a regra portproxy (netsh delete) com o MESMO fluxo honesto do
    /// apply_portproxy: tentativa direta, fallback elevado via UAC oficial e
    /// confirmação relendo `show v4tov4` — o estado exibido nunca é presumido.
    pub fn remove_portproxy(&mut self) {
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
                self.calibration_log = format!(
                    "get_screen_size:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.calibration_log = format!(
                    "get_screen_size falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.calibration_log =
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
                self.calibration_log = format!(
                    "move_cursor ({}, {}):\n{}",
                    x,
                    y,
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.calibration_log = format!(
                    "move_cursor ({}, {}) falhou (exit {:?}):\n{}",
                    x,
                    y,
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.calibration_log =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn refresh_windows(&mut self) {
        match self.run_logged("cua-driver", &["call", "list_windows"]) {
            Some(out) if out.status.success() => {
                self.windows_log = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
            Some(out) => {
                self.windows_log = format!(
                    "list_windows falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.windows_log =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn launch_app(&mut self) {
        let app = self.launch_input.trim().to_string();
        match self.run_logged("cua-driver", &["call", "launch_app", "--app", app.as_str()]) {
            Some(out) if out.status.success() => {
                self.windows_log = format!(
                    "launch_app '{}':\n{}",
                    app,
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.windows_log = format!(
                    "launch_app '{}' falhou (exit {:?}):\n{}",
                    app,
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.windows_log =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn start_recording(&mut self) {
        match self.run_logged("cua-driver", &["call", "start_recording"]) {
            Some(out) if out.status.success() => {
                self.is_recording = true;
                self.recording_log = format!(
                    "start_recording:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.recording_log = format!(
                    "start_recording falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.recording_log =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn stop_recording(&mut self) {
        match self.run_logged("cua-driver", &["call", "stop_recording"]) {
            Some(out) if out.status.success() => {
                self.is_recording = false;
                self.recording_log = format!(
                    "stop_recording:\n{}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            Some(out) => {
                self.recording_log = format!(
                    "stop_recording falhou (exit {:?}):\n{}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            None => {
                self.recording_log =
                    "ERRO: nao foi possivel executar 'cua-driver' (esta no PATH?).".to_string();
            }
        }
    }

    pub fn start_daemon(&mut self) {
        let _ = self.run_logged("cua-driver", &["autostart", "kick"]);
        self.check_port_status();
        self.daemon_running = self.port_active;
    }

    pub fn stop_daemon(&mut self) {
        let _ = self.run_logged("cua-driver", &["stop"]);
        self.check_port_status();
        self.daemon_running = self.port_active;
    }

    pub fn run_doctor(&mut self) {
        match self.run_logged("cua-driver", &["doctor"]) {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.doctor_output = if !stdout.is_empty() {
                    stdout
                } else if !stderr.is_empty() {
                    stderr
                } else {
                    format!("doctor: exit {:?} (sem saida)", out.status.code())
                };
            }
            None => {
                self.doctor_output =
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
                self.skills_output = if !stdout.is_empty() {
                    stdout
                } else if !stderr.is_empty() {
                    stderr
                } else {
                    format!("skills {}: exit {:?} (sem saida)", action, out.status.code())
                };
            }
            None => {
                self.skills_output =
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
                self.mcp_tools_output = if !stdout.is_empty() {
                    format!("[{}] OK:\n{}", tool_name, stdout)
                } else if !stderr.is_empty() {
                    format!("[{}] stderr:\n{}", tool_name, stderr)
                } else {
                    format!("[{}]: exit {:?} (sem saida)", tool_name, out.status.code())
                };
            }
            None => {
                self.mcp_tools_output = format!(
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

    /// PASSO 1 do upgrade: consulta rapida ao GitHub Releases. Se houver
    /// versao mais nova, apenas MARCA update_available — o download so
    /// comeca depois que o usuario confirmar no dialogo (nada e baixado nem
    /// instalado sem consentimento).
    pub fn check_for_updates(&mut self) {
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

    /// PASSO 2: download do instalador em PROCESSO SEPARADO (a UI nao trava).
    /// O processo grava ready.flag ao terminar; poll_update_download observa.
    pub fn start_update_download(&mut self) {
        let Some(tag) = self.update_available.clone() else {
            return;
        };
        #[cfg(target_os = "windows")]
        {
            let dir = Self::update_dir();
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
                dir = dir.display(),
                tag = tag
            );
            match quiet_cmd("powershell")
                .args(["-NoProfile", "-Command", ps.as_str()])
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
            self.update_downloading = false;
            self.update_available = None;
            self.log_debug(&format!(
                "[upgrade] FALHA no download/verificacao do instalador: {}",
                msg.trim()
            ));
            return;
        }

        if flag.exists() && setup.exists() {
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
                 cua-driver autostart kick",
                dir = dir.display(),
                cur = current_exe
            );
            match quiet_cmd("powershell")
                .args(["-NoProfile", "-Command", ps.as_str()])
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
}

#[derive(Default)]
pub struct FzComputerApp {
    pub state: AppState,
}

fn setup_fazai_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = Color32::from_rgb(28, 28, 28);
    visuals.window_fill = Color32::from_rgb(28, 28, 28);
    visuals.faint_bg_color = Color32::from_rgb(38, 38, 38);
    visuals.extreme_bg_color = Color32::from_rgb(20, 20, 20);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(38, 38, 38);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(230, 230, 230));

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 45);
    visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(200, 200, 200));

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 73, 94);
    visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.active.bg_fill = Color32::from_rgb(33, 150, 243);
    visuals.widgets.active.rounding = egui::Rounding::same(6.0);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);

    visuals.selection.bg_fill = Color32::from_rgb(33, 150, 243);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);
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
        if self.state.update_downloading {
            // Garante novos frames mesmo sem input, para o poll acontecer.
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }

        // Header Principal
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().inner_margin(12.0).fill(Color32::from_rgb(24, 24, 24)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.heading(
                        egui::RichText::new("FZComputerAI")
                            .size(24.0)
                            .strong()
                            .color(Color32::WHITE)
                    );
                    ui.label(
                        egui::RichText::new(concat!(
                            "v",
                            env!("CARGO_PKG_VERSION"),
                            " - Computer Vision, MCP & CLI Hub"
                        ))
                        .size(13.0)
                        .color(Color32::from_rgb(170, 170, 170))
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let about_btn = egui::Button::new(
                            egui::RichText::new(match self.state.language {
                                Language::PtBr => "Ajuda & Sobre",
                                Language::English => "Help & About",
                            })
                            .color(Color32::WHITE)
                            .size(13.0)
                        )
                        .fill(Color32::from_rgb(52, 73, 94))
                        .min_size(Vec2::new(110.0, 30.0))
                        .rounding(egui::Rounding::same(6.0));

                        if ui.add(about_btn).clicked() {
                            self.state.show_about = true;
                        }

                        // Sem emojis de bandeira: a fonte default do egui nao
                        // renderiza flags (aparecem como caixas).
                        let lang_btn = egui::Button::new(match self.state.language {
                            Language::PtBr => "EN | English",
                            Language::English => "PT | Português (BR)",
                        })
                        .fill(Color32::from_rgb(45, 45, 45))
                        .min_size(Vec2::new(90.0, 30.0))
                        .rounding(egui::Rounding::same(6.0));

                        if ui.add(lang_btn).clicked() {
                            self.state.language = match self.state.language {
                                Language::PtBr => Language::English,
                                Language::English => Language::PtBr,
                            };
                        }

                        ui.add_space(10.0);
                        // Mesmo critério do status da aba MCP & Rede:
                        // verde só com LAN confirmada pelo netstat + TCP.
                        // Ponto DESENHADO (status_dot) — a fonte padrao nao
                        // tem o glifo "●" e renderizava uma caixa quebrada.
                        let (status_txt, status_color) = match self.state.port_status {
                            crate::app::PortStatus::LanListening => (
                                match self.state.language {
                                    Language::PtBr => format!("MCP HTTP Ativo (local + LAN) (:{})", self.state.http_port),
                                    Language::English => format!("MCP HTTP Active (local + LAN) (:{})", self.state.http_port),
                                },
                                Color32::from_rgb(76, 175, 80),
                            ),
                            crate::app::PortStatus::LocalOnly => (
                                match self.state.language {
                                    Language::PtBr => format!("MCP HTTP Local apenas (:{})", self.state.http_port),
                                    Language::English => format!("MCP HTTP Local only (:{})", self.state.http_port),
                                },
                                Color32::from_rgb(255, 193, 7),
                            ),
                            crate::app::PortStatus::Stopped => (
                                match self.state.language {
                                    Language::PtBr => "MCP HTTP Parado".to_string(),
                                    Language::English => "MCP HTTP Stopped".to_string(),
                                },
                                Color32::from_rgb(239, 83, 80),
                            ),
                        };
                        ui.label(
                            egui::RichText::new(status_txt)
                                .color(status_color)
                                .strong()
                                .size(13.0)
                        );
                        crate::app::status_dot(ui, status_color);
                    });
                });
            });

        // Rodapé
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::none().inner_margin(10.0).fill(Color32::from_rgb(20, 20, 20)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Adaptativo: em janela estreita o texto da esquerda
                    // colidia com o bloco Donate/empresa da direita — some
                    // quando nao ha largura para os dois.
                    if ui.available_width() > 920.0 {
                        ui.label(
                            egui::RichText::new(match self.state.language {
                                Language::PtBr => "O FzComputerAI integra ferramentas CLI e MCP para Visão e Automação de UI.",
                                Language::English => "FzComputerAI integrates CLI and MCP tooling for Vision and UI Automation.",
                            })
                            .size(12.0)
                            .color(Color32::from_rgb(150, 150, 150))
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Grupo FazAI | Webstorage Tecnologia")
                                .size(12.0)
                                .color(Color32::from_rgb(140, 140, 140))
                        );
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(match self.state.language {
                                Language::PtBr => "Doações / Donate: +55 51 99242539",
                                Language::English => "Donate: +55 51 99242539",
                            })
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(76, 175, 80))
                        );
                    });
                });
            });

        // Painel Central com Abas Estilo Pill
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(16.0).fill(Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Barra de abas ADAPTATIVA: nada de largura fixa de 940px.
                    // Em janela larga a fileira fica centralizada (add_space
                    // calculado sobre a largura real); em janela estreita o
                    // horizontal_wrapped quebra em duas fileiras em vez de
                    // estourar/cortar botões.
                    ui.horizontal_wrapped(|ui| {
                        const TAB_W: f32 = 150.0;
                        const TAB_SPACING: f32 = 4.0;

                        let tabs = [
                            (Tab::Network, match self.state.language {
                                Language::PtBr => "MCP & Rede",
                                Language::English => "MCP & Network",
                            }),
                            (Tab::Calibration, match self.state.language {
                                Language::PtBr => "Calibração & Visão",
                                Language::English => "Calibration & Vision",
                            }),
                            (Tab::Windows, match self.state.language {
                                Language::PtBr => "Janelas & Processos",
                                Language::English => "Windows & Apps",
                            }),
                            (Tab::Recording, match self.state.language {
                                Language::PtBr => "Gravação Trajetória",
                                Language::English => "Recording Trajectory",
                            }),
                            (Tab::DoctorSkills, match self.state.language {
                                Language::PtBr => "Doctor & Skills",
                                Language::English => "Doctor & Skills",
                            }),
                            (Tab::McpTools, match self.state.language {
                                Language::PtBr => "MCP Tools",
                                Language::English => "MCP Tools",
                            }),
                        ];

                        // Centraliza SOMENTE quando a fileira inteira cabe;
                        // quando não cabe, o espaço vira 0 e o wrapped quebra.
                        let n = tabs.len() as f32;
                        let row_w = n * TAB_W + (n - 1.0) * TAB_SPACING;
                        ui.add_space(((ui.available_width() - row_w) / 2.0).max(0.0));

                        for (tab, label) in tabs {
                            let is_selected = self.state.active_tab == tab;
                            let bg_color = if is_selected {
                                Color32::from_rgb(33, 150, 243)
                            } else {
                                Color32::from_rgb(45, 45, 45)
                            };

                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .color(Color32::WHITE)
                                    .size(13.0)
                                    .strong()
                            )
                            .fill(bg_color)
                            .min_size(Vec2::new(TAB_W, 32.0))
                            .rounding(egui::Rounding::same(8.0));

                            if ui.add(btn).clicked() {
                                self.state.active_tab = tab;
                            }
                            ui.add_space(TAB_SPACING);
                        }
                    });
                });

                ui.add_space(16.0);

                match self.state.active_tab {
                    Tab::Network => crate::tabs::network::render(ui, &mut self.state),
                    Tab::Calibration => crate::tabs::calibration::render(ui, &mut self.state),
                    Tab::Windows => crate::tabs::windows::render(ui, &mut self.state),
                    Tab::Recording => crate::tabs::recording::render(ui, &mut self.state),
                    Tab::DoctorSkills => crate::tabs::doctor_skills::render(ui, &mut self.state),
                    Tab::McpTools => crate::tabs::mcp_tools::render(ui, &mut self.state),
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
                    Language::PtBr => "Desenvolvido por: Roger Luft <roger@webstorage.com.br>",
                    Language::English => "Developed by: Roger Luft <roger@webstorage.com.br>",
                });
                ui.add_space(8.0);
                ui.label(match lang {
                    Language::PtBr => "Patrocinadores Oficiais:",
                    Language::English => "Official Sponsors:",
                });
                ui.hyperlink_to("Webstorage Tecnologia", "https://www.webstorage.com.br");
                ui.hyperlink_to("Imóvel Site", "https://www.imovelsite.com.br");
                ui.add_space(8.0);
                ui.label(match lang {
                    Language::PtBr => "Suporte & WhatsApp:",
                    Language::English => "Support & WhatsApp:",
                });
                ui.label("+55 51 99242539");
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

        // ─── Dialogo 1 do upgrade: nova versao encontrada — baixar? ───
        if self.state.update_available.is_some()
            && !self.state.update_downloading
            && !self.state.update_ready
        {
            let lang = self.state.language;
            let tag = self.state.update_available.clone().unwrap_or_default();
            let mut do_download = false;
            let mut do_dismiss = false;

            egui::Window::new(match lang {
                Language::PtBr => "Atualização disponível",
                Language::English => "Update available",
            })
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.label(match lang {
                    Language::PtBr => format!(
                        "Nova versão {} disponível (atual: v{}).\nBaixar o instalador em segundo plano?\nVocê poderá continuar usando o aplicativo durante o download.",
                        tag,
                        env!("CARGO_PKG_VERSION")
                    ),
                    Language::English => format!(
                        "New version {} available (current: v{}).\nDownload the installer in the background?\nYou can keep using the app while it downloads.",
                        tag,
                        env!("CARGO_PKG_VERSION")
                    ),
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(match lang {
                            Language::PtBr => "Baixar agora",
                            Language::English => "Download now",
                        })
                        .clicked()
                    {
                        do_download = true;
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

            if do_download {
                self.state.start_update_download();
            }
            if do_dismiss {
                self.state.update_available = None;
            }
        }

        // ─── Dialogo 2 do upgrade: download pronto — fechar e instalar? ───
        if self.state.update_ready {
            let lang = self.state.language;
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
                ui.horizontal(|ui| {
                    if ui
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
                self.state.update_ready = false;
                self.state.update_available = None;
            }
        }
    }
}
