use egui::{FontFamily, FontId, TextStyle};
use std::time::{Duration, SystemTime};

use crate::{
    components::file_objects::FileObject, components::file_objects::MutFileObjectTypeInterface,
    ui::SceneTextEditor,
};

pub struct CheesePaperApp<'a> {
    pub editor: SceneTextEditor<'a>,
    last_write: SystemTime,
}

impl eframe::App for CheesePaperApp<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.panels(ctx);
        let current_time = SystemTime::now();
        if current_time.duration_since(self.last_write).unwrap() > Duration::from_secs(5) {
            println!("Writing at: {:#?}", current_time);
            println!(
                "Currently modified: {}",
                self.editor.scene.get_base().file.modified
            );
            self.editor.scene.get_base_mut().file.modified = false;
            self.last_write = current_time;
        }
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
                last_write: SystemTime::now(),
            },
            _ => panic!(),
        }
    }
}
