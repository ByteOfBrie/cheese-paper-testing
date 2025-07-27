use crate::{components::file_objects::FileObject, ui::project_editor::EditorContext};

pub trait FileObjectEditor: FileObject {
    fn ui<'a>(&'a mut self, ui: &'a mut egui::Ui, ctx: &'a mut EditorContext) -> egui::Response;
}
