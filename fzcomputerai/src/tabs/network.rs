use crate::app::{status_dot, AppState, Language, PortStatus};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Layout com ANCORAS FIXAS: controles do daemon, configuracao de
    // porta/IP e encaminhamento ficam SEMPRE visiveis (nenhum controle
    // interativo rola para fora da tela); o DIAGNOSTICO (endpoint real,
    // listeners do netstat, regras portproxy) e a unica area rolavel; o
    // Console Debug e faixa fixa no rodape.
    render_daemon_controls(ui, state);
    ui.add_space(10.0);
    render_controls_row(ui, state);
    ui.add_space(10.0);

    // Aritmetica que NUNCA estoura a janela: o diagnostico tem um minimo
    // pequeno (rola) e o console recebe o resto, com teto — em janela baixa
    // os dois continuam visiveis, sem empurrar nada para fora da tela.
    let restante = ui.available_height();
    let diag_min = 60.0;
    let console_h = ((restante - diag_min - 10.0) * 0.6)
        .clamp(90.0, 320.0)
        .min((restante - diag_min - 10.0).max(60.0));
    let diag_h = (restante - console_h - 10.0).max(diag_min);

    egui::ScrollArea::vertical()
        .id_salt("network_diag_scroll")
        .max_height(diag_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_diagnostics(ui, state);
        });

    ui.add_space(6.0);
    render_console(ui, state);
}

// ─── Seção 1 (FIXA no topo): controles do daemon + status ───
fn render_daemon_controls(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(Color32::from_rgb(24, 32, 42))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Serviço CUA Driver (Motor de Visão Computacional)",
                        Language::English => "CUA Driver Service (Computer Vision Engine)",
                    })
                    .size(15.0)
                    .strong()
                    .color(Color32::WHITE),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Status indicator (ponto DESENHADO — a fonte padrao nao
                    // tem o glifo "●" e renderizava uma caixa quebrada)
                    let (status_txt, status_color) = match state.port_status {
                        PortStatus::LanListening => (
                            match state.language {
                                Language::PtBr => format!("ATIVO (local + LAN) :{}", state.http_port),
                                Language::English => format!("ACTIVE (local + LAN) :{}", state.http_port),
                            },
                            Color32::from_rgb(76, 175, 80),
                        ),
                        PortStatus::LocalOnly => (
                            match state.language {
                                Language::PtBr => format!("LOCAL apenas :{}", state.http_port),
                                Language::English => format!("LOCAL only :{}", state.http_port),
                            },
                            Color32::from_rgb(255, 193, 7),
                        ),
                        PortStatus::Stopped => (
                            match state.language {
                                Language::PtBr => "PARADO".to_string(),
                                Language::English => "STOPPED".to_string(),
                            },
                            Color32::from_rgb(239, 83, 80),
                        ),
                    };
                    ui.label(RichText::new(status_txt).color(status_color).strong().size(13.0));
                    status_dot(ui, status_color);
                });
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let start_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "Iniciar",
                        Language::English => "Start",
                    })
                    .color(Color32::WHITE)
                    .strong(),
                )
                .fill(Color32::from_rgb(76, 175, 80))
                .min_size(Vec2::new(90.0, 30.0))
                .rounding(Rounding::same(6.0));

                if ui.add(start_btn).clicked() {
                    state.start_daemon();
                }

                let stop_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "Parar",
                        Language::English => "Stop",
                    })
                    .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(239, 83, 80))
                .min_size(Vec2::new(90.0, 30.0))
                .rounding(Rounding::same(6.0));

                if ui.add(stop_btn).clicked() {
                    state.stop_daemon();
                }

                let kick_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "Reiniciar",
                        Language::English => "Restart",
                    })
                    .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(84, 110, 122))
                .min_size(Vec2::new(90.0, 30.0))
                .rounding(Rounding::same(6.0));

                if ui.add(kick_btn).clicked() {
                    state.kick_autostart();
                }

                ui.add_space(16.0);

                let refresh_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "Testar Endpoint",
                        Language::English => "Test Endpoint",
                    })
                    .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(33, 150, 243))
                .min_size(Vec2::new(120.0, 30.0))
                .rounding(Rounding::same(6.0));

                if ui.add(refresh_btn).clicked() {
                    state.check_port_status();
                }

                #[cfg(target_os = "windows")]
                {
                    ui.add_space(16.0);
                    let mut autostart = state.autostart_enabled;
                    let label = match state.language {
                        Language::PtBr => "Iniciar com Windows",
                        Language::English => "Start with Windows",
                    };
                    if ui.checkbox(&mut autostart, label).changed() {
                        state.set_autostart(autostart);
                    }
                }
            });
        });
}

// ─── Seção 2 (FIXA): configuração de porta/IP + encaminhamento, lado a lado ───
fn render_controls_row(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Coluna 1: Configuração de Porta & Rede
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(12.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Configuração de Porta & Rede",
                        Language::English => "Port & Network Configuration",
                    })
                    .size(14.0)
                    .strong()
                    .color(Color32::WHITE),
                );

                ui.add_space(6.0);

                ui.label(match state.language {
                    Language::PtBr => "Porta TCP HTTP (padrão 8000):",
                    Language::English => "HTTP TCP Port (default 8000):",
                });
                ui.add(egui::TextEdit::singleline(&mut state.http_port).min_size(Vec2::new(180.0, 24.0)));

                ui.add_space(4.0);

                ui.label(match state.language {
                    Language::PtBr => "Endereço IP da LAN (autodetectado, editável):",
                    Language::English => "LAN IP Address (autodetected, editable):",
                });
                ui.add(egui::TextEdit::singleline(&mut state.lan_ip).min_size(Vec2::new(180.0, 24.0)));

                ui.add_space(8.0);

                // Lado a lado para nao inchar a area FIXA na vertical.
                ui.horizontal(|ui| {
                    let apply_env_btn = egui::Button::new(
                        RichText::new(match state.language {
                            Language::PtBr => "Aplicar Porta + Bind 0.0.0.0",
                            Language::English => "Apply Port + Bind 0.0.0.0",
                        })
                        .color(Color32::WHITE)
                        .strong(),
                    )
                    .fill(Color32::from_rgb(33, 150, 243))
                    .min_size(Vec2::new(170.0, 28.0))
                    .rounding(Rounding::same(6.0));

                    if ui.add(apply_env_btn).clicked() {
                        state.apply_env_port();
                    }

                    let check_update_btn = egui::Button::new(
                        RichText::new(match state.language {
                            Language::PtBr => "Verificar Atualizações",
                            Language::English => "Check for Updates",
                        })
                        .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(156, 39, 176))
                    .min_size(Vec2::new(150.0, 28.0))
                    .rounding(Rounding::same(6.0));

                    if ui.add(check_update_btn).clicked() {
                        state.check_for_updates();
                    }
                });
            });

        // Coluna 2: Encaminhamento de Porta (LAN -> localhost)
        Frame::none()
            .fill(Color32::from_rgb(24, 32, 42))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(12.0))
            .show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        // "->" ASCII de proposito: a fonte proporcional padrao
                        // do egui nao tem o glifo "→" (viraria caixa quebrada).
                        RichText::new(match state.language {
                            Language::PtBr => "Encaminhamento LAN -> localhost",
                            Language::English => "Forwarding LAN -> localhost",
                        })
                        .size(14.0)
                        .strong()
                        .color(Color32::WHITE),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Badge de TRES estados honestos:
                        //   FUNCIONANDO = regra na config E listener de pe
                        //                 no netstat (verificado, nao suposto);
                        //   SEM EFEITO  = regra existe na config do netsh mas
                        //                 o listener NAO subiu (IP Helper);
                        //   SEM REGRA   = nada configurado para o par IP:porta.
                        let (badge_txt, badge_color) = if state.portproxy_active
                            && state.portproxy_effective
                        {
                            (
                                match state.language {
                                    Language::PtBr => "REGRA FUNCIONANDO",
                                    Language::English => "RULE WORKING",
                                },
                                Color32::from_rgb(76, 175, 80),
                            )
                        } else if state.portproxy_active {
                            (
                                match state.language {
                                    Language::PtBr => "REGRA SEM EFEITO",
                                    Language::English => "RULE NOT EFFECTIVE",
                                },
                                Color32::from_rgb(239, 83, 80),
                            )
                        } else {
                            (
                                match state.language {
                                    Language::PtBr => "SEM REGRA",
                                    Language::English => "NO RULE",
                                },
                                Color32::from_rgb(255, 193, 7),
                            )
                        };
                        ui.label(RichText::new(badge_txt).color(badge_color).strong().size(12.0));
                        status_dot(ui, badge_color);
                    });
                });

                ui.add_space(6.0);

                // Mapeamento publicado: o que entra pelo IP da LAN e entregue
                // ao listener local do CUA Driver.
                let mapping = format!(
                    "{}:{}  ->  127.0.0.1:{}",
                    state.lan_ip.trim(),
                    state.http_port.trim(),
                    state.http_port.trim()
                );
                ui.code(&mapping);

                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "netsh interface portproxy (depende do serviço IP Helper). Pode pedir elevação (UAC).",
                        Language::English => "netsh interface portproxy (depends on the IP Helper service). May request elevation (UAC).",
                    })
                    .size(11.0)
                    .color(Color32::from_rgb(150, 150, 150)),
                );

                if state.portproxy_active && !state.portproxy_effective {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "A regra está na config do netsh mas o listener NÃO está de pé (teste real falhou). Reinicie o serviço IP Helper (iphlpsvc) ou Remova e reaplique.",
                            Language::English => "The rule is in the netsh config but the listener is NOT up (real test failed). Restart the IP Helper service (iphlpsvc) or Remove and re-apply.",
                        })
                        .size(11.0)
                        .color(Color32::from_rgb(239, 83, 80)),
                    );
                }

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let apply_btn = egui::Button::new(
                        RichText::new(match state.language {
                            Language::PtBr => "Aplicar Regra",
                            Language::English => "Apply Rule",
                        })
                        .color(Color32::WHITE)
                        .strong(),
                    )
                    .fill(Color32::from_rgb(76, 175, 80))
                    .min_size(Vec2::new(105.0, 28.0))
                    .rounding(Rounding::same(6.0));

                    if ui.add(apply_btn).clicked() {
                        state.apply_portproxy();
                    }

                    let remove_btn = egui::Button::new(
                        RichText::new(match state.language {
                            Language::PtBr => "Remover Regra",
                            Language::English => "Remove Rule",
                        })
                        .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(239, 83, 80))
                    .min_size(Vec2::new(105.0, 28.0))
                    .rounding(Rounding::same(6.0));

                    if ui.add(remove_btn).clicked() {
                        state.remove_portproxy();
                    }

                    let status_btn = egui::Button::new(
                        RichText::new(match state.language {
                            Language::PtBr => "Atualizar Status",
                            Language::English => "Refresh Status",
                        })
                        .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(84, 110, 122))
                    .min_size(Vec2::new(105.0, 28.0))
                    .rounding(Rounding::same(6.0));

                    if ui.add(status_btn).clicked() {
                        // check_port_status ja recalcula o badge do portproxy
                        // (regra na config + listener no netstat) num lugar so.
                        state.check_port_status();
                    }
                });
            });
    });
}

// ─── Diagnóstico (ROLÁVEL): endpoint real, listeners e regras existentes ───
fn render_diagnostics(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(Color32::from_rgb(38, 38, 38))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            // Transporte real do daemon: HTTP com JSON-RPC (MCP).
            // NAO existe WebSocket aqui — nao anuncie o que nao ha.
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Endpoint MCP HTTP (JSON-RPC) — estado real",
                    Language::English => "MCP HTTP Endpoint (JSON-RPC) — actual state",
                })
                .size(14.0)
                .strong()
                .color(Color32::WHITE),
            );

            ui.add_space(6.0);

            egui::Grid::new("endpoint_grid")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new(match state.language {
                        Language::PtBr => "Porta TCP",
                        Language::English => "TCP Port",
                    }).strong().color(Color32::from_rgb(180, 180, 180)));
                    ui.label(RichText::new("Host / IP").strong().color(Color32::from_rgb(180, 180, 180)));
                    ui.label(RichText::new("Transport").strong().color(Color32::from_rgb(180, 180, 180)));
                    ui.label(RichText::new("Status").strong().color(Color32::from_rgb(180, 180, 180)));
                    ui.end_row();

                    ui.label(&state.http_port);
                    // Host REAL onde o endpoint responde — nunca o IP
                    // "de intencao". LOCAL apenas => 127.0.0.1, e ponto.
                    ui.label(match state.port_status {
                        PortStatus::LanListening => state.lan_ip.trim().to_string(),
                        PortStatus::LocalOnly => "127.0.0.1 (loopback)".to_string(),
                        PortStatus::Stopped => "-".to_string(),
                    });
                    ui.label(RichText::new("HTTP / JSON-RPC").color(Color32::from_rgb(255, 167, 38)));
                    match state.port_status {
                        PortStatus::LanListening => {
                            ui.label(
                                RichText::new("LISTENING (local + LAN)")
                                    .color(Color32::from_rgb(76, 175, 80))
                                    .strong(),
                            );
                        }
                        PortStatus::LocalOnly => {
                            ui.label(
                                RichText::new(match state.language {
                                    Language::PtBr => "LOCAL APENAS",
                                    Language::English => "LOCAL ONLY",
                                })
                                .color(Color32::from_rgb(255, 193, 7))
                                .strong(),
                            );
                        }
                        PortStatus::Stopped => {
                            ui.label(
                                RichText::new("STOPPED")
                                    .color(Color32::from_rgb(239, 83, 80))
                                    .strong(),
                            );
                        }
                    }
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.label(match state.language {
                Language::PtBr => "URL de Conexão MCP (estado real):",
                Language::English => "MCP Connection URL (actual state):",
            });

            // A URL exibida e a que FUNCIONA AGORA: so mostra o IP da
            // LAN quando o netstat + TCP confirmaram o listener na LAN.
            let mcp_url = match state.port_status {
                PortStatus::LanListening => format!(
                    "http://{}:{}/mcp",
                    state.lan_ip.trim(),
                    state.http_port.trim()
                ),
                _ => format!("http://127.0.0.1:{}/mcp", state.http_port.trim()),
            };
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

            match state.port_status {
                PortStatus::LanListening => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Acessível pela LAN (confirmado por netstat + TCP).",
                            Language::English => "Reachable from the LAN (confirmed via netstat + TCP).",
                        })
                        .size(11.0)
                        .color(Color32::from_rgb(76, 175, 80)),
                    );
                }
                PortStatus::LocalOnly => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "O endpoint REAL está só em 127.0.0.1 — agentes remotos NÃO alcançam. Publique na LAN com o bind 0.0.0.0 (botão de env) ou o encaminhamento.",
                            Language::English => "The REAL endpoint is only on 127.0.0.1 — remote agents CANNOT reach it. Publish on the LAN via the 0.0.0.0 bind (env button) or the forwarding.",
                        })
                        .size(11.0)
                        .color(Color32::from_rgb(255, 193, 7)),
                    );
                }
                PortStatus::Stopped => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Nada está ouvindo nesta porta — inicie o daemon (botão Iniciar).",
                            Language::English => "Nothing is listening on this port — start the daemon (Start button).",
                        })
                        .size(11.0)
                        .color(Color32::from_rgb(239, 83, 80)),
                    );
                }
            }

            // Conexoes CRUS do netstat, MESMAS colunas do terminal do
            // usuario: LISTENING (em espera) e ESTABLISHED (conexoes MCP
            // reais em andamento), incluindo portas orfas no IP da LAN.
            ui.add_space(8.0);
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Conexões reais na porta (netstat -ano) — LISTENING + ESTABLISHED:",
                    Language::English => "Actual connections on the port (netstat -ano) — LISTENING + ESTABLISHED:",
                })
                .size(11.0)
                .color(Color32::from_rgb(160, 160, 160)),
            );
            if state.real_listeners.is_empty() {
                ui.monospace(match state.language {
                    Language::PtBr => "(nenhuma conexão na porta configurada nem no IP da LAN)",
                    Language::English => "(no connection on the configured port nor on the LAN IP)",
                });
            } else {
                ui.monospace(format!(
                    "{:<5}{:<23}{:<23}{:<13}{}",
                    "PROT", "LOCAL", "REMOTO", "ESTADO", "PID"
                ));
                for line in &state.real_listeners {
                    ui.monospace(line);
                }
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "REMOTO 0.0.0.0:0 = listener em espera (formato padrão do Windows) — não é um destino.",
                        Language::English => "REMOTE 0.0.0.0:0 = listener waiting for connections (standard Windows format) — not a destination.",
                    })
                    .size(10.0)
                    .color(Color32::from_rgb(130, 130, 130)),
                );
            }

            // Regras v4tov4 EXISTENTES, cruas do netsh — inclusive regras
            // orfas em outras portas (para o usuario enxergar e limpar:
            // ajuste o campo Porta para a porta da regra e use Remover).
            if !state.portproxy_rules.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Regras portproxy existentes (netsh show v4tov4):",
                        Language::English => "Existing portproxy rules (netsh show v4tov4):",
                    })
                    .size(11.0)
                    .color(Color32::from_rgb(160, 160, 160)),
                );
                for rule in &state.portproxy_rules {
                    ui.monospace(rule);
                }
            }
        });
}

// ─── Console Debug: faixa fixa no rodapé da aba, SEMPRE visível ───
fn render_console(ui: &mut Ui, state: &mut AppState) {
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
                    if ui
                        .button(match state.language {
                            Language::PtBr => "Copiar",
                            Language::English => "Copy",
                        })
                        .clicked()
                    {
                        let log = state.debug_log.clone();
                        ui.output_mut(|o| o.copied_text = log);
                    }
                });
            });

            ui.add_space(6.0);

            // Sem altura cravada: o console ocupa o espaco que foi
            // reservado para ele no render() (minimo de 120px).
            let console_height = (ui.available_height() - 8.0).max(120.0);

            egui::ScrollArea::vertical()
                .id_salt("debug_console_scroll")
                .max_height(console_height)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if state.debug_log.is_empty() {
                        ui.monospace(match state.language {
                            Language::PtBr => "(nenhum comando executado ainda)",
                            Language::English => "(no command executed yet)",
                        });
                    } else {
                        let formatted_log = format!("{}\n\n", state.debug_log.trim_end());
                        ui.monospace(&formatted_log);
                    }
                });
        });
}
