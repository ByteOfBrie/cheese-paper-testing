#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod components;
mod tiny_markdown;
mod ui;

use crate::ui::CheesePaperApp;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Cheese Paper Rust",
        options,
        Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc)))),
    )
}
