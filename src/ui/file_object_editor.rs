use std::fmt::Debug;

pub trait FileObjectEditorType<'a>: Debug {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response;
}
