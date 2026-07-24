use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, ProgressBar, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Serviço Daemon cua-driver",
                        Language::English => "cua-driver Daemon Service",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Status:");
                    if state.daemon_running {
                        ui.label(RichText::new("● EM EXECUÇÃO / RUNNING").color(Color32::from_rgb(76, 175, 80)).strong());
                    } else {
                        ui.label(RichText::new("● PARADO / STOPPED").color(Color32::from_rgb(239, 83, 80)).strong());
                    }
                });

                ui.add_space(14.0);

                let start_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "▶️ Iniciar Serviço Daemon",
                        Language::English => "▶️ Start Daemon Service",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(76, 175, 80))
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(start_btn).clicked() {
                    state.start_daemon();
                }

                ui.add_space(6.0);

                let stop_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "⏹️ Parar Serviço",
                        Language::English => "⏹️ Stop Service",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(239, 83, 80))
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(stop_btn).clicked() {
                    state.stop_daemon();
                }

                ui.add_space(6.0);

                let kick_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🔄 Reiniciar Autostart Task",
                        Language::English => "🔄 Kick Autostart Task",
                    })
                    .color(Color32::WHITE)
                )
                .fill(Color32::from_rgb(84, 110, 122))
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(kick_btn).clicked() {
                    state.kick_autostart();
                }
            });

        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
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
                .min_size(Vec2::new(200.0, 32.0))
                .rounding(Rounding::same(6.0));

                if ui.add(doc_btn).clicked() {
                    state.run_doctor();
                }

                if state.is_downloading {
                    ui.add_space(6.0);
                    ui.add(ProgressBar::new(state.download_progress).show_percentage());
                }

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.doctor_output);
                    });
            });
    });
}
