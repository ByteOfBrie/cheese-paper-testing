use crate::components::Project;
use crate::components::file_objects::{FileObject, FileObjectStore, run_with_file_object};
use egui_ltreeview::{Action, NodeBuilder, TreeView};

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
        const node_height: f32 = 26.0;
        if self.is_folder() {
            builder.node(
                NodeBuilder::dir(self.get_base().metadata.id.clone())
                    .height(node_height)
                    .label(&self.get_base().metadata.name),
            );

            for child_id in self.get_base().children.iter() {
                run_with_file_object(&child_id, objects, |child, objects| {
                    child.build_tree(objects, builder);
                });
            }

            builder.close_dir();
        } else {
            builder.node(
                NodeBuilder::leaf(self.get_base().metadata.id.clone())
                    .height(node_height)
                    .label(&self.get_base().metadata.name),
            );
        }
    }
}

impl Project {
    fn build_tree(&mut self, builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>) {
        run_with_file_object(&self.text_id, &mut self.objects, |text, objects| {
            text.build_tree(objects, builder)
        });
        run_with_file_object(
            &self.characters_id,
            &mut self.objects,
            |characters, objects| characters.build_tree(objects, builder),
        );
        run_with_file_object(
            &self.worldbuilding_id,
            &mut self.objects,
            |worldbuilding, objects| worldbuilding.build_tree(objects, builder),
        );
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
        let (_response, actions) = TreeView::new(ui.make_persistent_id("project tree"))
            .allow_multi_selection(false)
            .show(ui, |builder| {
                self.project.build_tree(builder);
            });

        for action in actions {
            match action {
                Action::SetSelected(nodes) => {
                    println!("nodes: {nodes:?}");
                }
                _ => {}
            }
        }
    }
}
