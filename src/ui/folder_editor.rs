use crate::components::file_objects::FileObject;
use crate::components::file_objects::Folder;

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

/// Text editor view for an entire scene object, will be embeded in other file objects
pub struct FolderEditor<'a> {
    pub folder: &'a mut Folder,
}

impl<'a> FolderEditor<'a> {
    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| self.show_editor(ui));
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.folder.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::CollapsingHeader::new("Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response =
                        ui.add(&mut BaseTextEditor::new(&mut self.folder.metadata.summary));
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response =
                        ui.add(&mut BaseTextEditor::new(&mut self.folder.metadata.notes));
                    self.process_response(response);
                });
        });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.folder.get_base_mut().file.modified = true;
        }
    }
}
