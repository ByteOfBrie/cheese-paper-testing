use crate::ui::{prelude::*, project_editor::update_title};

use egui::Id;
use egui::ScrollArea;

impl Project {
    pub fn metadata_ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_project_metadata_editor(ui, ctx))
            .inner
    }

    fn show_project_metadata_editor(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut EditorContext,
    ) -> Vec<Id> {
        let mut ids = Vec::new();
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.base_metadata.name)
                    .id_salt("name")
                    .hint_text("Story Title")
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            self.process_response(&response);
            ids.push(response.id);

            // Special case: update the title if we've changed it:
            if response.changed() {
                update_title(&self.base_metadata.name, ui.ctx());
            }

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.metadata.genre)
                    .id_salt("genre")
                    .hint_text("Genre")
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            self.process_response(&response);
            ids.push(response.id);

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.metadata.author)
                    .id_salt("author")
                    .hint_text("Author Name")
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            self.process_response(&response);
            ids.push(response.id);

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.metadata.email)
                    .id_salt("email")
                    .hint_text("Author Email")
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            self.process_response(&response);
            ids.push(response.id);

            egui::CollapsingHeader::new("Story Description/Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(|ui: &'_ mut Ui| self.metadata.summary.ui(ui, ctx));
                    self.process_response(&response);
                    ids.push(response.id);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add(|ui: &'_ mut Ui| self.metadata.notes.ui(ui, ctx));
                    self.process_response(&response);
                    ids.push(response.id);
                });
        });
        ids
    }

    pub fn process_response(&mut self, response: &egui::Response) {
        if response.changed() {
            self.file.modified = true;
        }
    }
}
