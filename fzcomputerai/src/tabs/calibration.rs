use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Coluna 1: Calibração de DPI e Mapeamento de Tela
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Calibração de Tela & DPI Scaling",
                        Language::English => "Screen Calibration & DPI Scaling",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                ui.label(match state.language {
                    Language::PtBr => "Ajuste de coordenadas físicas vs pixels lógicos para modelos de visão computacional.",
                    Language::English => "Physical coordinates vs logical pixels calibration for Computer Vision models.",
                });

                ui.add_space(12.0);

                let fetch_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "📐 Detectar Resolução & DPI (get_screen_size)",
                        Language::English => "📐 Detect Resolution & DPI (get_screen_size)",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(33, 150, 243))
                .min_size(Vec2::new(240.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(fetch_btn).clicked() {
                    state.fetch_screen_info();
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(10.0);

                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Teste de Precisão do Ponteiro (move_cursor):",
                        Language::English => "Pointer Precision Test (move_cursor):",
                    })
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::TextEdit::singleline(&mut state.test_x).min_size(Vec2::new(80.0, 24.0)));
                    ui.add_space(10.0);
                    ui.label("Y:");
                    ui.add(egui::TextEdit::singleline(&mut state.test_y).min_size(Vec2::new(80.0, 24.0)));
                });

                ui.add_space(10.0);

                let move_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🎯 Mover Ponteiro para (X, Y)",
                        Language::English => "🎯 Move Pointer to (X, Y)",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(76, 175, 80))
                .min_size(Vec2::new(220.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(move_btn).clicked() {
                    state.test_click_position();
                }
            });

        // Coluna 2: Log de Calibração e Visão Computacional
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Resultado do Mapeamento & Log",
                        Language::English => "Mapping Output & Vision Log",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.calibration_log);
                    });
            });
    });
}
