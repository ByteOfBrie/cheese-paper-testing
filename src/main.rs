#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example
#![allow(unused_imports)]

use eframe::egui;
use egui::{FontFamily, FontId, RichText, TextStyle};
use egui::{
    Key, KeyboardShortcut, Modifiers, ScrollArea, TextBuffer, TextEdit, Ui, text::CCursorRange,
};

use std::collections::BTreeMap;
mod default_text;
mod tiny_markdown;
use crate::default_text::DEFAULT_TEXT;
mod components;

pub struct BaseTextEditor {
    text: String,

    highlighter: crate::tiny_markdown::MemoizedMarkdownHighlighter,
}

impl Default for BaseTextEditor {
    fn default() -> Self {
        Self {
            text: DEFAULT_TEXT.trim().to_owned(),
            highlighter: Default::default(),
        }
    }
}

impl BaseTextEditor {
    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("text")
            .show(ui, |ui| self.editor_ui(ui));
    }

    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        let BaseTextEditor { text, highlighter } = self;

        let mut layouter = |ui: &egui::Ui, tinymark: &str, wrap_width: f32| {
            let mut layout_job = highlighter.highlight(ui.style(), tinymark);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        ui.add(
            egui::TextEdit::multiline(text)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter),
        );
    }
}

struct CheesePaperApp {
    editor: BaseTextEditor,
}

impl Default for CheesePaperApp {
    fn default() -> Self {
        Self {
            editor: BaseTextEditor {
                ..Default::default()
            },
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
