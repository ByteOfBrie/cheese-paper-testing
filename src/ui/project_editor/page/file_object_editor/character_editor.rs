use crate::ui::prelude::*;

use super::FileObjectEditor;
use crate::components::file_objects::Character;
use crate::components::file_objects::FileObject;

use egui::Id;
use egui::ScrollArea;

impl FileObjectEditor for Character {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        let sidebar_ids = egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..)
            .show_inside(ui, |ui| self.show_sidebar(ui, ctx))
            .inner;

        let mut ids = egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_editor(ui, ctx))
            .inner;

        ids.extend(sidebar_ids);
        ids
    }

    fn for_each_textbox<'a>(&'a self, f: &mut dyn FnMut(&Text, &'static str)) {
        f(&self.metadata.summary, "summary");
        f(&self.metadata.notes, "notes");
        f(&self.metadata.appearance, "appearance");
        f(&self.metadata.personality, "personality");
        f(&self.metadata.goal, "goal");
        f(&self.metadata.conflict, "conflict");
        f(&self.metadata.habits, "habits");
    }

    fn for_each_textbox_mut<'a>(&'a mut self, f: &mut dyn FnMut(&mut Text, &'static str)) {
        f(&mut self.metadata.summary, "summary");
        f(&mut self.metadata.notes, "notes");
        f(&mut self.metadata.appearance, "appearance");
        f(&mut self.metadata.personality, "personality");
        f(&mut self.metadata.goal, "goal");
        f(&mut self.metadata.conflict, "conflict");
        f(&mut self.metadata.habits, "habits");
    }
}

impl Character {
    fn show_sidebar(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        let mut ids = Vec::new();
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.get_base_mut().metadata.name)
                    .id_salt("name")
                    .hint_text("Character Name")
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            self.process_response(&response);
            ids.push(response.id);

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
                    self.process_response(&response);
                    ids.push(response.id);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        |ui: &'_ mut Ui| self.metadata.notes.ui(ui, ctx),
                    );
                    self.process_response(&response);
                    ids.push(response.id);
                });
        });

        ids
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        let mut ids = Vec::new();
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            ui.label("Appearance");
            let response: egui::Response =
                ui.add(|ui: &'_ mut Ui| self.metadata.appearance.ui(ui, ctx));
            self.process_response(&response);
            ids.push(response.id);

            ui.label("Personality");
            let response: egui::Response =
                ui.add(|ui: &'_ mut Ui| self.metadata.personality.ui(ui, ctx));
            self.process_response(&response);
            ids.push(response.id);

            ui.label("Goals");
            let response: egui::Response = ui.add(|ui: &'_ mut Ui| self.metadata.goal.ui(ui, ctx));
            self.process_response(&response);
            ids.push(response.id);

            ui.label("Conflicts");
            let response: egui::Response =
                ui.add(|ui: &'_ mut Ui| self.metadata.conflict.ui(ui, ctx));
            self.process_response(&response);
            ids.push(response.id);

            ui.label("Habits");
            let response: egui::Response =
                ui.add(|ui: &'_ mut Ui| self.metadata.habits.ui(ui, ctx));
            self.process_response(&response);
            ids.push(response.id);
        });
        ids
    }
}
