use crate::components::file_objects::FileObject;
use crate::components::file_objects::Place;
use crate::ui::EditorContext;
use crate::ui::FileObjectEditor;
use egui::Response;

use crate::ui::BaseTextEditor;
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
                let response = ui.add_sized(
                    ui.available_size(),
                    &mut BaseTextEditor::new(&mut self.metadata.notes, ctx),
                );
                self.process_response(response);
            });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical()
            .id_salt("main metadata")
            .show(ui, |ui| {
                ui.label("Connection To Story");
                let response = ui.add(&mut BaseTextEditor::new(&mut self.metadata.connection, ctx));
                self.process_response(response);

                ui.label("Description");
                let response = ui.add(&mut BaseTextEditor::new(
                    &mut self.metadata.description,
                    ctx,
                ));
                self.process_response(response);

                ui.label("Appearance");
                let response = ui.add(&mut BaseTextEditor::new(&mut self.metadata.appearance, ctx));
                self.process_response(response);

                ui.label("Other Senses");
                let response = ui.add(&mut BaseTextEditor::new(
                    &mut self.metadata.other_senses,
                    ctx,
                ));
                self.process_response(response);
            });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.get_base_mut().file.modified = true;
        }
    }
}
