#![windows_subsystem = "windows"]

mod app;
mod tabs;
mod tray;

use app::FzComputerApp;
use eframe::egui;

/// Icone da JANELA (barra de titulo + taskbar do app em execucao).
/// E RGBA CRU 64x64 gerado junto com o installer/fzcomputerai.ico — nao e PNG
/// de proposito: decodificar PNG exigiria a feature `image` do eframe (nova
/// dependencia), e AGENTS.md pede para nao adicionar dependencia desnecessaria.
/// Regenerar ambos com scripts/make-icon.ps1.
///
/// Isto e SEPARADO do icone embutido no .exe (recurso Win32 aplicado pelo
/// build.rs a partir do .ico): aquele e o que o Explorer/busca/atalho mostram;
/// este e o que a janela e a taskbar mostram enquanto o app roda.
const ICON_RGBA_64: &[u8] = include_bytes!("../assets/icon64.rgba");

fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: ICON_RGBA_64.to_vec(),
        width: 64,
        height: 64,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // Título vindo de UMA fonte só: a thread da bandeja localiza esta
            // janela por título (FindWindowW) para restaurá-la. Divergir os dois
            // lados quebra o clique na bandeja em silêncio.
            .with_title(tray::MAIN_WINDOW_TITLE)
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([780.0, 560.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "FzComputerAI",
        options,
        Box::new(|_cc| Ok(Box::new(FzComputerApp::default()))),
    )
}
