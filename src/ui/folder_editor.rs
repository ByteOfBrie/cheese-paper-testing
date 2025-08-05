use crate::components::file_objects::FileObject;
use crate::components::file_objects::Folder;
use crate::ui::EditorContext;
use crate::ui::FileObjectEditor;
use egui::Response;

use crate::ui::TextBox;
use egui::ScrollArea;

impl FileObjectEditor for Folder {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui, ctx))
            .response
    }
}

impl Folder {
    fn show_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name")
                    .hint_text("Folder Name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::CollapsingHeader::new("Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(&mut TextBox::new(&mut self.metadata.summary, ctx));
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(&mut TextBox::new(&mut self.metadata.notes, ctx));
                    self.process_response(response);
                });
        });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.get_base_mut().file.modified = true;
        }
    }
}
