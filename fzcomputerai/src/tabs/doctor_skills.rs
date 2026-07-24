use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Coluna 1: Doctor Diagnósticos
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Diagnóstico de Saúde (Doctor)",
                        Language::English => "System Health Doctor",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                let doc_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🩺 Executar Diagnóstico Doctor",
                        Language::English => "🩺 Run Doctor Diagnostics",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(33, 150, 243))
                .min_size(Vec2::new(220.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(doc_btn).clicked() {
                    state.run_doctor();
                }

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.doctor_output);
                    });
            });

        // Coluna 2: Pacote de Skills CLI
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Pacotes de Skills de Automação Visual",
                        Language::English => "Visual Automation Skill Packs",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(8.0);
                ui.label(match state.language {
                    Language::PtBr => "Instala symlinks de skills nos agentes detectados (Claude Code, Antigravity, Cursor, Codex, OpenClaw).",
                    Language::English => "Installs versioned skill pack symlinks into detected agents (Claude Code, Antigravity, Cursor, Codex, OpenClaw).",
                });

                ui.add_space(12.0);

                let install_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "📥 Instalar Skills nos Agentes",
                        Language::English => "📥 Install Skills to Agents",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(76, 175, 80))
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(install_btn).clicked() {
                    state.install_skills();
                }

                ui.add_space(6.0);

                let update_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🔄 Atualizar Pacote de Skills",
                        Language::English => "🔄 Update Skill Pack",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(84, 110, 122))
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(update_btn).clicked() {
                    state.update_skills();
                }

                ui.add_space(6.0);

                let uninstall_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🗑️ Remover Symlinks",
                        Language::English => "🗑️ Uninstall Symlinks",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(239, 83, 80))
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(uninstall_btn).clicked() {
                    state.uninstall_skills();
                }

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.skills_output);
                    });
            });
    });
}
