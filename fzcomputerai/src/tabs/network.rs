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
                        ui.label(RichText::new(match state.language {
                            Language::PtBr => "Porta TCP",
                            Language::English => "TCP Port",
                        }).strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.label(RichText::new("Host / IP").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.label(RichText::new(match state.language {
                            Language::PtBr => "Transporte",
                            Language::English => "Transport",
                        }).strong().color(Color32::from_rgb(180, 180, 180)));
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

                let mcp_url = format!("http://{}:{}/mcp", state.lan_ip, state.http_port);
                ui.horizontal(|ui| {
                    ui.code(&mcp_url);
                    if ui
                        .button(match state.language {
                            Language::PtBr => "Copiar",
                            Language::English => "Copy",
                        })
                        .clicked()
                    {
                        ui.output_mut(|o| o.copied_text = mcp_url.clone());
                    }
                });

                ui.add_space(16.0);

                let refresh_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "Testar Endpoint MCP",
                        Language::English => "Test MCP Endpoint",
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
                    Language::PtBr => "Endereço IP da LAN (autodetectado, editável):",
                    Language::English => "LAN IP Address (autodetected, editable):",
                });
                ui.add(egui::TextEdit::singleline(&mut state.lan_ip).min_size(Vec2::new(200.0, 24.0)));

                ui.add_space(16.0);

                let apply_env_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "Aplicar CUA_DRIVER_RS_MCP_HTTP_PORT",
                        Language::English => "Set CUA_DRIVER_RS_MCP_HTTP_PORT",
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
                        Language::PtBr => "Regra Windows PortProxy (netsh)",
                        Language::English => "Windows PortProxy Rule (netsh)",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(41, 182, 246))
                .min_size(Vec2::new(250.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(apply_proxy_btn).clicked() {
                    state.apply_portproxy();
                }

                #[cfg(target_os = "windows")]
                {
                    ui.add_space(12.0);
                    let mut autostart = state.autostart_enabled;
                    let label = match state.language {
                        Language::PtBr => "Iniciar com o Windows",
                        Language::English => "Start with Windows",
                    };
                    if ui.checkbox(&mut autostart, label).changed() {
                        state.set_autostart(autostart);
                    }
                }
            });
    });

    // Console Debug — todos os comandos executados aparecem aqui
    ui.add_space(16.0);
    Frame::none()
        .fill(Color32::from_rgb(22, 22, 22))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Console Debug",
                        Language::English => "Debug Console",
                    })
                    .size(14.0)
                    .strong()
                    .color(Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(match state.language {
                            Language::PtBr => "Limpar",
                            Language::English => "Clear",
                        })
                        .clicked()
                    {
                        state.debug_log.clear();
                    }
                });
            });

            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("debug_console_scroll")
                .max_height(180.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if state.debug_log.is_empty() {
                        ui.monospace(match state.language {
                            Language::PtBr => "(nenhum comando executado ainda)",
                            Language::English => "(no command executed yet)",
                        });
                    } else {
                        ui.monospace(&state.debug_log);
                    }
                });
        });
}
