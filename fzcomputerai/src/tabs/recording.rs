use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Coluna 1: Controles de Gravação de Vídeo & Trajetória
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Gravação de Vídeo & Trajetória de Agente",
                        Language::English => "Video & Trajectory Recording",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Status:");
                    if state.is_recording {
                        ui.label(RichText::new("🔴 GRAVANDO SESSÃO...").color(Color32::from_rgb(239, 83, 80)).strong());
                    } else {
                        ui.label(RichText::new("⚪ Inativo").color(Color32::from_rgb(180, 180, 180)).strong());
                    }
                });

                ui.add_space(16.0);

                let rec_start_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🎥 Iniciar Gravação (start_recording)",
                        Language::English => "🎥 Start Recording (start_recording)",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(239, 83, 80)) // Red
                .min_size(Vec2::new(240.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(rec_start_btn).clicked() {
                    state.start_recording();
                }

                ui.add_space(8.0);

                let rec_stop_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "⏹️ Finalizar & Salvar Trajetória",
                        Language::English => "⏹️ Stop & Save Trajectory",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(84, 110, 122))
                .min_size(Vec2::new(240.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(rec_stop_btn).clicked() {
                    state.stop_recording();
                }
            });

        // Coluna 2: Status & Output
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Diretório & Log da Trajetória",
                        Language::English => "Directory & Trajectory Log",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.recording_log);
                    });
            });
    });
}
