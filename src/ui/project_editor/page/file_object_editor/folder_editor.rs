use crate::ui::prelude::*;

use super::FileObjectEditor;
use crate::components::file_objects::FileObject;
use crate::components::file_objects::Folder;

use egui::ScrollArea;

impl FileObjectEditor for Folder {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui, ctx))
            .response
    }

    fn for_each_textbox<'a>(&'a self, f: &mut dyn FnMut(&Text, &'static str)) {
        f(&self.metadata.summary, "Summary");
        f(&self.metadata.notes, "Notes");
    }

    fn for_each_textbox_mut<'a>(&'a mut self, f: &mut dyn FnMut(&mut Text, &'static str)) {
        f(&mut self.metadata.summary, "Summary");
        f(&mut self.metadata.notes, "Notes");
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
                    let response = ui.add(|ui: &'_ mut Ui| self.metadata.summary.ui(ui, ctx));
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(|ui: &'_ mut Ui| self.metadata.notes.ui(ui, ctx));
                    self.process_response(response);
                });
        });
    }
}
