use super::FileObjectEditor;
use crate::components::file_objects::FileObject;
use crate::components::file_objects::Scene;
use crate::ui::project_editor::EditorContext;
use egui::{Response, Ui};

use egui::ScrollArea;

impl FileObjectEditor for Scene {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..=500.0)
            .show_inside(ui, |ui| self.show_sidebar(ui, ctx));

        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_text_editor(ui, ctx))
            .response
    }
}

impl Scene {
    fn show_text_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical()
            .id_salt("text")
            .auto_shrink(egui::Vec2b { x: false, y: false })
            .show(ui, |ui| {
                let response =
                    ui.add_sized(ui.available_size(), |ui: &'_ mut Ui| self.text.ui(ui, ctx));

                self.process_response(response);
            });
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name")
                    .hint_text("Scene Name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::TopBottomPanel::bottom("word_count").show_inside(ui, |ui| {
                let words = self.word_count();
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
                        |ui: &'_ mut Ui| self.metadata.summary.ui(ui, ctx),
                    );
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        |ui: &'_ mut Ui| self.metadata.notes.ui(ui, ctx),
                    );
                    self.process_response(response);
                });
        });
    }
}
