use crate::components::Project;
use egui_ltreeview::TreeView;

#[derive(Debug)]
pub struct ProjectEditor {
    pub project: Project,
}

impl ProjectEditor {
    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_tree(ui);
        });
    }
    fn ui(&mut self, ui: &mut egui::Ui) {}

    fn draw_tree(&mut self, ui: &mut egui::Ui) {
        TreeView::new(ui.make_persistent_id("Names tree view")).show(ui, |builder| {
            builder.dir(0, "Root");
            builder.dir(1, "Foo");
            builder.leaf(2, "Ava");
            builder.dir(3, "Bar");
            builder.leaf(4, "Benjamin");
            builder.leaf(5, "Charlotte");
            builder.close_dir();
            builder.close_dir();
            builder.leaf(6, "Daniel");
            builder.leaf(7, "Emma");
            builder.dir(8, "Baz");
            builder.leaf(9, "Finn");
            builder.leaf(10, "Grayson");
            builder.close_dir();
            builder.close_dir();
        });
    }
}
