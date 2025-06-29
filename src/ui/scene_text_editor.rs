/// Text editor view for an entire scene object, will be embeded in other file objects
use crate::components::file_objects::FileObjectMetadata;
use crate::components::file_objects::Scene;

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

pub struct SceneTextEditor<'a> {
    pub scene: &'a mut Scene,
    pub metadata: &'a mut FileObjectMetadata,
}

impl<'a> SceneTextEditor<'a> {
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
        let SceneTextEditor { scene, metadata } = self;

        let response = ui.add(&mut BaseTextEditor::new(&mut scene.text));

        if response.changed() {
            println!("Changed lines in {}: {}", &metadata.name, &scene.text);
            println!("{} words", scene.word_count());
        }

        ui.add(&mut BaseTextEditor::new(&mut scene.metadata.notes));
        ui.add(&mut BaseTextEditor::new(&mut scene.metadata.summary));
    }
}
