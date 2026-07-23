use crate::app::{AppState, Language};
use egui::{Color32, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading(match state.language {
        Language::PtBr => "🌐 Gerenciador de Rede & Port Proxy TCP",
        Language::English => "🌐 Network Manager & TCP Port Proxy",
    });

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.label(match state.language {
            Language::PtBr => "Configuração de Porta TCP para Orquestradores Remotos (ex: FazAI-NG):",
            Language::English => "TCP Port Configuration for Remote Orchestrators (e.g. FazAI-NG):",
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label(match state.language {
                Language::PtBr => "Porta TCP HTTP:",
                Language::English => "HTTP TCP Port:",
            });
            ui.text_edit_singleline(&mut state.http_port);
        });

        ui.horizontal(|ui| {
            ui.label(match state.language {
                Language::PtBr => "IP da LAN (Rede Local):",
                Language::English => "LAN IP Address:",
            });
            ui.text_edit_singleline(&mut state.lan_ip);
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button(match state.language {
                Language::PtBr => "⚡ Aplicar Variável CUA_DRIVER_RS_MCP_HTTP_PORT",
                Language::English => "⚡ Set Environment Variable CUA_DRIVER_RS_MCP_HTTP_PORT",
            }).clicked() {
                state.apply_env_port();
            }

            if ui.button(match state.language {
                Language::PtBr => "🔗 Configurar Windows PortProxy (netsh)",
                Language::English => "🔗 Configure Windows PortProxy (netsh)",
            }).clicked() {
                state.apply_portproxy();
            }
        });
    });

    ui.add_space(15.0);

    ui.group(|ui| {
        ui.label(match state.language {
            Language::PtBr => "Status do Endpoint MCP HTTP:",
            Language::English => "MCP HTTP Endpoint Status:",
        });

        ui.horizontal(|ui| {
            if state.port_active {
                ui.colored_label(Color32::GREEN, "● LISTENING / OUVINDO");
            } else {
                ui.colored_label(Color32::RED, "● STOPPED / PARADO");
            }
            ui.label(format!("http://{}:{}/mcp", state.lan_ip, state.http_port));
        });

        if ui.button(match state.language {
            Language::PtBr => "🔄 Verificar Status da Porta",
            Language::English => "🔄 Refresh Port Status",
        }).clicked() {
            state.check_port_status();
        }
    });
}
