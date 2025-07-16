use crate::components::file_objects::FileObject;
use crate::components::file_objects::Folder;
use egui::{Response, Widget};
use spellbook::Dictionary;

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

/// Text editor view for an entire scene object, will be embeded in other file objects
#[derive(Debug)]
pub struct FolderEditor<'a> {
    pub folder: &'a mut Folder,
    pub dictionary: &'a Option<&'a mut Dictionary>,
    pub cursor_pos: &'a mut usize,
}

impl<'a> Widget for &mut FolderEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui))
            .response
    }
}

impl<'a> FolderEditor<'a> {
    fn show_editor(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.folder.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name")
                    .hint_text("Folder Name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::CollapsingHeader::new("Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(&mut BaseTextEditor::new(
                        &mut self.folder.metadata.summary,
                        self.dictionary,
                        self.cursor_pos,
                    ));
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(&mut BaseTextEditor::new(
                        &mut self.folder.metadata.notes,
                        self.dictionary,
                        self.cursor_pos,
                    ));
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
