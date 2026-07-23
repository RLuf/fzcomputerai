use crate::app::{AppState, Language};
use egui::Ui;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading(match state.language {
        Language::PtBr => "🧩 Gerenciador de Skills & Agentes de IA",
        Language::English => "🧩 AI Agent Skill Pack Manager",
    });

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.label(match state.language {
            Language::PtBr => "Pacotes de Skills de Automação Visual para Agentes:",
            Language::English => "Visual Automation Skill Packs for AI Agents:",
        });

        ui.add_space(5.0);
        ui.label(match state.language {
            Language::PtBr => "Instala symlinks de skills nos agentes detectados (Claude Code, Antigravity, Cursor, Codex, OpenClaw).",
            Language::English => "Installs versioned skill pack symlinks into detected agents (Claude Code, Antigravity, Cursor, Codex, OpenClaw).",
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button(match state.language {
                Language::PtBr => "📥 Instalar Skills nos Agentes",
                Language::English => "📥 Install Skills to Agents",
            }).clicked() {
                state.install_skills();
            }

            if ui.button(match state.language {
                Language::PtBr => "🔄 Atualizar Skills",
                Language::English => "🔄 Update Skills",
            }).clicked() {
                state.update_skills();
            }

            if ui.button(match state.language {
                Language::PtBr => "🗑️ Desinstalar Symlinks",
                Language::English => "🗑️ Uninstall Symlinks",
            }).clicked() {
                state.uninstall_skills();
            }
        });
    });

    ui.add_space(15.0);

    ui.group(|ui| {
        ui.label(match state.language {
            Language::PtBr => "Status do Pacote de Skills:",
            Language::English => "Skill Pack Status:",
        });

        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                ui.monospace(&state.skills_output);
            });
    });
}
