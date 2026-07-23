use crate::app::{AppState, Language};
use egui::{Color32, Ui, ProgressBar};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.heading(match state.language {
        Language::PtBr => "🖥️ Controle do Motor cua-driver & Diagnóstico",
        Language::English => "🖥️ cua-driver Engine Control & Diagnostics",
    });

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.label(match state.language {
            Language::PtBr => "Estado do Serviço cua-driver Daemon:",
            Language::English => "cua-driver Daemon Service State:",
        });

        ui.horizontal(|ui| {
            if state.daemon_running {
                ui.colored_label(Color32::GREEN, "● EM EXECUÇÃO / RUNNING");
            } else {
                ui.colored_label(Color32::RED, "● PARADO / STOPPED");
            }
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button(match state.language {
                Language::PtBr => "▶️ Iniciar Daemon",
                Language::English => "▶️ Start Daemon",
            }).clicked() {
                state.start_daemon();
            }

            if ui.button(match state.language {
                Language::PtBr => "⏹️ Parar Daemon",
                Language::English => "⏹️ Stop Daemon",
            }).clicked() {
                state.stop_daemon();
            }

            if ui.button(match state.language {
                Language::PtBr => "🔄 Reiniciar Autostart",
                Language::English => "🔄 Kick Autostart",
            }).clicked() {
                state.kick_autostart();
            }
        });
    });

    ui.add_space(15.0);

    ui.group(|ui| {
        ui.label(match state.language {
            Language::PtBr => "Diagnóstico de Saúde do Sistema (Doctor):",
            Language::English => "System Health Diagnostics (Doctor):",
        });

        if ui.button(match state.language {
            Language::PtBr => "🩺 Executar cua-driver doctor",
            Language::English => "🩺 Run cua-driver doctor",
        }).clicked() {
            state.run_doctor();
        }

        ui.add_space(8.0);

        if state.is_downloading {
            ui.label(match state.language {
                Language::PtBr => "Baixando/Atualizando componentes...",
                Language::English => "Downloading/Updating components...",
            });
            ui.add(ProgressBar::new(state.download_progress).show_percentage());
        }

        ui.add_space(5.0);
        ui.label(match state.language {
            Language::PtBr => "Relatório de Saúde:",
            Language::English => "Health Report Output:",
        });

        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                ui.monospace(&state.doctor_output);
            });
    });
}
