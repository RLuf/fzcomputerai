use crate::app::{
    status_dot, term_button, term_button_danger, AppState, Language, TERM_BG_PANEL, TERM_GRAY,
    TERM_GREEN_BRIGHT, ST_ERR,
};
use egui::{Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Controles de Gravação de Vídeo & Trajetória.
    // A saída dos comandos aparece no console global do rodapé da janela.
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Gravação de Vídeo & Trajetória de Agente",
                    Language::English => "Video & Trajectory Recording",
                })
                .size(16.0)
                .strong()
                .color(TERM_GREEN_BRIGHT)
            );

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Status:");
                // Ponto DESENHADO (status_dot): a fonte do tema nao tem o
                // glifo "●" e renderizava uma caixa quebrada.
                if state.is_recording {
                    status_dot(ui, ST_ERR);
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "GRAVANDO SESSÃO...",
                            Language::English => "RECORDING SESSION...",
                        })
                        .color(ST_ERR)
                        .strong(),
                    );
                } else {
                    status_dot(ui, TERM_GRAY);
                    ui.label(
                        RichText::new(match state.language {
                            Language::PtBr => "Inativo",
                            Language::English => "Idle",
                        })
                        .color(TERM_GRAY)
                        .strong(),
                    );
                }
            });

            ui.add_space(16.0);

            let rec_start_btn = term_button(match state.language {
                Language::PtBr => "Iniciar Gravação (start_recording)",
                Language::English => "Start Recording (start_recording)",
            })
            .min_size(Vec2::new(240.0, 34.0));

            if ui.add(rec_start_btn).clicked() {
                state.start_recording();
            }

            ui.add_space(8.0);

            let rec_stop_btn = term_button_danger(match state.language {
                Language::PtBr => "Finalizar & Salvar Trajetória",
                Language::English => "Stop & Save Trajectory",
            })
            .min_size(Vec2::new(240.0, 34.0));

            if ui.add(rec_stop_btn).clicked() {
                state.stop_recording();
            }
        });
}
