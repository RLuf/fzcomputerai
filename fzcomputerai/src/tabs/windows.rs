use crate::app::{AppState, Language};
use egui::{Color32, Frame, Margin, RichText, Rounding, Ui, Vec2};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Coluna 1: Ações de Janelas & Processos CLI
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Controle de Janelas & Injeção de Apps",
                        Language::English => "Window Control & App Launcher",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                let list_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "📋 Listar Janelas Ativas (list_windows)",
                        Language::English => "📋 List Active Windows (list_windows)",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(33, 150, 243))
                .min_size(Vec2::new(240.0, 34.0))
                .rounding(Rounding::same(6.0));

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
                    .color(Color32::WHITE)
                );

                ui.add_space(6.0);

                ui.add(egui::TextEdit::singleline(&mut state.launch_input).min_size(Vec2::new(220.0, 24.0)));

                ui.add_space(10.0);

                let launch_btn = egui::Button::new(
                    RichText::new(match state.language {
                        Language::PtBr => "🚀 Iniciar App (ex: notepad, chrome)",
                        Language::English => "🚀 Launch App (e.g. notepad, chrome)",
                    })
                    .color(Color32::WHITE)
                    .strong()
                )
                .fill(Color32::from_rgb(76, 175, 80))
                .min_size(Vec2::new(220.0, 34.0))
                .rounding(Rounding::same(6.0));

                if ui.add(launch_btn).clicked() {
                    state.launch_app();
                }
            });

        // Coluna 2: Lista de Janelas & Inspector CLI
        Frame::none()
            .fill(Color32::from_rgb(38, 38, 38))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(match state.language {
                        Language::PtBr => "Janelas do Sistema & Inspeção UIA",
                        Language::English => "System Windows & UIA Inspection",
                    })
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE)
                );

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.windows_log);
                    });
            });
    });
}
