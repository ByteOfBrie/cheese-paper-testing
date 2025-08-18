use crate::components::Project;
use crate::components::Text;
use crate::ui::EditorContext;
use egui::{Response, Ui};

use egui::ScrollArea;

impl Project {
    pub fn metadata_ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui, ctx))
            .response
    }

    fn for_each_textbox<'a>(&'a self, f: &mut dyn FnMut(&Text, &'static str)) {
        f(&self.metadata.summary, "summary");
        f(&self.metadata.notes, "notes");
    }

    fn for_each_textbox_mut<'a>(&'a mut self, f: &mut dyn FnMut(&mut Text, &'static str)) {
        f(&mut self.metadata.summary, "summary");
        f(&mut self.metadata.notes, "notes");
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.base_metadata.name)
                    .id_salt("name")
                    .hint_text("Story Title")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.metadata.genre)
                    .id_salt("genre")
                    .hint_text("Genre")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.metadata.author)
                    .id_salt("author")
                    .hint_text("Author Name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.metadata.email)
                    .id_salt("email")
                    .hint_text("Author Email")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            egui::CollapsingHeader::new("Story Description/Summary")
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

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.file.modified = true;
        }
    }
}
