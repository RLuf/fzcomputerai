use crate::app::{
    status_dot, term_button, term_button_danger, AppState, Language, PortStatus, TlsBind,
    TlsMode, TlsStatus, TERM_BG_PANEL, TERM_GRAY, TERM_GREEN, TERM_GREEN_BRIGHT, TERM_WHITE,
    ST_ERR, ST_OK, ST_WARN,
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
            // HTTPS entra na area rolavel: nao encolhe os controles fixos
            // que ja funcionavam e fica logo acima do diagnostico.
            render_https(ui, state);
            ui.add_space(10.0);
            render_diagnostics(ui, state);
        });
}

// ─── HTTPS (terminacao TLS no proprio app, ver src/tls.rs) ───
fn render_https(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "HTTPS do endpoint MCP",
                        Language::English => "MCP endpoint HTTPS",
                    })
                    .size(14.0)
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Badge HONESTO: verde so com handshake TLS + JSON-RPC reais.
                    let (txt, color) = match state.tls_status {
                        TlsStatus::Listening => (
                            format!("HTTPS ATIVO :{}", state.tls_port.trim()),
                            ST_OK,
                        ),
                        TlsStatus::ListeningNoMcp => (
                            match state.language {
                                Language::PtBr => format!("TLS OK, MOTOR NAO RESPONDE :{}", state.tls_port.trim()),
                                Language::English => format!("TLS OK, ENGINE NOT ANSWERING :{}", state.tls_port.trim()),
                            },
                            ST_WARN,
                        ),
                        TlsStatus::Error => (
                            match state.language {
                                Language::PtBr => "ERRO".to_string(),
                                Language::English => "ERROR".to_string(),
                            },
                            ST_ERR,
                        ),
                        TlsStatus::Stopped => (
                            match state.language {
                                Language::PtBr => "DESLIGADO".to_string(),
                                Language::English => "OFF".to_string(),
                            },
                            ST_WARN,
                        ),
                    };
                    ui.label(RichText::new(txt).color(color).strong().size(12.0));
                    status_dot(ui, color);
                });
            });

            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "O motor so fala HTTP em 127.0.0.1. Este listener termina o TLS dentro do app e encaminha para ele — mesmo desenho do Encaminhamento LAN (sem admin, some ao fechar). O bearer token continua obrigatorio.",
                    Language::English => "The engine only speaks HTTP on 127.0.0.1. This listener terminates TLS inside the app and forwards to it — same design as LAN Forwarding (no admin, gone on close). The bearer token is still required.",
                })
                .size(11.0)
                .color(TERM_GRAY),
            );
            ui.add_space(6.0);

            // Linha 1: ligar, porta, bind
            ui.horizontal_wrapped(|ui| {
                let mut on = state.tls_enabled;
                if ui
                    .checkbox(
                        &mut on,
                        match state.language {
                            Language::PtBr => "Ligar HTTPS (persistido; sobe com o app)",
                            Language::English => "Enable HTTPS (persisted; starts with the app)",
                        },
                    )
                    .changed()
                {
                    state.set_tls_enabled(on);
                }
                ui.add_space(12.0);
                ui.label(match state.language {
                    Language::PtBr => "Porta HTTPS:",
                    Language::English => "HTTPS port:",
                });
                ui.add(egui::TextEdit::singleline(&mut state.tls_port).desired_width(60.0));
                ui.add_space(12.0);
                ui.label(match state.language {
                    Language::PtBr => "Escutar em:",
                    Language::English => "Listen on:",
                });
                let lan = state.lan_ip.trim().to_string();
                egui::ComboBox::from_id_salt("tls_bind")
                    .selected_text(match state.tls_bind {
                        TlsBind::Loopback => "127.0.0.1".to_string(),
                        TlsBind::Lan => format!("{} (LAN)", lan),
                        TlsBind::All => "0.0.0.0 (todas)".to_string(),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.tls_bind, TlsBind::Loopback, "127.0.0.1");
                        ui.selectable_value(&mut state.tls_bind, TlsBind::Lan, format!("{} (LAN)", lan));
                        ui.selectable_value(&mut state.tls_bind, TlsBind::All, "0.0.0.0");
                    });
            });

            // Linha 2: origem do certificado
            ui.horizontal_wrapped(|ui| {
                ui.label(match state.language {
                    Language::PtBr => "Certificado:",
                    Language::English => "Certificate:",
                });
                let prev = state.tls_mode;
                ui.selectable_value(
                    &mut state.tls_mode,
                    TlsMode::SelfSigned,
                    match state.language {
                        Language::PtBr => "Auto-assinado (gerado pelo app)",
                        Language::English => "Self-signed (generated by the app)",
                    },
                );
                ui.selectable_value(&mut state.tls_mode, TlsMode::LetsEncrypt, "Let's Encrypt");
                ui.selectable_value(
                    &mut state.tls_mode,
                    TlsMode::Custom,
                    match state.language {
                        Language::PtBr => "Proprio (.crt/.key PEM)",
                        Language::English => "Own (.crt/.key PEM)",
                    },
                );
                if prev != state.tls_mode {
                    state.tls_refresh_cert_info();
                }
            });

            match state.tls_mode {
                TlsMode::SelfSigned => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Gerado na instalacao (fzcomputerai --tls-init) ou no primeiro run, o que vier primeiro. SANs: localhost, 127.0.0.1, IP da LAN, nome da maquina. NUNCA e instalado em store de confianca — o cliente confia pelo SHA-256 abaixo ou pelo .crt (curl --cacert).",
                            Language::English => "Generated at install time (fzcomputerai --tls-init) or on first run, whichever comes first. SANs: localhost, 127.0.0.1, LAN IP, machine name. NEVER installed into a trust store — the client trusts via the SHA-256 below or the .crt file (curl --cacert).",
                        })
                        .size(11.0)
                        .color(TERM_GRAY),
                    );
                }
                TlsMode::LetsEncrypt => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(match state.language {
                            Language::PtBr => "Dominio(s) publico(s), separados por virgula:",
                            Language::English => "Public domain(s), comma-separated:",
                        });
                        ui.add(egui::TextEdit::singleline(&mut state.tls_domain).desired_width(200.0));
                        ui.label("E-mail:");
                        ui.add(egui::TextEdit::singleline(&mut state.tls_email).desired_width(180.0));
                        ui.checkbox(
                            &mut state.tls_staging,
                            match state.language {
                                Language::PtBr => "Staging (teste)",
                                Language::English => "Staging (test)",
                            },
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(match state.language {
                            Language::PtBr => "Desafio:",
                            Language::English => "Challenge:",
                        });
                        ui.selectable_value(&mut state.tls_acme_dns, true, match state.language {
                            Language::PtBr => "DNS-01 via Cloudflare (rede interna)",
                            Language::English => "DNS-01 via Cloudflare (internal network)",
                        });
                        ui.selectable_value(&mut state.tls_acme_dns, false, match state.language {
                            Language::PtBr => "HTTP-01 (porta 80 publica)",
                            Language::English => "HTTP-01 (public port 80)",
                        });
                    });
                    if state.tls_acme_dns {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(match state.language {
                                Language::PtBr => "Token API Cloudflare (Zone.DNS:Edit):",
                                Language::English => "Cloudflare API token (Zone.DNS:Edit):",
                            });
                            ui.add(egui::TextEdit::singleline(&mut state.tls_cf_token_input).password(true).desired_width(260.0).hint_text(if state.tls_cf_token_saved { "(salvo em arquivo — cole para trocar)" } else { "cole o token" }));
                            if ui.button(match state.language { Language::PtBr => "Verificar token", Language::English => "Verify token" }).clicked() {
                                state.tls_cf_verify_token();
                            }
                            ui.checkbox(&mut state.tls_cf_a_record, match state.language {
                                Language::PtBr => "Criar/atualizar registro A -> IP da LAN",
                                Language::English => "Create/update A record -> LAN IP",
                            });
                        });
                        ui.label(
                            RichText::new(match state.language {
                                Language::PtBr => "DNS-01: o app cria o TXT _acme-challenge na zona do Cloudflare pela API, espera propagar, valida e remove. Nao precisa de porta aberta: o dominio pode apontar para o IP privado da LAN (registro A criado pelo app). O token fica em arquivo 0600 na pasta dos certificados, nunca no registro nem no console. Renovacao automatica com menos de 30 dias.",
                                Language::English => "DNS-01: the app creates the _acme-challenge TXT in the Cloudflare zone via API, waits for propagation, validates and removes it. No open port needed: the domain may point to the private LAN IP (A record created by the app). The token is stored in a 0600 file in the certificates folder, never in the registry or console. Auto-renews under 30 days.",
                            })
                            .size(11.0)
                            .color(TERM_GRAY),
                        );
                    } else {
                        ui.label(
                            RichText::new(match state.language {
                                Language::PtBr => "ACME HTTP-01 (RFC 8555): o DNS do dominio precisa apontar para o IP PUBLICO desta maquina e a porta 80 precisa chegar ate aqui (encaminhamento no roteador + firewall). O app abre um respondedor temporario em 0.0.0.0:80 so durante a emissao. Renovacao automatica com menos de 30 dias. Let's Encrypt nao emite para IP nem para nome de maquina sem DNS publico.",
                                Language::English => "ACME HTTP-01 (RFC 8555): the domain's DNS must point to this machine's PUBLIC IP and port 80 must reach here (router forwarding + firewall). The app opens a temporary responder on 0.0.0.0:80 only during issuance. Auto-renews under 30 days. Let's Encrypt does not issue for IPs or machine names without public DNS.",
                            })
                            .size(11.0)
                            .color(TERM_GRAY),
                        );
                    }
                }
                TlsMode::Custom => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(".crt:");
                        ui.add(egui::TextEdit::singleline(&mut state.tls_custom_cert).desired_width(260.0));
                        ui.label(".key:");
                        ui.add(egui::TextEdit::singleline(&mut state.tls_custom_key).desired_width(260.0));
                    });
                }
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let apply = term_button(match state.language {
                    Language::PtBr => "Aplicar / Reiniciar HTTPS",
                    Language::English => "Apply / Restart HTTPS",
                })
                .min_size(Vec2::new(170.0, 28.0));
                if ui.add(apply).clicked() {
                    state.tls_save_cfg();
                    state.tls_refresh_cert_info();
                    if state.tls_enabled {
                        state.start_tls();
                    }
                }
                let test = term_button(match state.language {
                    Language::PtBr => "Testar HTTPS",
                    Language::English => "Test HTTPS",
                })
                .min_size(Vec2::new(110.0, 28.0));
                if ui.add(test).clicked() {
                    state.tls_refresh_cert_info();
                    state.check_tls_status();
                }
                match state.tls_mode {
                    TlsMode::SelfSigned => {
                        let regen = term_button_danger(match state.language {
                            Language::PtBr => "Regenerar auto-assinado",
                            Language::English => "Regenerate self-signed",
                        })
                        .min_size(Vec2::new(170.0, 28.0));
                        if ui.add(regen).clicked() {
                            state.tls_regenerate_self_signed();
                        }
                    }
                    TlsMode::LetsEncrypt => {
                        let issue = term_button(match state.language {
                            Language::PtBr => "Emitir Let's Encrypt",
                            Language::English => "Issue Let's Encrypt",
                        })
                        .min_size(Vec2::new(150.0, 28.0));
                        if ui.add_enabled(!state.tls_acme_busy, issue).clicked() {
                            state.tls_issue_letsencrypt();
                        }
                        if state.tls_acme_busy {
                            ui.label(
                                RichText::new(match state.language {
                                    Language::PtBr => "emitindo... (acompanhe no console)",
                                    Language::English => "issuing... (follow the console)",
                                })
                                .size(11.0)
                                .color(TERM_GRAY),
                            );
                        }
                    }
                    TlsMode::Custom => {}
                }
                let open_dir = term_button(match state.language {
                    Language::PtBr => "Abrir pasta dos certificados",
                    Language::English => "Open certificates folder",
                })
                .min_size(Vec2::new(180.0, 28.0));
                if ui.add(open_dir).clicked() {
                    let dir = state.tls_cert_dir.clone();
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = open::that(&dir);
                    state.log_debug(&format!("[https] Pasta dos certificados: {}", dir.display()));
                }
            });

            // ─── OAuth 2.1 para conectores ───
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let mut oa = state.oauth_enabled;
                if ui
                    .checkbox(&mut oa, match state.language {
                        Language::PtBr => "OAuth 2.1 para conectores (Claude.ai, Gemini, clientes MCP com login)",
                        Language::English => "OAuth 2.1 for connectors (Claude.ai, Gemini, MCP clients with login)",
                    })
                    .changed()
                {
                    state.set_oauth_enabled(oa);
                }
            });
            if state.oauth_enabled {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "O conector descobre /.well-known/oauth-authorization-server, registra-se sozinho, abre a pagina /authorize (pede a SENHA DE AUTORIZACAO abaixo) e recebe um token proprio. O app troca esse token pelo bearer do motor ao encaminhar — o conector nunca ve o token do motor. Precisa do HTTPS com certificado que o conector confie (Let's Encrypt).",
                        Language::English => "The connector discovers /.well-known/oauth-authorization-server, registers itself, opens the /authorize page (asks for the AUTHORIZATION PASSWORD below) and gets its own token. The app swaps that token for the engine bearer when forwarding — the connector never sees the engine token. Requires HTTPS with a certificate the connector trusts (Let's Encrypt).",
                    })
                    .size(11.0)
                    .color(TERM_GRAY),
                );
                ui.horizontal_wrapped(|ui| {
                    let gen = term_button(match state.language {
                        Language::PtBr => if state.oauth_has_password { "Gerar nova senha de autorizacao" } else { "Gerar senha de autorizacao" },
                        Language::English => if state.oauth_has_password { "Generate new authorization password" } else { "Generate authorization password" },
                    })
                    .min_size(Vec2::new(210.0, 28.0));
                    if ui.add(gen).clicked() {
                        state.oauth_generate_password();
                    }
                    let revoke = term_button_danger(match state.language {
                        Language::PtBr => "Revogar todos os conectores",
                        Language::English => "Revoke all connectors",
                    })
                    .min_size(Vec2::new(180.0, 28.0));
                    if ui.add(revoke).clicked() {
                        state.oauth_revoke_all();
                    }
                    ui.label(
                        RichText::new(format!(
                            "{}: {}   {}: {}",
                            match state.language { Language::PtBr => "conectores", Language::English => "connectors" },
                            state.oauth_clients,
                            match state.language { Language::PtBr => "tokens ativos", Language::English => "active tokens" },
                            state.oauth_tokens
                        ))
                        .size(11.0)
                        .color(TERM_GRAY),
                    );
                });
                if !state.oauth_password_shown.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(match state.language {
                            Language::PtBr => "Senha de autorizacao (mostrada UMA vez — guarde):",
                            Language::English => "Authorization password (shown ONCE — keep it):",
                        }).color(ST_WARN));
                        ui.code(&state.oauth_password_shown);
                        if ui.button(match state.language { Language::PtBr => "Copiar", Language::English => "Copy" }).clicked() {
                            ui.output_mut(|o| o.copied_text = state.oauth_password_shown.clone());
                        }
                        if ui.button(match state.language { Language::PtBr => "Ocultar", Language::English => "Hide" }).clicked() {
                            state.oauth_password_shown.clear();
                        }
                    });
                } else if !state.oauth_has_password {
                    ui.label(RichText::new(match state.language {
                        Language::PtBr => "Sem senha de autorizacao: nenhum conector consegue autorizar. Gere uma.",
                        Language::English => "No authorization password: no connector can authorize. Generate one.",
                    }).size(11.0).color(ST_ERR));
                }
            }

            if !state.tls_last_error.is_empty() && state.tls_status == TlsStatus::Error {
                ui.label(RichText::new(&state.tls_last_error).size(11.0).color(ST_ERR));
            }

            // Certificado REAL em uso (lido do arquivo/do handshake)
            ui.add_space(6.0);
            let info = state
                .tls_probe
                .as_ref()
                .and_then(|p| p.cert.clone())
                .or_else(|| state.tls_cert_info.clone());
            match info {
                Some(c) => {
                    let color = if c.expired() { ST_ERR } else if c.needs_renewal() { ST_WARN } else { ST_OK };
                    egui::Grid::new("tls_cert_grid").spacing([12.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new(match state.language { Language::PtBr => "Arquivo", Language::English => "File" }).strong().color(TERM_WHITE));
                        ui.monospace(&state.tls_cert_path);
                        ui.end_row();
                        ui.label(RichText::new(match state.language { Language::PtBr => "Emissor", Language::English => "Issuer" }).strong().color(TERM_WHITE));
                        ui.monospace(if c.self_signed {
                            match state.language { Language::PtBr => "auto-assinado (o proprio app)".to_string(), Language::English => "self-signed (this app)".to_string() }
                        } else { c.issuer.clone() });
                        ui.end_row();
                        ui.label(RichText::new("SANs").strong().color(TERM_WHITE));
                        ui.monospace(c.sans.join(", "));
                        ui.end_row();
                        ui.label(RichText::new(match state.language { Language::PtBr => "Validade", Language::English => "Validity" }).strong().color(TERM_WHITE));
                        ui.label(RichText::new(format!("{}  ->  {}  ({} {})", c.not_before, c.not_after, c.days_left, match state.language { Language::PtBr => "dias", Language::English => "days" })).color(color));
                        ui.end_row();
                        ui.label(RichText::new("SHA-256").strong().color(TERM_WHITE));
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(&c.sha256_fingerprint);
                            if ui.button(match state.language { Language::PtBr => "Copiar", Language::English => "Copy" }).clicked() {
                                ui.output_mut(|o| o.copied_text = c.sha256_fingerprint.clone());
                            }
                        });
                        ui.end_row();
                    });
                }
                None => {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Nenhum certificado carregado para o modo selecionado.",
                            Language::English => "No certificate loaded for the selected mode.",
                        })
                        .size(11.0)
                        .color(ST_WARN),
                    );
                }
            }

            // URL HTTPS que funciona AGORA
            if matches!(state.tls_status, TlsStatus::Listening | TlsStatus::ListeningNoMcp) {
                ui.add_space(6.0);
                let url = state.tls_url();
                ui.horizontal_wrapped(|ui| {
                    ui.label(match state.language {
                        Language::PtBr => "URL HTTPS (estado real):",
                        Language::English => "HTTPS URL (actual state):",
                    });
                    ui.code(&url);
                    if ui.button(match state.language { Language::PtBr => "Copiar", Language::English => "Copy" }).clicked() {
                        ui.output_mut(|o| o.copied_text = url.clone());
                    }
                });
                if let Some(p) = &state.tls_probe {
                    ui.label(
                        RichText::new(format!(
                            "{}: {}  |  HTTP {}  |  JSON-RPC: {}  |  {}: {}",
                            match state.language { Language::PtBr => "protocolo", Language::English => "protocol" },
                            p.protocol,
                            p.http_status,
                            if p.jsonrpc { "OK" } else { "-" },
                            match state.language { Language::PtBr => "conexoes aceitas", Language::English => "accepted connections" },
                            state.tls_accepted
                        ))
                        .size(11.0)
                        .color(TERM_GRAY),
                    );
                }
            }
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

                    // Linha HTTPS — mesmo criterio honesto (sonda TLS + JSON-RPC).
                    ui.label(&state.tls_port);
                    ui.label(match state.tls_status {
                        TlsStatus::Listening | TlsStatus::ListeningNoMcp => {
                            if state.tls_probe_lan_ok { state.lan_ip.trim().to_string() } else { "127.0.0.1 (loopback)".to_string() }
                        }
                        _ => "-".to_string(),
                    });
                    ui.label(RichText::new("HTTPS / TLS -> HTTP").color(TERM_GREEN));
                    match state.tls_status {
                        TlsStatus::Listening => { ui.label(RichText::new("LISTENING (TLS + JSON-RPC)").color(ST_OK).strong()); }
                        TlsStatus::ListeningNoMcp => { ui.label(RichText::new(match state.language { Language::PtBr => "TLS OK / MOTOR MUDO", Language::English => "TLS OK / ENGINE SILENT" }).color(ST_WARN).strong()); }
                        TlsStatus::Error => { ui.label(RichText::new(match state.language { Language::PtBr => "ERRO", Language::English => "ERROR" }).color(ST_ERR).strong()); }
                        TlsStatus::Stopped => { ui.label(RichText::new(match state.language { Language::PtBr => "DESLIGADO", Language::English => "OFF" }).color(ST_WARN).strong()); }
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
