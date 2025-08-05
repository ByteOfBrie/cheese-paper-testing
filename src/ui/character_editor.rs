use crate::components::file_objects::Character;
use crate::components::file_objects::FileObject;
use crate::ui::EditorContext;
use crate::ui::FileObjectEditor;
use egui::Response;

use crate::ui::TextBox;
use egui::ScrollArea;

impl FileObjectEditor for Character {
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

impl Character {
    fn show_sidebar(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.get_base_mut().metadata.name)
                    .char_limit(50)
                    .id_salt("name")
                    .hint_text("Character Name")
                    .desired_width(f32::INFINITY),
            );
            self.process_response(response);

            // Make each text box take up a bit of the screen by default
            // this could be smarter, but available/2.5 is visually better than /3, and /2
            // doesn't work (because the collapsing headers themself take up space)
            let min_height = ui.available_height() / 2.5;

            egui::CollapsingHeader::new("Summary")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        &mut TextBox::new(&mut self.metadata.summary, ctx),
                    );
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        &mut TextBox::new(&mut self.metadata.notes, ctx),
                    );
                    self.process_response(response);
                });
        });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            ui.label("Appearance");
            let response: egui::Response =
                ui.add(&mut TextBox::new(&mut self.metadata.appearance, ctx));
            self.process_response(response);

            ui.label("Personality");
            let response: egui::Response =
                ui.add(&mut TextBox::new(&mut self.metadata.personality, ctx));
            self.process_response(response);

            ui.label("Goals");
            let response: egui::Response = ui.add(&mut TextBox::new(&mut self.metadata.goal, ctx));
            self.process_response(response);

            ui.label("Conflicts");
            let response: egui::Response =
                ui.add(&mut TextBox::new(&mut self.metadata.conflict, ctx));
            self.process_response(response);

            ui.label("Habits");
            let response: egui::Response =
                ui.add(&mut TextBox::new(&mut self.metadata.habits, ctx));
            self.process_response(response);
        });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.get_base_mut().file.modified = true;
        }
    }
}
