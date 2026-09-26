use crate::app::{term_button, AppState, Language, TERM_BG_PANEL, TERM_GREEN_BRIGHT};
use egui::{Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    // Ações de Janelas & Processos CLI.
    // A saída dos comandos aparece no console global do rodapé da janela.
    Frame::none()
        .fill(TERM_BG_PANEL)
        .rounding(Rounding::same(2.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Controle de Janelas & Injeção de Apps",
                    Language::English => "Window Control & App Launcher",
                })
                .size(16.0)
                .strong()
                .color(TERM_GREEN_BRIGHT)
            );

            ui.add_space(10.0);

            let list_btn = term_button(match state.language {
                Language::PtBr => "Listar Janelas Ativas (list_windows)",
                Language::English => "List Active Windows (list_windows)",
            })
            .min_size(Vec2::new(240.0, 34.0));

            if ui.add(list_btn).clicked() {
                state.refresh_windows();
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label(
                RichText::new(match state.language {
                    Language::PtBr => "Iniciar Aplicação Sem Foco (launch_app):",
                    Language::English => "Launch Background App (launch_app):",
                })
                .strong()
                .color(TERM_GREEN_BRIGHT)
            );

            ui.add_space(6.0);

            ui.add(egui::TextEdit::singleline(&mut state.launch_input).min_size(Vec2::new(220.0, 24.0)));

            ui.add_space(10.0);

            // Sem o glifo "▶": a fonte monoespaçada do tema nao o tem
            // (renderizava caixa quebrada).
            let launch_btn = term_button(match state.language {
                Language::PtBr => "Iniciar App (ex: notepad, chrome)",
                Language::English => "Launch App (e.g. notepad, chrome)",
            })
            .min_size(Vec2::new(220.0, 34.0));

            if ui.add(launch_btn).clicked() {
                state.launch_app();
            }
        });
}
