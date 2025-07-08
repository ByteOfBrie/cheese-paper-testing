use crate::components::Project;
use crate::components::file_objects::{FileObject, FileObjectStore, run_with_file_object};
use crate::components::project::ProjectFolder;
use egui_ltreeview::TreeView;

#[derive(Debug)]
pub struct ProjectEditor {
    pub project: Project,
}

impl dyn FileObject {
    fn build_tree(
        &self,
        objects: &mut FileObjectStore,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
    ) {
        if self.is_folder() {
            builder.dir(
                self.get_base().metadata.id.clone(),
                &self.get_base().metadata.name,
            );

            for child_id in self.get_base().children.iter() {
                run_with_file_object(&child_id, objects, |child, objects| {
                    child.build_tree(objects, builder);
                });
            }

            builder.close_dir();
        } else {
            builder.leaf(
                self.get_base().metadata.id.clone(),
                format!("leaf: {}", &self.get_base().metadata.name),
            );
        }
    }
}

impl ProjectEditor {
    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("project tree panel").show(ctx, |ui| {
            self.draw_tree(ui);
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui) {}

    fn draw_tree(&mut self, ui: &mut egui::Ui) {
        TreeView::new(ui.make_persistent_id("project tree")).show(ui, |builder| {
            self.project
                .run_with_folder(ProjectFolder::text, |text, objects| {
                    text.build_tree(objects, builder);
                });
        });
    }
}
