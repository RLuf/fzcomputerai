#![windows_subsystem = "windows"]

mod app;
mod tabs;

use app::FzComputerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FzComputerAI — Computer Vision MCP Manager")
            .with_inner_size([850.0, 600.0])
            .with_min_inner_size([700.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "FzComputerAI",
        options,
        Box::new(|_cc| Ok(Box::new(FzComputerApp::default()))),
    )
}
