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
                        Language::PtBr => "Verificar Atualizações",
                        Language::English => "Check for Updates",
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
                        // Badge do RELAY (caminho padrão desde a 2.3.0). Só
                        // dois estados, ambos verificáveis pelo próprio
                        // processo: o socket está escutando ou não está —
                        // sem o antigo limbo "regra existe mas não funciona"
                        // do netsh (que dependia do serviço IP Helper).
                        let (badge_txt, badge_color) = if state.lan_relay_listen.is_some() {
                            (
                                match state.language {
                                    Language::PtBr => "PUBLICADO NA REDE",
                                    Language::English => "PUBLISHED ON NETWORK",
                                },
                                ST_OK,
                            )
                        } else {
                            (
                                match state.language {
                                    Language::PtBr => "SÓ LOCAL",
                                    Language::English => "LOCAL ONLY",
                                },
                                ST_WARN,
                            )
                        };
                        ui.label(RichText::new(badge_txt).color(badge_color).strong().size(12.0));
                        status_dot(ui, badge_color);
                    });
                });

                ui.add_space(6.0);

                // Mapeamento publicado: o que entra pela rede e entregue ao
                // listener local do CUA Driver.
                let listen = state
                    .lan_relay_listen
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| state.http_port.trim().to_string());
                let target = state
                    .lan_relay_target
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| state.http_port.trim().to_string());
                let mapping = format!(
                    "{}:{}  ->  127.0.0.1:{}",
                    state.lan_relay_bind.trim(),
                    listen,
                    target
                );
                ui.code(&mapping);

                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Relay TCP do próprio aplicativo: não pede UAC, não deixa regra no sistema e é encerrado junto com o app. Escutar em 0.0.0.0 convive com o motor em 127.0.0.1 na MESMA porta.",
                        Language::English => "TCP relay inside this app: no UAC, leaves no system rule and shuts down with the app. Listening on 0.0.0.0 coexists with the engine on 127.0.0.1 on the SAME port.",
                    })
                    .size(11.0)
                    .color(TERM_GRAY),
                );

                // Endereço de escuta editável: 0.0.0.0 (qualquer interface,
                // padrão) ou um IP específico da máquina.
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Escutar em:",
                            Language::English => "Listen on:",
                        })
                        .size(11.0),
                    );
                    ui.add_enabled(
                        state.lan_relay_listen.is_none(),
                        egui::TextEdit::singleline(&mut state.lan_relay_bind)
                            .min_size(Vec2::new(110.0, 22.0)),
                    );
                    if state.lan_relay_listen.is_none() {
                        if ui.small_button("0.0.0.0").clicked() {
                            state.lan_relay_bind = "0.0.0.0".to_string();
                        }
                        let lan = state.lan_ip.trim().to_string();
                        if !lan.is_empty() && ui.small_button(&lan).clicked() {
                            state.lan_relay_bind = lan;
                        }
                    }
                });

                // Uso REAL (não "deve estar funcionando"): conexões contadas
                // pelas próprias threads do relay.
                if state.lan_relay_listen.is_some() {
                    use std::sync::atomic::Ordering;
                    let ativas = state.lan_relay_conns.load(Ordering::SeqCst);
                    let total = state.lan_relay_total.load(Ordering::SeqCst);
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => format!("conexões: {} ativas / {} desde o início", ativas, total),
                            Language::English => format!("connections: {} active / {} since start", ativas, total),
                        })
                        .size(11.0)
                        .color(TERM_GREEN),
                    );
                }

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    let publish_btn = term_button(match state.language {
                        Language::PtBr => "Publicar na rede",
                        Language::English => "Publish on network",
                    })
                    .min_size(Vec2::new(130.0, 28.0));
                    if ui
                        .add_enabled(state.lan_relay_listen.is_none(), publish_btn)
                        .clicked()
                    {
                        state.start_lan_relay();
                    }

                    let stop_btn = term_button_danger(match state.language {
                        Language::PtBr => "Parar",
                        Language::English => "Stop",
                    })
                    .min_size(Vec2::new(80.0, 28.0));
                    if ui
                        .add_enabled(state.lan_relay_listen.is_some(), stop_btn)
                        .clicked()
                    {
                        state.stop_lan_relay();
                    }

                    let status_btn = term_button(match state.language {
                        Language::PtBr => "Atualizar Status",
                        Language::English => "Refresh Status",
                    })
                    .min_size(Vec2::new(105.0, 28.0));

                    if ui.add(status_btn).clicked() {
                        state.check_port_status();
                    }
                });

                // ─── Regras portproxy LEGADAS ────────────────────────────
                // O netsh deixou de ser o caminho padrão, mas máquinas que
                // usaram versões anteriores têm regras persistidas (elas
                // sobrevivem a reboot). Oferecemos a remoção explícita —
                // some da tela quando não houver nenhuma.
                if state.portproxy_active {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Existe uma regra netsh portproxy antiga para este IP:porta (de versões anteriores). Ela persiste após reiniciar o Windows e não é mais necessária.",
                            Language::English => "There is an old netsh portproxy rule for this IP:port (from earlier versions). It survives reboots and is no longer needed.",
                        })
                        .size(11.0)
                        .color(ST_WARN),
                    );
                    let remove_btn = term_button_danger(match state.language {
                        Language::PtBr => "Remover regra antiga (UAC)",
                        Language::English => "Remove old rule (UAC)",
                    })
                    .min_size(Vec2::new(180.0, 26.0));
                    if ui.add(remove_btn).clicked() {
                        state.remove_portproxy();
                    }
                }
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
