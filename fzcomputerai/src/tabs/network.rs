use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Endpoint & Servidor MCP HTTP/WebSocket",
                        Language::English => "MCP HTTP/WebSocket Server Endpoint",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                egui::Grid::new("rules_grid")
                    .striped(true)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Porta TCP").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.label(RichText::new("Host / IP").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.label(RichText::new("Transporte").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.label(RichText::new("Status").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.end_row();

                        ui.label(&state.http_port);
                        ui.label(&state.lan_ip);
                        ui.label(RichText::new("HTTP / JSON-RPC").color(Color32::from_rgb(255, 167, 38)));
                        if state.port_active {
                            ui.label(RichText::new("LISTENING").color(Color32::from_rgb(76, 175, 80)).strong());
                        } else {
                            ui.label(RichText::new("STOPPED").color(Color32::from_rgb(239, 83, 80)).strong());
                        }
                        ui.end_row();
                    });

                ui.add_space(16.0);
                ui.label(match state.language {
                    Language::PtBr => "URL de Conexão MCP para Agentes Remotos:",
                    Language::English => "MCP Connection URL for Remote Agents:",
                });
                ui.code(format!("http://{}:{}/mcp", state.lan_ip, state.http_port));

                ui.add_space(16.0);

                let refresh_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🔄 Testar Endpoint MCP",
                        Language::English => "🔄 Test MCP Endpoint",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(84, 110, 122))
                .min_size(Vec2::new(160.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(refresh_btn).clicked() {
                    state.check_port_status();
                }
            });

        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Configuração de Porta & Windows PortProxy",
                        Language::English => "Port Setup & Windows PortProxy",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                ui.label(match state.language {
                    Language::PtBr => "Porta TCP HTTP (padrão 8000):",
                    Language::English => "HTTP TCP Port (default 8000):",
                });
                ui.add(egui::TextEdit::singleline(&mut state.http_port).min_size(Vec2::new(200.0, 24.0)));

                ui.add_space(8.0);

                ui.label(match state.language {
                    Language::PtBr => "Endereço IP da LAN:",
                    Language::English => "LAN IP Address:",
                });
                ui.add(egui::TextEdit::singleline(&mut state.lan_ip).min_size(Vec2::new(200.0, 24.0)));

                ui.add_space(16.0);

                let apply_env_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "⚡ Aplicar CUA_DRIVER_RS_MCP_HTTP_PORT",
                        Language::English => "⚡ Set CUA_DRIVER_RS_MCP_HTTP_PORT",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(33, 150, 243))
                .min_size(Vec2::new(250.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(apply_env_btn).clicked() {
                    state.apply_env_port();
                }

                ui.add_space(8.0);

                let apply_proxy_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🔗 Regra Windows PortProxy (netsh)",
                        Language::English => "🔗 Windows PortProxy Rule (netsh)",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(41, 182, 246))
                .min_size(Vec2::new(250.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(apply_proxy_btn).clicked() {
                    state.apply_portproxy();
                }
            });
    });
}
