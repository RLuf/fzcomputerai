#![windows_subsystem = "windows"]

mod app;
mod lifecycle;
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
    // ANTES de qualquer spawn: cria o Job Object que faz TODO processo filho
    // (motor, túnel) morrer junto com esta GUI — inclusive em taskkill /F e
    // crash, porque a garantia é do kernel. Ver lifecycle.rs. O resultado é
    // registrado no Console Debug pelo AppState (status honesto: se o job não
    // subiu, a UI diz que a limpeza automática não está garantida).
    let job_ok = lifecycle::init();

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
        Box::new(move |_cc| {
            let mut app = FzComputerApp::default();
            // Status honesto sobre a garantia de limpeza: se o job NÃO subiu,
            // "tudo morre com o app" deixa de ser garantido pelo kernel e o
            // usuário precisa saber — calar seria vender uma garantia que não
            // existe.
            if job_ok {
                app.state.log_debug(
                    "[lifecycle] Job Object ativo: motor e tuneis iniciados por esta GUI morrem junto com ela (garantia do kernel, vale ate para taskkill /F).",
                );
            } else {
                app.state.log_debug(
                    "[lifecycle] AVISO: NAO foi possivel criar o Job Object. A limpeza automatica ao fechar NAO esta garantida — os processos filhos contam apenas com o encerramento normal e com o watchdog do tunel.",
                );
            }
            Ok(Box::new(app))
        }),
    )
}
