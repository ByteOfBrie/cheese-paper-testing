// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod components;
mod ui;

use crate::ui::CheesePaperApp;

fn main() -> eframe::Result {
    env_logger::init();

    eframe::run_native(
        "Cheese Paper Rust Single File",
        Default::default(),
        Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc)))),
    )
}
