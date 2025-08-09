mod character_editor;
mod folder_editor;
mod place_editor;
mod scene_editor;

use crate::{components::file_objects::FileObject, ui::project_editor::EditorContext};

pub trait FileObjectEditor: FileObject {
    fn ui<'a>(&'a mut self, ui: &'a mut egui::Ui, ctx: &'a mut EditorContext) -> egui::Response;

    fn process_response(&mut self, response: egui::Response) {
        if response.changed() {
            self.get_base_mut().file.modified = true;
        }
    }
}
