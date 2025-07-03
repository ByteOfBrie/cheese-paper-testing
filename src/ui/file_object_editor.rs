use std::fmt::Debug;

pub trait FileObjectEditor<'a>: Debug {
    fn panels(&mut self, ctx: &egui::Context);
}
