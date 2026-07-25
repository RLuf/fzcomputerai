use eframe::egui::{self, Color32, Vec2};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(PartialEq, Clone, Copy)]
pub enum Language {
    PtBr,
    English,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Network,
    Calibration,
    Windows,
    Recording,
    DoctorSkills,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WindowItem {
    pub pid: u32,
    pub window_id: u64,
    pub title: String,
    pub app_name: Option<String>,
    pub minimized: Option<bool>,
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
    pub daemon_running: bool,

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
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            language: Language::PtBr,
            active_tab: Tab::Network,
            http_port: "8000".to_string(),
            lan_ip: detect_lan_ip(),
            port_active: false,
            daemon_running: false,

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
        };
        state.log_debug(&format!("[startup] IP LAN autodetectado: {}", state.lan_ip));
        state.check_port_status();
        state.daemon_running = state.port_active;
        #[cfg(target_os = "windows")]
        state.read_autostart();
        state.fetch_screen_info();
        state
    }
}

impl AppState {
    /// Anexa uma entrada ao Console Debug (mantém tamanho limitado).
    pub fn log_debug(&mut self, entry: &str) {
        if !self.debug_log.is_empty() {
            self.debug_log.push('\n');
        }
        self.debug_log.push_str(entry);

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

    /// Teste REAL do endpoint MCP: TcpStream::connect_timeout em
    /// 127.0.0.1:{porta} (~800ms) e, se conectar, GET /mcp mínimo lendo a
    /// primeira linha da resposta. Atualiza `port_active` com a verdade.
    pub fn check_port_status(&mut self) {
        use std::io::{Read, Write};

        let port: u16 = match self.http_port.trim().parse() {
            Ok(p) => p,
            Err(_) => {
                self.port_active = false;
                let port_txt = self.http_port.clone();
                self.log_debug(&format!(
                    "> tcp-check 127.0.0.1:{}\n  ERRO: porta invalida",
                    port_txt
                ));
                return;
            }
        };

        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let timeout = std::time::Duration::from_millis(800);

        match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(mut stream) => {
                self.port_active = true;
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));

                let request = format!(
                    "GET /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
                    port
                );
                let first_line = match stream.write_all(request.as_bytes()) {
                    Ok(()) => {
                        let mut buf = [0u8; 1024];
                        match stream.read(&mut buf) {
                            Ok(n) if n > 0 => String::from_utf8_lossy(&buf[..n])
                                .lines()
                                .next()
                                .unwrap_or("")
                                .to_string(),
                            Ok(_) => "(conexao fechada sem resposta HTTP)".to_string(),
                            Err(e) => format!("(erro de leitura: {})", e),
                        }
                    }
                    Err(e) => format!("(erro de escrita: {})", e),
                };
                self.log_debug(&format!(
                    "> tcp-check 127.0.0.1:{}\n  TCP conectado. GET /mcp -> {}",
                    port, first_line
                ));
            }
            Err(e) => {
                self.port_active = false;
                self.log_debug(&format!(
                    "> tcp-check 127.0.0.1:{}\n  SEM conexao ({})",
                    port, e
                ));
            }
        }
    }

    /// Define CUA_DRIVER_RS_MCP_HTTP_PORT (User) pela via oficial e RELÊ a
    /// variável no registro para confirmar sucesso/falha real.
    pub fn apply_env_port(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let port = self.http_port.trim().to_string();
            let ps = format!(
                "[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '{}', 'User')",
                port
            );
            let set_ok = self
                .run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()])
                .map(|o| o.status.success())
                .unwrap_or(false);

            let confirmed = self
                .run_logged(
                    "reg",
                    &[
                        "query",
                        r"HKCU\Environment",
                        "/v",
                        "CUA_DRIVER_RS_MCP_HTTP_PORT",
                    ],
                )
                .map(|o| {
                    o.status.success()
                        && String::from_utf8_lossy(&o.stdout).contains(port.as_str())
                })
                .unwrap_or(false);

            if set_ok && confirmed {
                self.log_debug(&format!(
                    "[env] OK: CUA_DRIVER_RS_MCP_HTTP_PORT = {} confirmado em HKCU\\Environment.",
                    port
                ));
            } else {
                self.log_debug(
                    "[env] FALHA: valor NAO confirmado em HKCU\\Environment apos SetEnvironmentVariable.",
                );
            }
        }
        self.check_port_status();
    }

    /// Regra portproxy do netsh (exige admin). Tenta sem elevação; se o
    /// erro indicar falta de privilégio, dispara UAC oficial via
    /// `Start-Process netsh -Verb RunAs` e valida com `show v4tov4`.
    pub fn apply_portproxy(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let port = self.http_port.trim().to_string();
            let ip = self.lan_ip.trim().to_string();
            let netsh_args = format!(
                "interface portproxy add v4tov4 listenport={} listenaddress={} connectport={} connectaddress=127.0.0.1",
                port, ip, port
            );
            let args_vec: Vec<&str> = netsh_args.split(' ').collect();

            let direct = self.run_logged("netsh", &args_vec);
            let direct_ok = direct
                .as_ref()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !direct_ok {
                let combined = direct
                    .as_ref()
                    .map(|o| {
                        format!(
                            "{}{}",
                            String::from_utf8_lossy(&o.stdout),
                            String::from_utf8_lossy(&o.stderr)
                        )
                        .to_lowercase()
                    })
                    .unwrap_or_default();

                let needs_elevation = direct.is_some()
                    && (combined.contains("elev")   // "elevation" / "elevação"
                        || combined.contains("admin")
                        || combined.contains("denied")
                        || combined.contains("negad")); // "negado/negada"

                if needs_elevation {
                    self.log_debug(
                        "[portproxy] Privilegio insuficiente detectado — solicitando UAC...",
                    );
                    let ps = format!(
                        "Start-Process -FilePath netsh -ArgumentList '{}' -Verb RunAs -Wait",
                        netsh_args
                    );
                    match self.run_logged("powershell", &["-NoProfile", "-Command", ps.as_str()]) {
                        Some(o) if o.status.success() => {
                            self.log_debug("[portproxy] UAC solicitado: comando elevado concluiu.")
                        }
                        Some(_) => self.log_debug(
                            "[portproxy] UAC solicitado: CANCELADO pelo usuario ou falhou.",
                        ),
                        None => {}
                    }
                } else if direct.is_some() {
                    self.log_debug("[portproxy] netsh falhou (motivo acima, nao parece ser privilegio).");
                }
            }

            // Validação final com a verdade do sistema
            if let Some(show) = self.run_logged("netsh", &["interface", "portproxy", "show", "v4tov4"]) {
                let text = String::from_utf8_lossy(&show.stdout).to_string();
                if text.contains(port.as_str()) && text.contains(ip.as_str()) {
                    self.log_debug(&format!(
                        "[portproxy] Regra APLICADA: {}:{} -> 127.0.0.1:{}",
                        ip, port, port
                    ));
                } else {
                    self.log_debug("[portproxy] Regra NAO encontrada em 'show v4tov4'.");
                }
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        setup_fazai_theme(ctx);

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
                        if self.state.port_active {
                            ui.label(
                                egui::RichText::new(match self.state.language {
                                    Language::PtBr => format!("● MCP HTTP Ativo (:{})", self.state.http_port),
                                    Language::English => format!("● MCP HTTP Active (:{})", self.state.http_port),
                                })
                                .color(Color32::from_rgb(76, 175, 80))
                                .strong()
                                .size(13.0)
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(match self.state.language {
                                    Language::PtBr => "● MCP HTTP Parado",
                                    Language::English => "● MCP HTTP Stopped",
                                })
                                .color(Color32::from_rgb(239, 83, 80))
                                .strong()
                                .size(13.0)
                            );
                        }
                    });
                });
            });

        // Rodapé
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::none().inner_margin(10.0).fill(Color32::from_rgb(20, 20, 20)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(match self.state.language {
                            Language::PtBr => "O FzComputerAI integra ferramentas CLI e MCP para Visão e Automação de UI.",
                            Language::English => "FzComputerAI integrates CLI and MCP tooling for Vision and UI Automation.",
                        })
                        .size(12.0)
                        .color(Color32::from_rgb(150, 150, 150))
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Grupo FazAI | Webstorage Tecnologia | Imóvel Site")
                                .size(12.0)
                                .color(Color32::from_rgb(140, 140, 140))
                        );
                    });
                });
            });

        // Painel Central com Abas Estilo Pill
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(16.0).fill(Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 780.0).max(0.0) / 2.0);

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
                        ];

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
                            .min_size(Vec2::new(150.0, 32.0))
                            .rounding(egui::Rounding::same(8.0));

                            if ui.add(btn).clicked() {
                                self.state.active_tab = tab;
                            }
                            ui.add_space(4.0);
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
    }
}
