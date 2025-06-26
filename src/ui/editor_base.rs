use egui::{FontFamily, FontId, TextStyle};

use crate::{
    components::file_objects::FileObject,
    ui::{BaseTextEditor, SceneTextEditor},
};

pub struct CheesePaperApp<'a> {
    pub editor: SceneTextEditor<'a>,
}

impl eframe::App for CheesePaperApp<'_> {
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

impl<'a> CheesePaperApp<'a> {
    pub fn new(cc: &eframe::CreationContext<'_>, file_object: &'a mut FileObject) -> Self {
        configure_text_styles(&cc.egui_ctx);
        Self {
            editor: SceneTextEditor { scene: file_object },
        }
    }
}
