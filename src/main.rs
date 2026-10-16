#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bloat_killer;
mod config;
mod fans;
mod modes;
mod terminal_log;
mod theme;
mod vitals;

use app::KilJoyApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::write("kiljoy_crash.log", format!("PANIC: {:?}", info));
    }));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("⚡ KILJOY // HP VICTUS CYBER-DECK")
            .with_inner_size([1180.0, 740.0])
            .with_min_inner_size([960.0, 620.0])
            .with_active(true),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "kiljoy",
        native_options,
        Box::new(|cc| Ok(Box::new(KilJoyApp::new(cc)))),
    )
}
