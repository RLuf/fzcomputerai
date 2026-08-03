use crate::app::{
    status_dot, term_button, term_button_danger, AppState, Language, TunnelExposure, TunnelProvider,
    TunnelStatus, TERM_BG_PANEL, TERM_GRAY, TERM_GREEN, TERM_GREEN_BRIGHT, ST_ERR, ST_OK, ST_WARN,
};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Detecção de binários + leitura de config: LAZY, só na 1ª abertura.
    if !state.tunnel_bins_checked {
        state.read_tunnel_cfg();
        state.detect_tunnel_bins();
    }

    // Topo FIXO: aviso honesto + badge de exposição + botão Parar.
    render_top(ui, state);
    ui.add_space(8.0);

    // Meio ROLÁVEL: seletor, config do provedor, status/URL/snippet.
    // Sem log na aba: o meio recebe todo o espaço restante (a saída dos
    // comandos aparece no console global do rodapé da janela).
    egui::ScrollArea::vertical()
        .id_salt("tunnel_scroll")
        .max_height(ui.available_height())
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_provider_selector(ui, state);
            ui.add_space(8.0);
            render_provider_config(ui, state);
            ui.add_space(8.0);
            render_status_and_url(ui, state);
        });

    // Modais.
    render_start_modal(ui, state);
    render_ngrok_tos_modal(ui, state);
}

// ─── Topo fixo ──────────────────────────────────────────────────────────
fn render_top(ui: &mut Ui, state: &mut AppState) {
    // Frame VERMELHO de proposito: e aviso de seguranca (exposicao publica
    // de mouse/teclado/tela), nao decoracao — permanece vermelho no tema.
    Frame::none()
        .fill(Color32::from_rgb(60, 20, 20))
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                status_dot(ui, ST_ERR);
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Túnel: HTTPS público -> MCP HTTP local",
                        Language::English => "Tunnel: public HTTPS -> local MCP HTTP",
                    })
                    .size(15.0)
                    .strong()
                    .color(TERM_GREEN_BRIGHT),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let active = matches!(
                        state.tunnel_status,
                        TunnelStatus::Starting | TunnelStatus::Running
                    );
                    let stop_btn = term_button_danger(match state.language {
                        Language::PtBr => "Parar túnel",
                        Language::English => "Stop tunnel",
                    })
                    .min_size(Vec2::new(110.0, 28.0));
                    if ui.add_enabled(active, stop_btn).clicked() {
                        state.stop_tunnel();
                    }
                });
            });

            ui.add_space(6.0);
            // Aviso HONESTO e dependente do estado REAL: o motor antigo (<=0.8.x)
            // nao tem autenticacao nenhuma; o motor novo (0.16+) EXIGE token
            // Bearer. Afirmar "nao tem autenticacao" para quem roda o motor novo
            // seria mentira — e mentira no aviso de seguranca e pior que silencio.
            let has_token = !state.mcp_token.trim().is_empty();
            // Sem token, o texto depende da GERAÇÃO DO MOTOR — e essa distinção
            // não é detalhe: dizer "o endpoint aceita qualquer requisição" para
            // quem roda um motor 0.16+ é FALSO. MEDIDO no 0.17.0: sem
            // CUA_DRIVER_RS_MCP_HTTP_TOKEN o `serve` sai com erro
            // ("must be set to a host-generated bearer token") e NENHUMA porta
            // abre — não existe endpoint para expor. Inventar risco num aviso de
            // segurança gasta a confiança do usuário exatamente onde ela é
            // necessária.
            let engine_requires_token = {
                let v = state.driver_version.trim();
                match v.split('.').next().and_then(|s| s.parse::<u32>().ok()) {
                    Some(major) if major >= 1 => true,
                    Some(0) => v
                        .split('.')
                        .nth(1)
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|minor| minor >= 16)
                        .unwrap_or(false),
                    _ => false, // versão desconhecida: não afirma nada
                }
            };
            ui.label(
                RichText::new(if has_token {
                    match state.language {
                        Language::PtBr => "Há token configurado no motor (CUA_DRIVER_RS_MCP_HTTP_TOKEN): o endpoint exige 'Authorization: Bearer <token>' e responde 401 sem ele. Quem tiver a URL E o token controla esta máquina — trate o token como senha. URL aleatória NÃO é proteção.",
                        Language::English => "A token is configured in the engine (CUA_DRIVER_RS_MCP_HTTP_TOKEN): the endpoint requires 'Authorization: Bearer <token>' and answers 401 without it. Anyone with the URL AND the token controls this machine — treat the token as a password. A random URL is NOT protection.",
                    }
                } else if engine_requires_token {
                    match state.language {
                        Language::PtBr => "Sem token configurado: ESTE motor exige CUA_DRIVER_RS_MCP_HTTP_TOKEN e nem inicia o endpoint HTTP sem ele — não há porta aberta, logo não há o que expor pelo túnel. Para publicar, configure o token; ele passa a ser a senha de acesso.",
                        Language::English => "No token configured: THIS engine requires CUA_DRIVER_RS_MCP_HTTP_TOKEN and will not even start the HTTP endpoint without it — no port is open, so there is nothing to expose through the tunnel. To publish, configure the token; it becomes the access password.",
                    }
                } else {
                    match state.language {
                        Language::PtBr => "Sem token configurado: se este motor for de uma série antiga (0.8.x), o endpoint aceita qualquer requisição e quem tiver a URL pública controla mouse, teclado e tela desta máquina. Use senha na URL (abaixo) ou a autenticação do provedor. URL aleatória NÃO é proteção.",
                        Language::English => "No token configured: if this engine is from an older series (0.8.x), the endpoint accepts any request and anyone with the public URL controls this machine's mouse, keyboard and screen. Use a URL password (below) or the provider's authentication. A random URL is NOT protection.",
                    }
                })
                .size(11.0)
                .color(Color32::from_rgb(255, 205, 205)),
            );

            // Badge de exposição (só depois da sonda).
            if let Some(exp) = state.tunnel_exposure {
                ui.add_space(6.0);
                let (txt, color) = match exp {
                    TunnelExposure::Exposed => (
                        match state.language {
                            Language::PtBr => "EXPOSTO SEM AUTENTICAÇÃO (verificado agora)".to_string(),
                            Language::English => "EXPOSED WITH NO AUTHENTICATION (verified now)".to_string(),
                        },
                        ST_ERR,
                    ),
                    TunnelExposure::EdgeAuth(code) => (
                        match state.language {
                            Language::PtBr => format!("BORDA EXIGIU AUTENTICAÇÃO (verificado: HTTP {})", code),
                            Language::English => format!("EDGE REQUIRED AUTHENTICATION (verified: HTTP {})", code),
                        },
                        ST_OK,
                    ),
                    TunnelExposure::Unknown => (
                        match state.language {
                            Language::PtBr => "NÃO FOI POSSÍVEL VERIFICAR — trate como exposto".to_string(),
                            Language::English => "COULD NOT VERIFY — treat as exposed".to_string(),
                        },
                        ST_WARN,
                    ),
                };
                ui.horizontal_wrapped(|ui| {
                    status_dot(ui, color);
                    ui.label(RichText::new(txt).color(color).strong().size(12.0));
                });
            }
        });
}

// ─── Seletor de provedor + status do binário + botões ────────────────────
fn render_provider_selector(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.radio_value(
                    &mut state.tunnel_provider,
                    TunnelProvider::Ssh,
                    match state.language {
                        Language::PtBr => "SSH reverso",
                        Language::English => "Reverse SSH",
                    },
                );
                ui.radio_value(&mut state.tunnel_provider, TunnelProvider::Cloudflare, "Cloudflare");
                ui.radio_value(&mut state.tunnel_provider, TunnelProvider::Ngrok, "ngrok");
            });

            ui.add_space(4.0);

            // Caminho do binário do provedor selecionado.
            let (bin, dl_label, can_dl) = match state.tunnel_provider {
                TunnelProvider::Cloudflare => (
                    state.tunnel_cf_bin.clone(),
                    match state.language {
                        Language::PtBr => "Baixar cloudflared",
                        Language::English => "Download cloudflared",
                    },
                    true,
                ),
                TunnelProvider::Ngrok => (
                    state.tunnel_ngrok_bin.clone(),
                    match state.language {
                        Language::PtBr => "Baixar ngrok",
                        Language::English => "Download ngrok",
                    },
                    true,
                ),
                TunnelProvider::Ssh => (state.tunnel_ssh_bin.clone(), "", false),
            };
            ui.horizontal_wrapped(|ui| {
                if bin.is_empty() {
                    status_dot(ui, ST_ERR);
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "binário: NÃO ENCONTRADO",
                            Language::English => "binary: NOT FOUND",
                        })
                        .size(11.0)
                        .color(ST_ERR),
                    );
                } else {
                    status_dot(ui, ST_OK);
                    let prefix = match state.language {
                        Language::PtBr => "binário",
                        Language::English => "binary",
                    };
                    ui.label(RichText::new(format!("{}: {}", prefix, bin)).size(11.0).color(TERM_GREEN));
                }
            });

            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                let start_active = !matches!(
                    state.tunnel_status,
                    TunnelStatus::Starting | TunnelStatus::Running
                );
                let start_btn = term_button(match state.language {
                    Language::PtBr => "Iniciar túnel",
                    Language::English => "Start tunnel",
                })
                .min_size(Vec2::new(110.0, 30.0));
                if ui.add_enabled(start_active, start_btn).clicked() {
                    state.tunnel_show_start_modal = true;
                }

                let test_btn = term_button(match state.language {
                    Language::PtBr => "Testar pela internet",
                    Language::English => "Test over the internet",
                })
                .min_size(Vec2::new(150.0, 30.0));
                if ui.add(test_btn).clicked() {
                    state.verify_tunnel();
                }

                let detect_btn = term_button(match state.language {
                    Language::PtBr => "Detectar binários",
                    Language::English => "Detect binaries",
                })
                .min_size(Vec2::new(130.0, 30.0));
                if ui.add(detect_btn).clicked() {
                    state.detect_tunnel_bins();
                }

                if can_dl && bin.is_empty() {
                    let dl_btn = term_button(dl_label).min_size(Vec2::new(140.0, 30.0));
                    if ui.add(dl_btn).clicked() {
                        match state.tunnel_provider {
                            TunnelProvider::Cloudflare => state.download_cloudflared(),
                            TunnelProvider::Ngrok => state.tunnel_show_ngrok_tos = true,
                            TunnelProvider::Ssh => {}
                        }
                    }
                }
            });
        });
}

// ─── Config do provedor selecionado ──────────────────────────────────────
fn render_provider_config(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            match state.tunnel_provider {
                TunnelProvider::Cloudflare => render_cf_config(ui, state),
                TunnelProvider::Ngrok => render_ngrok_config(ui, state),
                TunnelProvider::Ssh => render_ssh_config(ui, state),
            }

            ui.add_space(8.0);
            let save_btn = term_button(match state.language {
                Language::PtBr => "Salvar configuração",
                Language::English => "Save configuration",
            })
            .min_size(Vec2::new(160.0, 28.0));
            if ui.add(save_btn).clicked() {
                state.save_tunnel_cfg();
            }
        });
}

fn render_cf_config(ui: &mut Ui, state: &mut AppState) {
    ui.label(
        RichText::new(match state.language {
            Language::PtBr => "Cloudflare Tunnel",
            Language::English => "Cloudflare Tunnel",
        })
        .size(14.0)
        .strong()
        .color(TERM_GREEN_BRIGHT),
    );
    let named = !state.tunnel_cf_token_file.trim().is_empty();
    ui.label(
        RichText::new(if named {
            match state.language {
                Language::PtBr => "Modo NOMEADO (token-file) — domínio fixo; proteja com Cloudflare Access.",
                Language::English => "NAMED mode (token-file) — fixed domain; protect with Cloudflare Access.",
            }
        } else {
            match state.language {
                Language::PtBr => "Modo QUICK (sem conta) — URL aleatória *.trycloudflare.com, sem autenticação.",
                Language::English => "QUICK mode (no account) — random *.trycloudflare.com URL, no authentication.",
            }
        })
        .size(11.0)
        .color(TERM_GRAY),
    );

    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        let login_btn = term_button(match state.language {
            Language::PtBr => "Login Cloudflare (OAuth)",
            Language::English => "Cloudflare Login (OAuth)",
        })
        .min_size(Vec2::new(180.0, 26.0));
        if ui.add(login_btn).clicked() {
            state.cloudflared_login();
        }
        ui.hyperlink_to(
            match state.language {
                Language::PtBr => "Painel Zero Trust",
                Language::English => "Zero Trust dashboard",
            },
            "https://one.dash.cloudflare.com/",
        );
    });

    ui.add_space(6.0);
    ui.label(match state.language {
        Language::PtBr => "Token do túnel (cole aqui p/ modo nomeado; guardado em arquivo, nunca em log):",
        Language::English => "Tunnel token (paste for named mode; stored in a file, never logged):",
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.tunnel_cf_token_input)
                .password(true)
                .min_size(Vec2::new(240.0, 24.0)),
        );
        if ui
            .button(match state.language {
                Language::PtBr => "Salvar token",
                Language::English => "Save token",
            })
            .clicked()
        {
            state.save_cf_token();
        }
        if named
            && ui
                .button(match state.language {
                    Language::PtBr => "Esquecer token",
                    Language::English => "Forget token",
                })
                .clicked()
        {
            state.forget_cf_token();
        }
    });
}

fn render_ngrok_config(ui: &mut Ui, state: &mut AppState) {
    ui.label(
        RichText::new("ngrok").size(14.0).strong().color(TERM_GREEN_BRIGHT),
    );
    ui.label(
        RichText::new(match state.language {
            Language::PtBr => "Requer conta + authtoken (no SEU terminal): ngrok config add-authtoken <TOKEN>",
            Language::English => "Requires an account + authtoken (in YOUR terminal): ngrok config add-authtoken <TOKEN>",
        })
        .size(11.0)
        .color(TERM_GRAY),
    );
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(match state.language {
                Language::PtBr => "Copiar comando authtoken",
                Language::English => "Copy authtoken command",
            })
            .clicked()
        {
            ui.output_mut(|o| o.copied_text = "ngrok config add-authtoken <TOKEN>".to_string());
        }
        ui.hyperlink_to("ngrok.com/download", "https://ngrok.com/download");
    });

    ui.add_space(6.0);
    ui.checkbox(
        &mut state.tunnel_ngrok_use_policy,
        match state.language {
            Language::PtBr => "Proteger com basic-auth (traffic policy gerada)",
            Language::English => "Protect with basic-auth (generated traffic policy)",
        },
    );
    if state.tunnel_ngrok_use_policy && !state.tunnel_ngrok_password.is_empty() {
        let cred = format!("fz:{}", state.tunnel_ngrok_password);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Credencial basic-auth (guarde; o cliente MCP precisa enviá-la):",
                    Language::English => "basic-auth credential (save it; the MCP client must send it):",
                })
                .size(11.0)
                .color(ST_WARN),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.code(&cred);
            if ui
                .button(match state.language {
                    Language::PtBr => "Copiar",
                    Language::English => "Copy",
                })
                .clicked()
            {
                ui.output_mut(|o| o.copied_text = cred.clone());
            }
        });
    }

    ui.add_space(4.0);
    ui.label(match state.language {
        Language::PtBr => "Argumentos extras (opcional):",
        Language::English => "Extra arguments (optional):",
    });
    ui.add(
        egui::TextEdit::singleline(&mut state.tunnel_ngrok_extra).min_size(Vec2::new(320.0, 24.0)),
    );

    ui.add_space(4.0);
    if ui
        .button(match state.language {
            Language::PtBr => "Descobrir URL (API local 4040)",
            Language::English => "Discover URL (local 4040 API)",
        })
        .clicked()
    {
        state.ngrok_query_local_api();
    }
}

fn render_ssh_config(ui: &mut Ui, state: &mut AppState) {
    ui.label(
        RichText::new(match state.language {
            Language::PtBr => "SSH reverso",
            Language::English => "Reverse SSH",
        })
        .size(14.0)
        .strong()
        .color(TERM_GREEN_BRIGHT),
    );
    ui.label(
        RichText::new(match state.language {
            Language::PtBr => "Servidor próprio (mais seguro) OU serviço público (sem garantia de uptime). BatchMode=yes exige chave.",
            Language::English => "Your own server (most secure) OR a public service (no uptime guarantee). BatchMode=yes requires a key.",
        })
        .size(11.0)
        .color(TERM_GRAY),
    );

    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("localhost.run").clicked() {
            state.tunnel_ssh_target = "nokey@localhost.run".to_string();
            state.tunnel_ssh_remote_port = "80".to_string();
        }
        if ui.button("serveo.net").clicked() {
            state.tunnel_ssh_target = "serveo.net".to_string();
            state.tunnel_ssh_remote_port = "80".to_string();
        }
    });

    ui.add_space(6.0);
    egui::Grid::new("ssh_cfg_grid")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label(match state.language {
                Language::PtBr => "Destino (usuário@host):",
                Language::English => "Target (user@host):",
            });
            ui.add(egui::TextEdit::singleline(&mut state.tunnel_ssh_target).min_size(Vec2::new(240.0, 24.0)));
            ui.end_row();

            ui.label(match state.language {
                Language::PtBr => "Porta remota:",
                Language::English => "Remote port:",
            });
            ui.add(egui::TextEdit::singleline(&mut state.tunnel_ssh_remote_port).min_size(Vec2::new(80.0, 24.0)));
            ui.end_row();

            ui.label(match state.language {
                Language::PtBr => "Chave (-i, opcional):",
                Language::English => "Key (-i, optional):",
            });
            ui.add(egui::TextEdit::singleline(&mut state.tunnel_ssh_key).min_size(Vec2::new(240.0, 24.0)));
            ui.end_row();

            ui.label(match state.language {
                Language::PtBr => "Args extras:",
                Language::English => "Extra args:",
            });
            ui.add(egui::TextEdit::singleline(&mut state.tunnel_ssh_extra).min_size(Vec2::new(240.0, 24.0)));
            ui.end_row();
        });
}

// ─── Status + URL + snippet ───────────────────────────────────────────────
fn render_status_and_url(ui: &mut Ui, state: &mut AppState) {
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            let (txt, color) = match state.tunnel_status {
                TunnelStatus::Stopped => (
                    match state.language {
                        Language::PtBr => "PARADO",
                        Language::English => "STOPPED",
                    },
                    ST_ERR,
                ),
                TunnelStatus::Starting => (
                    match state.language {
                        Language::PtBr => "INICIANDO (processo vivo, URL ainda não publicada)",
                        Language::English => "STARTING (process alive, URL not published yet)",
                    },
                    ST_WARN,
                ),
                TunnelStatus::Running => (
                    match state.language {
                        Language::PtBr => "ATIVO (URL pública publicada)",
                        Language::English => "ACTIVE (public URL published)",
                    },
                    ST_OK,
                ),
                TunnelStatus::Error => (
                    match state.language {
                        Language::PtBr => "ERRO (veja o log abaixo)",
                        Language::English => "ERROR (see the log below)",
                    },
                    ST_ERR,
                ),
            };
            ui.horizontal_wrapped(|ui| {
                status_dot(ui, color);
                ui.label(RichText::new(txt).color(color).strong().size(13.0));
            });

            ui.add_space(6.0);
            ui.label(match state.language {
                Language::PtBr => "URL pública (preenchida automaticamente; informe à mão no túnel nomeado):",
                Language::English => "Public URL (auto-filled; enter manually for the named tunnel):",
            });
            ui.add(
                egui::TextEdit::singleline(&mut state.tunnel_public_url).min_size(Vec2::new(320.0, 24.0)),
            );

            let full = state.tunnel_full_url();
            if !full.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.code(&full);
                    if ui
                        .button(match state.language {
                            Language::PtBr => "Copiar URL",
                            Language::English => "Copy URL",
                        })
                        .clicked()
                    {
                        let f = full.clone();
                        ui.output_mut(|o| o.copied_text = f);
                    }
                });

                let snippet = state.tunnel_mcp_snippet();
                ui.add_space(6.0);
                ui.label(match state.language {
                    Language::PtBr => "Snippet para o cliente MCP:",
                    Language::English => "Snippet for the MCP client:",
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(match state.language {
                            Language::PtBr => "Copiar snippet",
                            Language::English => "Copy snippet",
                        })
                        .clicked()
                    {
                        let s = snippet.clone();
                        ui.output_mut(|o| o.copied_text = s);
                    }
                });
                ui.monospace(&snippet);

                if state.tunnel_provider == TunnelProvider::Ngrok {
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Clientes MCP não são navegadores — o interstitial do ngrok não afeta; se afetar, envie o header ngrok-skip-browser-warning: 1.",
                            Language::English => "MCP clients are not browsers — the ngrok interstitial does not apply; if it does, send the header ngrok-skip-browser-warning: 1.",
                        })
                        .size(10.0)
                        .color(TERM_GRAY),
                    );
                }
            }
        });
}

// ─── Modal de início (senha na URL) ───────────────────────────────────────
fn render_start_modal(ui: &mut Ui, state: &mut AppState) {
    if !state.tunnel_show_start_modal {
        return;
    }
    let lang = state.language;
    let mut do_start = false;
    let mut do_cancel = false;
    egui::Window::new(match lang {
        Language::PtBr => "Iniciar túnel",
        Language::English => "Start tunnel",
    })
    .collapsible(false)
    .resizable(false)
    .order(egui::Order::Foreground)
    .pivot(egui::Align2::CENTER_CENTER)
    .default_pos(ui.ctx().screen_rect().center())
    .show(ui.ctx(), |ui| {
        ui.label(match lang {
            Language::PtBr => "Senha na URL (opcional). Com senha, a URL vira https://.../s/<senha>/mcp e um porteiro local só deixa passar quem a tiver. Vazio = túnel direto (sem senha).",
            Language::English => "URL password (optional). With a password the URL becomes https://.../s/<password>/mcp and a local gate only lets through who has it. Empty = direct tunnel (no password).",
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.tunnel_gate_password)
                    .min_size(Vec2::new(220.0, 24.0)),
            );
            if ui
                .button(match lang {
                    Language::PtBr => "Gerar",
                    Language::English => "Generate",
                })
                .clicked()
            {
                state.tunnel_generate_password();
            }
        });
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(match lang {
                    Language::PtBr => "Iniciar",
                    Language::English => "Start",
                })
                .clicked()
            {
                do_start = true;
            }
            if ui
                .button(match lang {
                    Language::PtBr => "Cancelar",
                    Language::English => "Cancel",
                })
                .clicked()
            {
                do_cancel = true;
            }
        });
    });

    if do_start {
        state.tunnel_show_start_modal = false;
        state.start_tunnel();
    }
    if do_cancel {
        state.tunnel_show_start_modal = false;
    }
}

// ─── Modal de termos do ngrok (antes de baixar) ───────────────────────────
fn render_ngrok_tos_modal(ui: &mut Ui, state: &mut AppState) {
    if !state.tunnel_show_ngrok_tos {
        return;
    }
    let lang = state.language;
    let mut do_accept = false;
    let mut do_cancel = false;
    egui::Window::new(match lang {
        Language::PtBr => "Termos do ngrok",
        Language::English => "ngrok Terms",
    })
    .collapsible(false)
    .resizable(false)
    .order(egui::Order::Foreground)
    .pivot(egui::Align2::CENTER_CENTER)
    .default_pos(ui.ctx().screen_rect().center())
    .show(ui.ctx(), |ui| {
        ui.label(match lang {
            Language::PtBr => "O ngrok é um binário proprietário da ngrok Inc. O download é da fonte oficial. Exige conta e authtoken próprios. O plano gratuito tem limites (ex.: 20.000 requisições/mês, 1 GB de tráfego).",
            Language::English => "ngrok is a proprietary binary by ngrok Inc. The download is from the official source. It requires your own account and authtoken. The free plan has limits (e.g. 20,000 requests/month, 1 GB of traffic).",
        });
        ui.add_space(6.0);
        ui.hyperlink_to(
            match lang {
                Language::PtBr => "Abrir Termos de Serviço",
                Language::English => "Open Terms of Service",
            },
            "https://ngrok.com/tos",
        );
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(match lang {
                    Language::PtBr => "Aceito — baixar",
                    Language::English => "Accept — download",
                })
                .clicked()
            {
                do_accept = true;
            }
            if ui
                .button(match lang {
                    Language::PtBr => "Cancelar",
                    Language::English => "Cancel",
                })
                .clicked()
            {
                do_cancel = true;
            }
        });
    });

    if do_accept {
        state.tunnel_show_ngrok_tos = false;
        state.log_debug("[tunnel] Termos do ngrok aceitos pelo usuario.");
        state.save_ngrok_tos_accepted();
        state.download_ngrok();
    }
    if do_cancel {
        state.tunnel_show_ngrok_tos = false;
    }
}
