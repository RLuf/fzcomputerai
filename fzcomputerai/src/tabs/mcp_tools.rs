use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

/// Definição de uma tool MCP exposta pelo CUA Driver.
struct ToolEntry {
    name: &'static str,
    desc_pt: &'static str,
    desc_en: &'static str,
}

struct ToolCategory {
    name_pt: &'static str,
    name_en: &'static str,
    color: Color32,
    tools: &'static [ToolEntry],
}

const CATEGORIES: &[ToolCategory] = &[
    ToolCategory {
        name_pt: "Visão & Screenshot",
        name_en: "Vision & Screenshot",
        color: Color32::from_rgb(33, 150, 243),
        tools: &[
            ToolEntry { name: "screenshot", desc_pt: "Captura imagem base64 da tela", desc_en: "Capture base64 screen image" },
            ToolEntry { name: "get_screen_size", desc_pt: "Resolução e DPI da tela", desc_en: "Screen resolution and DPI" },
            ToolEntry { name: "get_cursor_position", desc_pt: "Posição atual do cursor", desc_en: "Current cursor position" },
            ToolEntry { name: "get_desktop_state", desc_pt: "Estado completo do desktop + janelas", desc_en: "Full desktop state + windows" },
        ],
    },
    ToolCategory {
        name_pt: "Mouse & Cursor",
        name_en: "Mouse & Cursor",
        color: Color32::from_rgb(76, 175, 80),
        tools: &[
            ToolEntry { name: "click", desc_pt: "Clique esquerdo em (x, y)", desc_en: "Left click at (x, y)" },
            ToolEntry { name: "double_click", desc_pt: "Duplo clique em (x, y)", desc_en: "Double click at (x, y)" },
            ToolEntry { name: "right_click", desc_pt: "Clique direito em (x, y)", desc_en: "Right click at (x, y)" },
            ToolEntry { name: "drag", desc_pt: "Arrastar de (x1,y1) para (x2,y2)", desc_en: "Drag from (x1,y1) to (x2,y2)" },
            ToolEntry { name: "move_cursor", desc_pt: "Mover ponteiro para (x, y)", desc_en: "Move pointer to (x, y)" },
            ToolEntry { name: "scroll", desc_pt: "Rolar tela (up/down/left/right)", desc_en: "Scroll screen (up/down/left/right)" },
        ],
    },
    ToolCategory {
        name_pt: "Teclado & Digitação",
        name_en: "Keyboard & Typing",
        color: Color32::from_rgb(255, 167, 38),
        tools: &[
            ToolEntry { name: "type_text", desc_pt: "Digitar texto via input simulation", desc_en: "Type text via input simulation" },
            ToolEntry { name: "type_text_chars", desc_pt: "Digitar char a char (lento, preciso)", desc_en: "Type char by char (slow, precise)" },
            ToolEntry { name: "press_key", desc_pt: "Pressionar tecla (enter, tab, esc...)", desc_en: "Press key (enter, tab, esc...)" },
            ToolEntry { name: "hotkey", desc_pt: "Atalho de teclado (ctrl+c, alt+f4...)", desc_en: "Keyboard shortcut (ctrl+c, alt+f4...)" },
            ToolEntry { name: "set_value", desc_pt: "Definir valor em controle UIA", desc_en: "Set value on UIA control" },
        ],
    },
    ToolCategory {
        name_pt: "Janelas & Aplicações",
        name_en: "Windows & Applications",
        color: Color32::from_rgb(156, 39, 176),
        tools: &[
            ToolEntry { name: "list_apps", desc_pt: "Listar aplicações instaladas", desc_en: "List installed applications" },
            ToolEntry { name: "list_windows", desc_pt: "Listar janelas ativas do sistema", desc_en: "List active system windows" },
            ToolEntry { name: "get_window_state", desc_pt: "Estado de uma janela específica", desc_en: "State of a specific window" },
            ToolEntry { name: "launch_app", desc_pt: "Iniciar aplicação", desc_en: "Launch application" },
            ToolEntry { name: "kill_app", desc_pt: "Encerrar aplicação", desc_en: "Kill application" },
            ToolEntry { name: "zoom", desc_pt: "Zoom em janela (coords relativas)", desc_en: "Zoom to window (relative coords)" },
        ],
    },
    ToolCategory {
        name_pt: "Acessibilidade & UIA",
        name_en: "Accessibility & UIA",
        color: Color32::from_rgb(0, 188, 212),
        tools: &[
            ToolEntry { name: "get_accessibility_tree", desc_pt: "Árvore de acessibilidade (UIA tokens)", desc_en: "Accessibility tree (UIA tokens)" },
            ToolEntry { name: "check_permissions", desc_pt: "Verificar permissões do sistema", desc_en: "Check system permissions" },
        ],
    },
    ToolCategory {
        name_pt: "Cursor do Agente",
        name_en: "Agent Cursor",
        color: Color32::from_rgb(121, 134, 203),
        tools: &[
            ToolEntry { name: "set_agent_cursor_enabled", desc_pt: "Ativar/desativar cursor do agente", desc_en: "Enable/disable agent cursor" },
            ToolEntry { name: "set_agent_cursor_motion", desc_pt: "Animação de movimento do cursor", desc_en: "Cursor motion animation" },
            ToolEntry { name: "set_agent_cursor_style", desc_pt: "Estilo visual do cursor", desc_en: "Cursor visual style" },
            ToolEntry { name: "get_agent_cursor_state", desc_pt: "Estado atual do cursor agente", desc_en: "Current agent cursor state" },
        ],
    },
    ToolCategory {
        name_pt: "Gravação & Replay",
        name_en: "Recording & Replay",
        color: Color32::from_rgb(239, 83, 80),
        tools: &[
            ToolEntry { name: "start_recording", desc_pt: "Iniciar gravação de sessão", desc_en: "Start session recording" },
            ToolEntry { name: "stop_recording", desc_pt: "Parar gravação e salvar", desc_en: "Stop recording and save" },
            ToolEntry { name: "get_recording", desc_pt: "Obter dados da gravação", desc_en: "Get recording data" },
            ToolEntry { name: "replay_recording", desc_pt: "Reproduzir gravação salva", desc_en: "Replay saved recording" },
            ToolEntry { name: "install_ffmpeg", desc_pt: "Instalar ffmpeg para vídeo", desc_en: "Install ffmpeg for video" },
        ],
    },
    ToolCategory {
        name_pt: "Sessão & Configuração",
        name_en: "Session & Configuration",
        color: Color32::from_rgb(84, 110, 122),
        tools: &[
            ToolEntry { name: "session_start", desc_pt: "Iniciar sessão MCP isolada", desc_en: "Start isolated MCP session" },
            ToolEntry { name: "session_end", desc_pt: "Encerrar sessão MCP", desc_en: "End MCP session" },
            ToolEntry { name: "get_config", desc_pt: "Ler configuração atual", desc_en: "Read current configuration" },
            ToolEntry { name: "set_config", desc_pt: "Definir configuração", desc_en: "Set configuration" },
            ToolEntry { name: "health_report", desc_pt: "Relatório de saúde do sistema", desc_en: "System health report" },
            ToolEntry { name: "check_for_update", desc_pt: "Verificar atualizações", desc_en: "Check for updates" },
        ],
    },
];

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Header com info MCP
    Frame::none()
        .fill(Color32::from_rgb(24, 32, 42))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Catálogo de Ferramentas MCP — CUA Driver (Computer Use Agent)",
                        Language::English => "MCP Tools Catalog — CUA Driver (Computer Use Agent)",
                    })
                    .size(15.0)
                    .strong()
                    .color(Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let total: usize = CATEGORIES.iter().map(|c| c.tools.len()).sum();
                    ui.label(
                        RichText::new(format!(
                            "{} tools | {} {}",
                            total,
                            CATEGORIES.len(),
                            match state.language {
                                Language::PtBr => "categorias",
                                Language::English => "categories",
                            }
                        ))
                        .size(12.0)
                        .color(Color32::from_rgb(76, 175, 80)),
                    );
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Filtro:",
                        Language::English => "Filter:",
                    })
                    .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.mcp_tools_filter)
                        .min_size(Vec2::new(200.0, 22.0))
                        .hint_text(match state.language {
                            Language::PtBr => "Buscar ferramenta...",
                            Language::English => "Search tool...",
                        }),
                );
            });
        });

    ui.add_space(8.0);

    // Alturas ADAPTATIVAS: derivadas do espaço restante da janela em vez de
    // valores cravados (350/200px), para a aba aproveitar janelas maiores e
    // não estourar as menores.
    let body_h = ui.available_height();
    let catalog_h = (body_h - 16.0).max(220.0);
    let output_h = (body_h - 300.0).max(160.0);

    // Layout: Tools à esquerda, Output à direita
    ui.columns(2, |cols| {
        // Coluna 1: Catálogo de tools por categoria
        egui::ScrollArea::vertical()
            .id_salt("mcp_tools_catalog")
            .max_height(catalog_h)
            .auto_shrink([false, false])
            .show(&mut cols[0], |ui| {
                let filter = state.mcp_tools_filter.to_lowercase();
                let mut tool_clicked: Option<String> = None;

                for cat in CATEGORIES {
                    let cat_name = match state.language {
                        Language::PtBr => cat.name_pt,
                        Language::English => cat.name_en,
                    };

                    // Filtrar: se há filtro, mostrar só tools que batem
                    let matching: Vec<&ToolEntry> = if filter.is_empty() {
                        cat.tools.iter().collect()
                    } else {
                        cat.tools
                            .iter()
                            .filter(|t| {
                                t.name.contains(&filter)
                                    || t.desc_pt.to_lowercase().contains(&filter)
                                    || t.desc_en.to_lowercase().contains(&filter)
                            })
                            .collect()
                    };
                    if matching.is_empty() {
                        continue;
                    }

                    Frame::none()
                        .fill(Color32::from_rgb(35, 35, 35))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Color indicator bar
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(4.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, cat.color);
                                ui.label(
                                    RichText::new(cat_name)
                                        .size(13.0)
                                        .strong()
                                        .color(cat.color),
                                );
                            });

                            ui.add_space(4.0);

                            for tool in matching {
                                ui.horizontal(|ui| {
                                    let desc = match state.language {
                                        Language::PtBr => tool.desc_pt,
                                        Language::English => tool.desc_en,
                                    };

                                    let btn = egui::Button::new(
                                        RichText::new(tool.name)
                                            .size(12.0)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(50, 50, 50))
                                    .min_size(Vec2::new(160.0, 24.0))
                                    .rounding(Rounding::same(4.0));

                                    if ui.add(btn).clicked() {
                                        tool_clicked = Some(tool.name.to_string());
                                    }

                                    ui.label(
                                        RichText::new(desc)
                                            .size(11.0)
                                            .color(Color32::from_rgb(160, 160, 160)),
                                    );
                                });
                            }
                        });
                    ui.add_space(4.0);
                }

                if let Some(name) = tool_clicked {
                    // Tools sem parametros podem ser testadas diretamente
                    let no_param_tools = [
                        "screenshot",
                        "get_screen_size",
                        "get_cursor_position",
                        "get_desktop_state",
                        "list_apps",
                        "list_windows",
                        "get_agent_cursor_state",
                        "get_config",
                        "health_report",
                        "check_for_update",
                        "check_permissions",
                        "get_recording",
                    ];
                    if no_param_tools.contains(&name.as_str()) {
                        state.call_mcp_tool(&name, &[]);
                    } else {
                        state.mcp_tools_output = format!(
                            "[{}] {} '{}' {}",
                            name,
                            match state.language {
                                Language::PtBr => "Esta tool requer parâmetros. Use via MCP JSON-RPC ou CLI:\n\ncua-driver call",
                                Language::English => "This tool requires parameters. Use via MCP JSON-RPC or CLI:\n\ncua-driver call",
                            },
                            name,
                            match state.language {
                                Language::PtBr => "--help\n\nOu envie via POST http://<ip>:8000/mcp com JSON-RPC.",
                                Language::English => "--help\n\nOr send via POST http://<ip>:8000/mcp with JSON-RPC.",
                            }
                        );
                    }
                }
            });

        // Coluna 2: Output da tool + referência rápida
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(12.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Resultado da Tool & Referência MCP",
                        Language::English => "Tool Output & MCP Reference",
                    })
                    .size(14.0)
                    .strong()
                    .color(Color32::WHITE),
                );

                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .id_salt("mcp_tools_output")
                    .max_height(output_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if state.mcp_tools_output.is_empty() {
                            ui.monospace(match state.language {
                                Language::PtBr => "Clique em uma tool para testá-la.\nTools sem parâmetros são executadas diretamente.",
                                Language::English => "Click a tool to test it.\nParameterless tools run directly.",
                            });
                        } else {
                            ui.monospace(&state.mcp_tools_output);
                        }
                    });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Referência Rápida — Workflow CUA",
                        Language::English => "Quick Reference — CUA Workflow",
                    })
                    .size(13.0)
                    .strong()
                    .color(Color32::from_rgb(33, 150, 243)),
                );

                ui.add_space(6.0);

                ui.monospace(match state.language {
                    Language::PtBr => concat!(
                        "Ciclo: Olhar → Agir → Verificar\n",
                        "\n",
                        "1. screenshot           (captura tela)\n",
                        "2. click 450 280        (age na UI)\n",
                        "3. screenshot           (verifica resultado)\n",
                        "\n",
                        "Acesso via MCP JSON-RPC:\n",
                        "  POST http://<ip>:8000/mcp\n",
                        "  {\"jsonrpc\":\"2.0\",\"id\":1,\n",
                        "   \"method\":\"tools/call\",\n",
                        "   \"params\":{\"name\":\"screenshot\",\n",
                        "             \"arguments\":{}}}\n",
                        "\n",
                        "Acesso via CLI:\n",
                        "  cua-driver call screenshot\n",
                        "  cua-driver call click --x 450 --y 280\n",
                    ),
                    Language::English => concat!(
                        "Cycle: Look → Act → Verify\n",
                        "\n",
                        "1. screenshot           (capture screen)\n",
                        "2. click 450 280        (act on UI)\n",
                        "3. screenshot           (verify result)\n",
                        "\n",
                        "Access via MCP JSON-RPC:\n",
                        "  POST http://<ip>:8000/mcp\n",
                        "  {\"jsonrpc\":\"2.0\",\"id\":1,\n",
                        "   \"method\":\"tools/call\",\n",
                        "   \"params\":{\"name\":\"screenshot\",\n",
                        "             \"arguments\":{}}}\n",
                        "\n",
                        "Access via CLI:\n",
                        "  cua-driver call screenshot\n",
                        "  cua-driver call click --x 450 --y 280\n",
                    ),
                });
            });
    });
}
