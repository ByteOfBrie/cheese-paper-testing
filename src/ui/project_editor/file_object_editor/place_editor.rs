use super::FileObjectEditor;
use crate::components::file_objects::FileObject;
use crate::components::file_objects::Place;
use crate::ui::EditorContext;
use egui::{Response, Ui};

use egui::ScrollArea;

impl FileObjectEditor for Place {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..=500.0)
            .show_inside(ui, |ui| self.show_sidebar(ui, ctx));

        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui, ctx))
            .response
    }
}

impl Place {
    fn show_sidebar(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical()
            .id_salt("main metadata")
            .show(ui, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.get_base_mut().metadata.name)
                        .char_limit(50)
                        .id_salt("name")
                        .hint_text("Place Name")
                        .desired_width(f32::INFINITY),
                );
                self.process_response(response);

                ui.label("Notes");
                let response = ui.add_sized(ui.available_size(), |ui: &'_ mut Ui| {
                    self.metadata.notes.ui(ui, ctx)
                });
                self.process_response(response);
            });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical()
            .id_salt("main metadata")
            .show(ui, |ui| {
                ui.label("Connection To Story");
                let response = ui.add(|ui: &'_ mut Ui| self.metadata.connection.ui(ui, ctx));
                self.process_response(response);

                ui.label("Description");
                let response = ui.add(|ui: &'_ mut Ui| self.metadata.description.ui(ui, ctx));
                self.process_response(response);

                ui.label("Appearance");
                let response = ui.add(|ui: &'_ mut Ui| self.metadata.appearance.ui(ui, ctx));
                self.process_response(response);

                ui.label("Other Senses");
                let response = ui.add(|ui: &'_ mut Ui| self.metadata.other_senses.ui(ui, ctx));
                self.process_response(response);
            });
    }
}
