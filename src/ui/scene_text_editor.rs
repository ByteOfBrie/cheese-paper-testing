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
        egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..=500.0)
            .show_inside(ui, |ui| self.show_sidebar(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.show_text_editor(ui));
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

    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.scene.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name"),
            );
            ui.collapsing("Summary", |ui| {
                ui.add(&mut BaseTextEditor::new(&mut self.scene.metadata.summary))
            });
            ui.collapsing("Notes", |ui| {
                ui.add(&mut BaseTextEditor::new(&mut self.scene.metadata.notes))
            });
        });
    }
}
