use crate::components::file_objects::FileObject;
use crate::components::file_objects::Scene;

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

/// Text editor view for an entire scene object, will be embeded in other file objects
pub struct SceneTextEditor<'a> {
    pub scene: &'a mut Scene,
}

impl<'a> SceneTextEditor<'a> {
    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            cols[0].vertical(|ui| self.show_text_editor(ui));
            cols[1].vertical(|ui| {
                ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
                    ui.label("Summary");
                    ui.add(&mut BaseTextEditor::new(&mut self.scene.metadata.summary));
                    ui.label("Notes");
                    ui.add(&mut BaseTextEditor::new(&mut self.scene.metadata.notes));
                })
            })
        });
    }

    fn show_text_editor(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("text")
            .auto_shrink(egui::Vec2b { x: false, y: false })
            .show(ui, |ui| {
                let response = ui.add(&mut BaseTextEditor::new(&mut self.scene.text));

                if response.changed() {
                    println!(
                        "Changed lines in {}: {}",
                        &self.scene.get_base().metadata.name,
                        &self.scene.text
                    );
                    println!("{} words", self.scene.word_count());
                }
            });
    }
}
