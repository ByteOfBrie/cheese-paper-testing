use crate::components::file_objects::FileObject;
use crate::components::file_objects::Scene;
use egui::{Response, Widget};

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

/// Text editor view for an entire scene object, will be embeded in other file objects
#[derive(Debug)]
pub struct SceneEditor<'a> {
    pub scene: &'a mut Scene,
}

impl<'a> Widget for &mut SceneEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..=500.0)
            .show_inside(ui, |ui| self.show_sidebar(ui));

        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_text_editor(ui))
            .response
    }
}

impl<'a> SceneEditor<'a> {
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
                    .hint_text("Scene Name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::TopBottomPanel::bottom("word_count").show_inside(ui, |ui| {
                let words = self.scene.word_count();
                let text = format!("{words} Words");
                ui.vertical_centered(|ui| {
                    let response = ui.label(text);
                    self.process_response(response);
                });
            });

            // Make each text box take up a bit of the screen by default
            // this could be smarter, but available/2.5 is visually better than /3, and /2
            // doesn't work (because the collapsing headers themself take up space)
            let min_height = ui.available_height() / 2.5;

            egui::CollapsingHeader::new("Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        &mut BaseTextEditor::new(&mut self.scene.metadata.summary),
                    );
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        &mut BaseTextEditor::new(&mut self.scene.metadata.notes),
                    );
                    self.process_response(response);
                });
        });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.scene.get_base_mut().file.modified = true;
        }
    }
}
