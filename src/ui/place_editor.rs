use crate::components::file_objects::FileObject;
use crate::components::file_objects::Place;
use egui::{Response, Widget};

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

/// Text editor view for an entire scene object, will be embeded in other file objects
#[derive(Debug)]
pub struct PlaceEditor<'a> {
    pub place: &'a mut Place,
}

impl<'a> Widget for &mut PlaceEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..=500.0)
            .show_inside(ui, |ui| self.show_sidebar(ui));

        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui))
            .response
    }
}

impl<'a> PlaceEditor<'a> {
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("main metadata")
            .show(ui, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.place.get_base_mut().metadata.name)
                        .char_limit(50)
                        .id_salt("name")
                        .desired_width(f32::INFINITY),
                );
                self.process_response(response);

                ui.label("Notes");
                let response = ui.add_sized(
                    ui.available_size(),
                    &mut BaseTextEditor::new(&mut self.place.metadata.notes),
                );
                self.process_response(response);
            });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("main metadata")
            .show(ui, |ui| {
                ui.label("Connection To Story");
                let response = ui.add(&mut BaseTextEditor::new(
                    &mut self.place.metadata.connection,
                ));
                self.process_response(response);

                ui.label("Description");
                let response = ui.add(&mut BaseTextEditor::new(
                    &mut self.place.metadata.description,
                ));
                self.process_response(response);

                ui.label("Appearance");
                let response = ui.add(&mut BaseTextEditor::new(
                    &mut self.place.metadata.appearance,
                ));
                self.process_response(response);

                ui.label("Other Senses");
                let response = ui.add(&mut BaseTextEditor::new(
                    &mut self.place.metadata.other_senses,
                ));
                self.process_response(response);
            });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.place.get_base_mut().file.modified = true;
        }
    }
}
