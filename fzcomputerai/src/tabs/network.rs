use crate::app::{
    status_dot, term_button, term_button_danger, AppState, Language, PortStatus, TERM_BG_PANEL,
    TERM_GRAY, TERM_GREEN, TERM_GREEN_BRIGHT, TERM_WHITE, ST_ERR, ST_OK, ST_WARN,
};
use egui::{Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Layout com ANCORAS FIXAS: controles do daemon, configuracao de
    // porta/IP e encaminhamento ficam SEMPRE visiveis (nenhum controle
    // interativo rola para fora da tela); o DIAGNOSTICO (endpoint real,
    // listeners do netstat, regras portproxy) e a unica area rolavel.
    // A saida de comandos vive no console global do rodape da janela.
    // Motor ausente = NADA funciona. Este painel cumpre o contrato que o
    // instalador anuncia ao usuario ("instale o motor depois pelo proprio
    // aplicativo"), que antes nao existia no codigo.
    if !state.driver_present {
        render_missing_engine(ui, state);
        ui.add_space(10.0);
    }
    render_daemon_controls(ui, state);
    ui.add_space(10.0);
    render_controls_row(ui, state);
    ui.add_space(10.0);

    // Sem console na aba: o diagnostico recebe todo o espaco restante.
    egui::ScrollArea::vertical()
        .id_salt("network_diag_scroll")
        .max_height(ui.available_height())
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_diagnostics(ui, state);
        });
}

// ─── Motor ausente: instalar pelo instalador OFICIAL do projeto Cua ───
fn render_missing_engine(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(egui::Color32::from_rgb(60, 20, 20))
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_dot(ui, ST_ERR);
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Motor cua-driver NAO encontrado",
                        Language::English => "cua-driver engine NOT found",
                    })
                    .size(14.0)
                    .strong()
                    .color(ST_ERR),
                );
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "O cua-driver nao e um extra: e o motor que executa clique, digitacao e captura de tela. Sem ele o aplicativo abre, mas nenhuma acao funciona. A instalacao usa o instalador OFICIAL do projeto Cua.",
                    Language::English => "cua-driver is not an add-on: it is the engine that performs clicking, typing and screen capture. Without it the app opens but no action works. Installation uses the Cua project's OFFICIAL installer.",
                })
                .size(11.0)
                .color(TERM_GRAY),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let btn = term_button(match state.language {
                    Language::PtBr => "Instalar motor cua-driver",
                    Language::English => "Install cua-driver engine",
                })
                .min_size(Vec2::new(210.0, 30.0));
                if ui.add_enabled(!state.driver_updating, btn).clicked() {
                    state.install_driver_engine();
                }
                let recheck = term_button(match state.language {
                    Language::PtBr => "Verificar de novo",
                    Language::English => "Check again",
                })
                .min_size(Vec2::new(140.0, 30.0));
                if ui.add(recheck).clicked() {
                    state.check_driver_present();
                    state.check_port_status();
                }
                if state.driver_updating {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "instalando em segundo plano...",
                            Language::English => "installing in the background...",
                        })
                        .size(11.0)
                        .color(TERM_GRAY),
                    );
                }
            });
        });
}

// ─── Seção 1 (FIXA no topo): controles do daemon + status ───
fn render_daemon_controls(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(match state.language {
                        // Titulo curto de proposito: em fonte mono o texto e
                        // ~15% mais largo e o titulo longo colidia com o badge
                        // de status alinhado a direita na mesma linha.
                        Language::PtBr => "Serviço CUA Driver",
                        Language::English => "CUA Driver Service",
                    })
                    .size(15.0)
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
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
                            ST_OK,
                        ),
                        PortStatus::LocalOnly => (
                            match state.language {
                                Language::PtBr => format!("LOCAL apenas :{}", state.http_port),
                                Language::English => format!("LOCAL only :{}", state.http_port),
                            },
                            ST_WARN,
                        ),
                        PortStatus::Stopped => (
                            match state.language {
                                Language::PtBr => "PARADO".to_string(),
                                Language::English => "STOPPED".to_string(),
                            },
                            ST_ERR,
                        ),
                    };
                    ui.label(RichText::new(status_txt).color(status_color).strong().size(13.0));
                    status_dot(ui, status_color);
                });
            });

            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                let start_btn = term_button(match state.language {
                    Language::PtBr => "Iniciar",
                    Language::English => "Start",
                })
                .min_size(Vec2::new(90.0, 30.0));

                if ui.add(start_btn).clicked() {
                    state.start_daemon();
                }

                let stop_btn = term_button_danger(match state.language {
                    Language::PtBr => "Parar",
                    Language::English => "Stop",
                })
                .min_size(Vec2::new(90.0, 30.0));

                if ui.add(stop_btn).clicked() {
                    state.stop_daemon();
                }

                let kick_btn = term_button(match state.language {
                    Language::PtBr => "Reiniciar",
                    Language::English => "Restart",
                })
                .min_size(Vec2::new(90.0, 30.0));

                if ui.add(kick_btn).clicked() {
                    state.kick_autostart();
                }

                ui.add_space(16.0);

                let refresh_btn = term_button(match state.language {
                    Language::PtBr => "Testar Endpoint",
                    Language::English => "Test Endpoint",
                })
                .min_size(Vec2::new(120.0, 30.0));

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
                    // No modo portatil o autostart fica INDISPONIVEL: ele exige
                    // escrever em HKCU\...\Run, que e exatamente o rastro que o
                    // portatil nao deve deixar na maquina.
                    let resp = ui.add_enabled(
                        !state.portable_mode,
                        egui::Checkbox::new(&mut autostart, label),
                    );
                    if state.portable_mode {
                        resp.on_hover_text(match state.language {
                            Language::PtBr => "Indisponivel no modo portatil (exigiria gravar no registro).",
                            Language::English => "Unavailable in portable mode (it would require writing to the registry).",
                        });
                    } else if resp.changed() {
                        state.set_autostart(autostart);
                    }

                    ui.add_space(12.0);
                    let mut tray = state.minimize_to_tray;
                    let tray_label = match state.language {
                        Language::PtBr => "Minimizar para a bandeja",
                        Language::English => "Minimize to tray",
                    };
                    if ui.checkbox(&mut tray, tray_label).changed() {
                        state.set_minimize_to_tray(tray);
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
            .fill(TERM_BG_PANEL)
            .rounding(Rounding::same(2.0))
            .inner_margin(Margin::same(12.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Configuração de Porta & Rede",
                        Language::English => "Port & Network Configuration",
                    })
                    .size(14.0)
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
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
                ui.horizontal_wrapped(|ui| {
                    let apply_env_btn = term_button(match state.language {
                        // NAO prometa "Bind 0.0.0.0": o motor oficial do Cua
                        // escuta somente em 127.0.0.1 (endereco fixo no codigo
                        // deles) e ignora qualquer variavel de bind. O botao
                        // aplica a PORTA, que e o que de fato tem efeito.
                        Language::PtBr => "Aplicar Porta",
                        Language::English => "Apply Port",
                    })
                    .min_size(Vec2::new(170.0, 28.0));

                    if ui.add(apply_env_btn).clicked() {
                        state.apply_env_port();
                    }

                    let check_update_btn = term_button(match state.language {
                        // O botão AGE: o que estiver desatualizado (GUI e/ou
                        // motor) já começa a baixar/atualizar — ver
                        // check_for_updates em app.rs.
                        Language::PtBr => "Verificar e Atualizar",
                        Language::English => "Check & Update",
                    })
                    .min_size(Vec2::new(150.0, 28.0));

                    if ui.add(check_update_btn).clicked() {
                        state.check_for_updates();
                    }
                });
            });

        // Coluna 2: Encaminhamento de Porta (LAN -> localhost)
        Frame::none()
            .fill(TERM_BG_PANEL)
            .rounding(Rounding::same(2.0))
            .inner_margin(Margin::same(12.0))
            .show(&mut cols[1], |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        // "->" ASCII de proposito: a fonte proporcional padrao
                        // do egui nao tem o glifo "→" (viraria caixa quebrada).
                        RichText::new(match state.language {
                            // Curto: dividia a linha com o badge de 3 estados.
                            Language::PtBr => "Encaminhamento LAN",
                            Language::English => "LAN Forwarding",
                        })
                        .size(14.0)
                        .strong()
                        .color(TERM_GREEN_BRIGHT),
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
                                ST_OK,
                            )
                        } else if state.portproxy_active {
                            (
                                match state.language {
                                    Language::PtBr => "REGRA SEM EFEITO",
                                    Language::English => "RULE NOT EFFECTIVE",
                                },
                                ST_ERR,
                            )
                        } else {
                            (
                                match state.language {
                                    Language::PtBr => "SEM REGRA",
                                    Language::English => "NO RULE",
                                },
                                ST_WARN,
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
                    .color(TERM_GRAY),
                );

                if state.portproxy_active && !state.portproxy_effective {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "A regra está na config do netsh mas o listener NÃO está de pé (teste real falhou). Reinicie o serviço IP Helper (iphlpsvc) ou Remova e reaplique.",
                            Language::English => "The rule is in the netsh config but the listener is NOT up (real test failed). Restart the IP Helper service (iphlpsvc) or Remove and re-apply.",
                        })
                        .size(11.0)
                        .color(ST_ERR),
                    );
                }

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    let apply_btn = term_button(match state.language {
                        Language::PtBr => "Aplicar Regra",
                        Language::English => "Apply Rule",
                    })
                    .min_size(Vec2::new(105.0, 28.0));

                    if ui.add(apply_btn).clicked() {
                        state.apply_portproxy();
                    }

                    let remove_btn = term_button_danger(match state.language {
                        Language::PtBr => "Remover Regra",
                        Language::English => "Remove Rule",
                    })
                    .min_size(Vec2::new(105.0, 28.0));

                    if ui.add(remove_btn).clicked() {
                        state.remove_portproxy();
                    }

                    let status_btn = term_button(match state.language {
                        Language::PtBr => "Atualizar Status",
                        Language::English => "Refresh Status",
                    })
                    .min_size(Vec2::new(105.0, 28.0));

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
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
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
                .color(TERM_GREEN_BRIGHT),
            );

            ui.add_space(6.0);

            egui::Grid::new("endpoint_grid")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new(match state.language {
                        Language::PtBr => "Porta TCP",
                        Language::English => "TCP Port",
                    }).strong().color(TERM_WHITE));
                    ui.label(RichText::new("Host / IP").strong().color(TERM_WHITE));
                    ui.label(RichText::new("Transport").strong().color(TERM_WHITE));
                    ui.label(RichText::new("Status").strong().color(TERM_WHITE));
                    ui.end_row();

                    ui.label(&state.http_port);
                    // Host REAL onde o endpoint responde — nunca o IP
                    // "de intencao". LOCAL apenas => 127.0.0.1, e ponto.
                    ui.label(match state.port_status {
                        PortStatus::LanListening => state.lan_ip.trim().to_string(),
                        PortStatus::LocalOnly => "127.0.0.1 (loopback)".to_string(),
                        PortStatus::Stopped => "-".to_string(),
                    });
                    ui.label(RichText::new("HTTP / JSON-RPC").color(TERM_GREEN));
                    match state.port_status {
                        PortStatus::LanListening => {
                            ui.label(
                                RichText::new("LISTENING (local + LAN)")
                                    .color(ST_OK)
                                    .strong(),
                            );
                        }
                        PortStatus::LocalOnly => {
                            ui.label(
                                RichText::new(match state.language {
                                    Language::PtBr => "LOCAL APENAS",
                                    Language::English => "LOCAL ONLY",
                                })
                                .color(ST_WARN)
                                .strong(),
                            );
                        }
                        PortStatus::Stopped => {
                            ui.label(
                                RichText::new("STOPPED")
                                    .color(ST_ERR)
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
            ui.horizontal_wrapped(|ui| {
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
                        .color(ST_OK),
                    );
                }
                PortStatus::LocalOnly => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "O endpoint REAL está só em 127.0.0.1 — agentes remotos NÃO alcançam. O motor oficial escuta apenas em loopback (endereço fixo no código do Cua): para a LAN use o Encaminhamento ao lado; para a internet, a aba Túnel.",
                            Language::English => "The REAL endpoint is only on 127.0.0.1 — remote agents CANNOT reach it. The official engine listens on loopback only (address hardcoded in Cua): use the Forwarding beside for the LAN, or the Tunnel tab for the internet.",
                        })
                        .size(11.0)
                        .color(ST_WARN),
                    );
                }
                PortStatus::Stopped => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Nada está ouvindo nesta porta — inicie o daemon (botão Iniciar).",
                            Language::English => "Nothing is listening on this port — start the daemon (Start button).",
                        })
                        .size(11.0)
                        .color(ST_ERR),
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
                .color(TERM_GRAY),
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
                    .color(TERM_GRAY),
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
                    .color(TERM_GRAY),
                );
                for rule in &state.portproxy_rules {
                    ui.monospace(rule);
                }
            }
        });
}
