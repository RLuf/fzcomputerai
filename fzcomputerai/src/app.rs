use eframe::egui::{self, Color32, Vec2};
use std::process::Command;
use serde::{Deserialize, Serialize};

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
}

impl Default for AppState {
    fn default() -> Self {
        let mut state = Self {
            language: Language::PtBr,
            active_tab: Tab::Network,
            http_port: "8000".to_string(),
            lan_ip: "192.168.0.101".to_string(),
            port_active: true,
            daemon_running: true,

            screen_width: 1920,
            screen_height: 1080,
            dpi_scale: 1.0,
            test_x: "960".to_string(),
            test_y: "540".to_string(),
            calibration_log: "Pronto para calibrar tela e coordenadas de visão.".to_string(),

            windows_list: Vec::new(),
            launch_input: "notepad".to_string(),
            windows_log: "Clique em 'Atualizar Lista' para carregar janelas ativas.".to_string(),

            is_recording: false,
            recording_path: "./recordings".to_string(),
            recording_log: "Gravação de trajetória inativa.".to_string(),

            doctor_output: "Pronto para executar diagnósticos.".to_string(),
            skills_output: "Nenhuma ação de skill executada.".to_string(),
            
            show_about: false,
        };
        state.check_port_status();
        state.fetch_screen_info();
        state
    }
}

impl AppState {
    pub fn check_port_status(&mut self) {
        let output = Command::new("netstat")
            .args(["-an"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            self.port_active = text.contains(&self.http_port);
        }
    }

    pub fn apply_env_port(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("powershell")
                .args(["-Command", &format!("[Environment]::SetEnvironmentVariable('CUA_DRIVER_RS_MCP_HTTP_PORT', '{}', 'User')", self.http_port)])
                .output();
        }
        self.check_port_status();
    }

    pub fn apply_portproxy(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let cmd = format!(
                "netsh interface portproxy add v4tov4 listenport={} listenaddress={} connectport={} connectaddress=127.0.0.1",
                self.http_port, self.lan_ip, self.http_port
            );
            let _ = Command::new("cmd")
                .args(["/C", &cmd])
                .output();
        }
        self.check_port_status();
    }

    pub fn fetch_screen_info(&mut self) {
        let output = Command::new("cua-driver")
            .args(["call", "get_screen_size"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            self.calibration_log = format!("Informações da Tela:\n{}", text);
        }
    }

    pub fn test_click_position(&mut self) {
        let x: i32 = self.test_x.parse().unwrap_or(0);
        let y: i32 = self.test_y.parse().unwrap_or(0);

        let output = Command::new("cua-driver")
            .args(["call", "move_cursor", "--x", &x.to_string(), "--y", &y.to_string()])
            .output();

        if let Ok(out) = output {
            self.calibration_log = format!("Cursor movido para ({}, {}):\n{}", x, y, String::from_utf8_lossy(&out.stdout));
        } else {
            self.calibration_log = format!("Falha ao mover cursor para ({}, {}).", x, y);
        }
    }

    pub fn refresh_windows(&mut self) {
        let output = Command::new("cua-driver")
            .args(["call", "list_windows"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            self.windows_log = text.to_string();
        } else {
            self.windows_log = "Falha ao listar janelas.".to_string();
        }
    }

    pub fn launch_app(&mut self) {
        let output = Command::new("cua-driver")
            .args(["call", "launch_app", "--app", &self.launch_input])
            .output();

        if let Ok(out) = output {
            self.windows_log = format!("Aplicação '{}' iniciada:\n{}", self.launch_input, String::from_utf8_lossy(&out.stdout));
        }
    }

    pub fn start_recording(&mut self) {
        let output = Command::new("cua-driver")
            .args(["call", "start_recording"])
            .output();

        if let Ok(out) = output {
            self.is_recording = true;
            self.recording_log = format!("Gravação Iniciada:\n{}", String::from_utf8_lossy(&out.stdout));
        }
    }

    pub fn stop_recording(&mut self) {
        let output = Command::new("cua-driver")
            .args(["call", "stop_recording"])
            .output();

        if let Ok(out) = output {
            self.is_recording = false;
            self.recording_log = format!("Gravação Finalizada:\n{}", String::from_utf8_lossy(&out.stdout));
        }
    }

    pub fn start_daemon(&mut self) {
        let _ = Command::new("cua-driver")
            .arg("autostart")
            .arg("kick")
            .output();
        self.daemon_running = true;
    }

    pub fn stop_daemon(&mut self) {
        let _ = Command::new("cua-driver")
            .arg("stop")
            .output();
        self.daemon_running = false;
    }

    pub fn run_doctor(&mut self) {
        let output = Command::new("cua-driver")
            .arg("doctor")
            .output();

        if let Ok(out) = output {
            self.doctor_output = String::from_utf8_lossy(&out.stdout).to_string();
        } else {
            self.doctor_output = "Erro ao executar cua-driver doctor.".to_string();
        }
    }

    pub fn install_skills(&mut self) {
        let output = Command::new("cua-driver")
            .args(["skills", "install"])
            .output();

        if let Ok(out) = output {
            self.skills_output = String::from_utf8_lossy(&out.stdout).to_string();
        }
    }

    pub fn update_skills(&mut self) {
        let output = Command::new("cua-driver")
            .args(["skills", "update"])
            .output();

        if let Ok(out) = output {
            self.skills_output = String::from_utf8_lossy(&out.stdout).to_string();
        }
    }

    pub fn uninstall_skills(&mut self) {
        let output = Command::new("cua-driver")
            .args(["skills", "uninstall"])
            .output();

        if let Ok(out) = output {
            self.skills_output = String::from_utf8_lossy(&out.stdout).to_string();
        }
    }
}

pub struct FzComputerApp {
    pub state: AppState,
}

impl Default for FzComputerApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
        }
    }
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
                        egui::RichText::new("v1.0.0 - Computer Vision, MCP & CLI Hub")
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

                        let lang_btn = egui::Button::new(match self.state.language {
                            Language::PtBr => "🇺🇸 English",
                            Language::English => "🇧🇷 PT-BR",
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
                                egui::RichText::new("● MCP HTTP Active (:8000)")
                                    .color(Color32::from_rgb(76, 175, 80))
                                    .strong()
                                    .size(13.0)
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("● MCP HTTP Stopped")
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
                        egui::RichText::new("ℹ  O FzComputerAI integra ferramentas CLI e MCP para Visão e Automação de UI.")
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
                                Language::PtBr => "🌐 MCP & Rede",
                                Language::English => "🌐 MCP & Network",
                            }),
                            (Tab::Calibration, match self.state.language {
                                Language::PtBr => "🎯 Calibração & Visão",
                                Language::English => "🎯 Calibration & Vision",
                            }),
                            (Tab::Windows, match self.state.language {
                                Language::PtBr => "🖥️ Janelas & Processos",
                                Language::English => "🖥️ Windows & Apps",
                            }),
                            (Tab::Recording, match self.state.language {
                                Language::PtBr => "🎥 Gravação Trajetória",
                                Language::English => "🎥 Recording Trajectory",
                            }),
                            (Tab::DoctorSkills, match self.state.language {
                                Language::PtBr => "🩺 Doctor & Skills",
                                Language::English => "🩺 Doctor & Skills",
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
            egui::Window::new(match self.state.language {
                Language::PtBr => "Sobre o FzComputerAI & Suporte",
                Language::English => "About FzComputerAI & Support",
            })
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("FZComputerAI — Grupo FazAI");
                ui.label("Servidor Nativo de Visão Computacional, MCP & Hub CLI");
                ui.label("Desenvolvido por: Roger Luft <roger@webstorage.com.br>");
                ui.add_space(8.0);
                ui.label("Patrocinadores Oficiais:");
                ui.hyperlink_to("Webstorage Tecnologia", "https://www.webstorage.com.br");
                ui.hyperlink_to("Imóvel Site", "https://www.imovelsite.com.br");
                ui.add_space(8.0);
                ui.label("Suporte & WhatsApp:");
                ui.label("📱 +55 51 99242539");
                ui.add_space(12.0);
                if ui.button("Fechar / Close").clicked() {
                    self.state.show_about = false;
                }
            });
        }
    }
}
