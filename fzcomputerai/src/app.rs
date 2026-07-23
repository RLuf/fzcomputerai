use eframe::egui;
use std::process::Command;

#[derive(PartialEq)]
pub enum Language {
    PtBr,
    English,
}

#[derive(PartialEq)]
pub enum Tab {
    Network,
    Driver,
    Skills,
}

pub struct AppState {
    pub language: Language,
    pub active_tab: Tab,
    pub http_port: String,
    pub lan_ip: String,
    pub port_active: bool,
    pub daemon_running: bool,
    pub doctor_output: String,
    pub skills_output: String,
    pub is_downloading: bool,
    pub download_progress: f32,
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
            doctor_output: "Pronto para executar diagnósticos.".to_string(),
            skills_output: "Nenhuma ação de skill executada.".to_string(),
            is_downloading: false,
            download_progress: 1.0,
            show_about: false,
        };
        state.check_port_status();
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

    pub fn kick_autostart(&mut self) {
        let _ = Command::new("cua-driver")
            .arg("autostart")
            .arg("kick")
            .output();
        self.daemon_running = true;
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

impl eframe::App for FzComputerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Painel Superior (Header)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("💻 FzComputerAI");
                ui.label("v1.0.0 — Computer Vision via MCP");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("❓ Sobre / Donate").clicked() {
                        self.state.show_about = true;
                    }

                    if ui.button(match self.state.language {
                        Language::PtBr => "🇺🇸 English",
                        Language::English => "🇧🇷 PT-BR",
                    }).clicked() {
                        self.state.language = match self.state.language {
                            Language::PtBr => Language::English,
                            Language::English => Language::PtBr,
                        };
                    }
                });
            });
        });

        // Painel de Rodapé (Footer Branding)
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Patrocinadores:");
                if ui.link("Webstorage Tecnologia").clicked() {
                    let _ = open::that("https://www.webstorage.com.br");
                }
                ui.label("|");
                if ui.link("Imóvel Site").clicked() {
                    let _ = open::that("https://www.imovelsite.com.br");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Autor: Roger Luft");
                });
            });
        });

        // Painel Central (Navegação de Abas & Conteúdo)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state.active_tab, Tab::Network, match self.state.language {
                    Language::PtBr => "🌐 Rede & TCP Proxy",
                    Language::English => "🌐 Network & TCP Proxy",
                });
                ui.selectable_value(&mut self.state.active_tab, Tab::Driver, match self.state.language {
                    Language::PtBr => "🖥️ Motor Driver",
                    Language::English => "🖥️ Engine Driver",
                });
                ui.selectable_value(&mut self.state.active_tab, Tab::Skills, match self.state.language {
                    Language::PtBr => "🧩 Skills & Agentes",
                    Language::English => "🧩 Skills & Agents",
                });
            });

            ui.separator();

            match self.state.active_tab {
                Tab::Network => crate::tabs::network::render(ui, &mut self.state),
                Tab::Driver => crate::tabs::driver::render(ui, &mut self.state),
                Tab::Skills => crate::tabs::skills::render(ui, &mut self.state),
            }
        });

        // Modal / Janela de Sobre & Doação
        if self.state.show_about {
            egui::Window::new(match self.state.language {
                Language::PtBr => "Sobre o FzComputerAI & Doação",
                Language::English => "About FzComputerAI & Donation",
            })
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("FzComputerAI — Servidor de Visão Computacional por MCP");
                ui.label("Desenvolvido por: Roger Luft <roger@webstorage.com.br>");
                ui.add_space(8.0);
                ui.label("Patrocinadores:");
                ui.hyperlink("https://www.webstorage.com.br");
                ui.hyperlink("https://www.imovelsite.com.br");
                ui.add_space(8.0);
                ui.label("Apoio / Pix / Suporte:");
                ui.label("📱 WhatsApp & Pix: +55 51 99242539");
                ui.add_space(10.0);
                if ui.button("Fechar / Close").clicked() {
                    self.state.show_about = false;
                }
            });
        }
    }
}
