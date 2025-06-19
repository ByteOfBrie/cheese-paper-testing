use egui::{FontFamily, FontId, TextStyle};

use crate::ui::BaseTextEditor;

pub struct CheesePaperApp {
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

fn configure_text_styles(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        *style.text_styles.get_mut(&TextStyle::Body).unwrap() =
            FontId::new(24.0, FontFamily::Proportional)
    });
}

impl CheesePaperApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_text_styles(&cc.egui_ctx);
        Self {
            ..Default::default()
        }
    }
}
