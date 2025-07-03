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
                let response = ui.add_sized(
                    ui.available_size(),
                    &mut BaseTextEditor::new(&mut self.scene.text),
                );

                self.process_response(response);
            });
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.scene.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::CollapsingHeader::new("Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response =
                        ui.add(&mut BaseTextEditor::new(&mut self.scene.metadata.summary));
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(&mut BaseTextEditor::new(&mut self.scene.metadata.notes));
                    self.process_response(response);
                });

            egui::TopBottomPanel::bottom("word_count").show_inside(ui, |ui| {
                let words = self.scene.word_count();
                let text = format!("{words} Words");
                ui.vertical_centered(|ui| {
                    let response = ui.label(text);
                    self.process_response(response);
                });
            })
        });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.scene.get_base_mut().file.modified = true;
        }
    }
}
