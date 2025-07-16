use crate::components::file_objects::Character;
use crate::components::file_objects::FileObject;
use egui::{Response, Widget};
use spellbook::Dictionary;

use crate::ui::BaseTextEditor;
use egui::ScrollArea;

/// Text editor view for an entire scene object, will be embeded in other file objects
#[derive(Debug)]
pub struct CharacterEditor<'a> {
    pub character: &'a mut Character,
    pub dictionary: &'a Option<&'a mut Dictionary>,
    pub current_selected_word: &'a mut String,
}

impl<'a> Widget for &mut CharacterEditor<'a> {
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

impl<'a> CharacterEditor<'a> {
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.character.get_base_mut().metadata.name)
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
                        &mut BaseTextEditor::new(
                            &mut self.character.metadata.summary,
                            self.dictionary,
                            self.current_selected_word,
                        ),
                    );
                    self.process_response(response);
                });

            egui::CollapsingHeader::new("Notes")
                .default_open(true)
                .show(ui, |ui| {
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), min_height),
                        &mut BaseTextEditor::new(
                            &mut self.character.metadata.notes,
                            self.dictionary,
                            self.current_selected_word,
                        ),
                    );
                    self.process_response(response);
                });
        });
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            ui.label("Appearance");
            let response: egui::Response = ui.add(&mut BaseTextEditor::new(
                &mut self.character.metadata.appearance,
                self.dictionary,
                self.current_selected_word,
            ));
            self.process_response(response);

            ui.label("Appearance");
            let response: egui::Response = ui.add(&mut BaseTextEditor::new(
                &mut self.character.metadata.appearance,
                self.dictionary,
                self.current_selected_word,
            ));
            self.process_response(response);

            ui.label("Personality");
            let response: egui::Response = ui.add(&mut BaseTextEditor::new(
                &mut self.character.metadata.personality,
                self.dictionary,
                self.current_selected_word,
            ));
            self.process_response(response);

            ui.label("Goals");
            let response: egui::Response = ui.add(&mut BaseTextEditor::new(
                &mut self.character.metadata.goal,
                self.dictionary,
                self.current_selected_word,
            ));
            self.process_response(response);

            ui.label("Conflicts");
            let response: egui::Response = ui.add(&mut BaseTextEditor::new(
                &mut self.character.metadata.conflict,
                self.dictionary,
                self.current_selected_word,
            ));
            self.process_response(response);

            ui.label("Habits");
            let response: egui::Response = ui.add(&mut BaseTextEditor::new(
                &mut self.character.metadata.habits,
                self.dictionary,
                self.current_selected_word,
            ));
            self.process_response(response);
        });
    }

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.character.get_base_mut().file.modified = true;
        }
    }
}
