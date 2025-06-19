#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{FontFamily, FontId, TextStyle};

mod components;
mod tiny_markdown;
mod ui;

use crate::ui::BaseTextEditor;

struct CheesePaperApp {
    editor: BaseTextEditor,
}

impl Default for CheesePaperApp {
    fn default() -> Self {
        Self {
            editor: BaseTextEditor::default(),
        }
    }
}

impl eframe::App for CheesePaperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.panels(ctx);
    }
}

impl CheesePaperApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_text_styles(&cc.egui_ctx);
        Self {
            ..Default::default()
        }
    }
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Cheese Paper Rust",
        options,
        Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc)))),
    )
}

fn configure_text_styles(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        *style.text_styles.get_mut(&TextStyle::Body).unwrap() =
            FontId::new(24.0, FontFamily::Proportional)
    });
}
