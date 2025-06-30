use egui::{FontFamily, FontId, TextStyle};

use crate::{
    components::file_objects::FileObject, components::file_objects::MutFileObjectTypeInterface,
    ui::SceneTextEditor,
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
    use FontFamily::{Monospace, Proportional};

    // TODO: when configs are read, scale all of these off of the configured font size
    let text_styles: std::collections::BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::new(28.0, Proportional)),
        (TextStyle::Body, FontId::new(24.0, Proportional)),
        (TextStyle::Monospace, FontId::new(24.0, Monospace)),
        (TextStyle::Button, FontId::new(20.0, Proportional)),
        (TextStyle::Small, FontId::new(20.0, Proportional)),
    ]
    .into();

    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}

impl<'a> CheesePaperApp<'a> {
    pub fn new(cc: &eframe::CreationContext<'_>, file_object: &'a mut Box<dyn FileObject>) -> Self {
        configure_text_styles(&cc.egui_ctx);

        match file_object.get_file_type_mut() {
            MutFileObjectTypeInterface::Scene(scene) => Self {
                editor: SceneTextEditor { scene: scene },
            },
            _ => panic!(),
        }
    }
}
