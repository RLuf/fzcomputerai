use crate::app::{
    term_button, term_button_danger, AppState, Language, TERM_BG_PANEL, TERM_GREEN_BRIGHT,
};
use egui::{Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Coluna 1: Doctor Diagnósticos
        Frame::none()
            .fill(TERM_BG_PANEL)
            .rounding(Rounding::same(2.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Diagnóstico de Saúde (Doctor)",
                        Language::English => "System Health Doctor",
                    })
                    .size(16.0)
                    .strong()
                    .color(TERM_GREEN_BRIGHT)
                );

                ui.add_space(10.0);

                let doc_btn = term_button(match state.language {
                    Language::PtBr => "Executar Diagnóstico Doctor",
                    Language::English => "Run Doctor Diagnostics",
                })
                .min_size(Vec2::new(220.0, 34.0));

                if ui.add(doc_btn).clicked() {
                    state.run_doctor();
                }
            });

        // Coluna 2: Pacote de Skills CLI
        Frame::none()
            .fill(TERM_BG_PANEL)
            .rounding(Rounding::same(2.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Pacotes de Skills de Automação Visual",
                        Language::English => "Visual Automation Skill Packs",
                    })
                    .size(16.0)
                    .strong()
                    .color(TERM_GREEN_BRIGHT)
                );

                ui.add_space(8.0);
                ui.label(match state.language {
                    Language::PtBr => "Instala symlinks de skills nos agentes detectados (Claude Code, Antigravity, Cursor, Codex, OpenClaw).",
                    Language::English => "Installs versioned skill pack symlinks into detected agents (Claude Code, Antigravity, Cursor, Codex, OpenClaw).",
                });

                ui.add_space(12.0);

                let install_btn = term_button(match state.language {
                    Language::PtBr => "Instalar Skills nos Agentes",
                    Language::English => "Install Skills to Agents",
                })
                .min_size(Vec2::new(200.0, 32.0));

                if ui.add(install_btn).clicked() {
                    state.install_skills();
                }

                ui.add_space(6.0);

                let update_btn = term_button(match state.language {
                    Language::PtBr => "Atualizar Pacote de Skills",
                    Language::English => "Update Skill Pack",
                })
                .min_size(Vec2::new(200.0, 32.0));

                if ui.add(update_btn).clicked() {
                    state.update_skills();
                }

                ui.add_space(6.0);

                let uninstall_btn = term_button_danger(match state.language {
                    Language::PtBr => "Remover Symlinks",
                    Language::English => "Uninstall Symlinks",
                })
                .min_size(Vec2::new(200.0, 32.0));

                if ui.add(uninstall_btn).clicked() {
                    state.uninstall_skills();
                }
            });
    });
}
