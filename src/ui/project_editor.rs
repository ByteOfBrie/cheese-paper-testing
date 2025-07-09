use crate::components::Project;
use crate::components::file_objects::{
    FileObject, FileObjectStore, MutFileObjectTypeInterface, run_with_file_object,
};
use crate::ui::{CharacterEditor, FolderEditor, PlaceEditor, SceneEditor};
use egui::{Response, Widget};
use egui_ltreeview::{Action, NodeBuilder, TreeView};

#[derive(Debug)]
pub struct ProjectEditor {
    pub project: Project,
    open_scene: Option<String>,
}

impl dyn FileObject {
    fn build_tree(
        &self,
        objects: &mut FileObjectStore,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
    ) {
        const NODE_HEIGHT: f32 = 26.0;
        if self.is_folder() {
            builder.node(
                NodeBuilder::dir(self.get_base().metadata.id.clone())
                    .height(NODE_HEIGHT)
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
                    .height(NODE_HEIGHT)
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

        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        if let Some(open_scene) = &self.open_scene {
            let file_object = self.project.objects.get_mut(open_scene).unwrap();
            match file_object.get_file_type_mut() {
                MutFileObjectTypeInterface::Scene(obj) => SceneEditor { scene: obj }.ui(ui),
                MutFileObjectTypeInterface::Character(obj) => {
                    CharacterEditor { character: obj }.ui(ui)
                }
                MutFileObjectTypeInterface::Folder(obj) => FolderEditor { folder: obj }.ui(ui),
                MutFileObjectTypeInterface::Place(obj) => PlaceEditor { place: obj }.ui(ui),
            }
        } else {
            ui.response()
        }
    }

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

                    // We only allow for one node at a time to be selected, so this is fine
                    self.open_scene = nodes.get(0).map(|id| id.clone());
                }
                Action::Drag(drag) => println!("drag: {:?}", drag.source),
                _ => {}
            }
        }
    }

    pub fn new(project: Project) -> Self {
        Self {
            project,
            open_scene: None,
        }
    }
}
