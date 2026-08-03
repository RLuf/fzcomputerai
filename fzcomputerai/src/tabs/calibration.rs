use crate::app::{term_button, AppState, Language, TERM_BG_PANEL, TERM_GREEN_BRIGHT};
use egui::{Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Calibração de DPI e Mapeamento de Tela.
    // A saída dos comandos aparece no console global do rodapé da janela.
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Calibração de Tela & DPI Scaling",
                    Language::English => "Screen Calibration & DPI Scaling",
                })
                .size(16.0)
                .strong()
                .color(TERM_GREEN_BRIGHT)
            );

            ui.add_space(10.0);

            ui.label(match state.language {
                Language::PtBr => "Ajuste de coordenadas físicas vs pixels lógicos para modelos de visão computacional.",
                Language::English => "Physical coordinates vs logical pixels calibration for Computer Vision models.",
            });

            ui.add_space(12.0);

            let fetch_btn = term_button(match state.language {
                Language::PtBr => "Detectar Resolução & DPI (get_screen_size)",
                Language::English => "Detect Resolution & DPI (get_screen_size)",
            })
            .min_size(Vec2::new(240.0, 34.0));

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
                .color(TERM_GREEN_BRIGHT)
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

            let move_btn = term_button(match state.language {
                Language::PtBr => "Mover Ponteiro para (X, Y)",
                Language::English => "Move Pointer to (X, Y)",
            })
            .min_size(Vec2::new(220.0, 34.0));

            if ui.add(move_btn).clicked() {
                state.test_click_position();
            }
        });
}
