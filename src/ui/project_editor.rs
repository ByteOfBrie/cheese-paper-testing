use crate::components::Project;
use crate::components::file_objects::{
    FileObject, FileObjectStore, MutFileObjectTypeInterface, move_child, run_with_file_object,
};
use crate::ui::{CharacterEditor, FolderEditor, PlaceEditor, SceneEditor};
use egui::Widget;
use egui_dock::{DockArea, DockState};
use egui_ltreeview::{Action, NodeBuilder, TreeView};

#[derive(Debug)]
pub struct ProjectEditor {
    pub project: Project,
    dock_state: DockState<String>,
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

struct TabViewer<'a> {
    project: &'a mut Project,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = String;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::from(tab.clone())
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        self.project
            .objects
            .get(tab)
            .unwrap()
            .get_base()
            .metadata
            .name
            .clone()
            .into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let file_object = self.project.objects.get_mut(tab).unwrap();
        match file_object.get_file_type_mut() {
            MutFileObjectTypeInterface::Scene(obj) => SceneEditor { scene: obj }.ui(ui),
            MutFileObjectTypeInterface::Character(obj) => CharacterEditor { character: obj }.ui(ui),
            MutFileObjectTypeInterface::Folder(obj) => FolderEditor { folder: obj }.ui(ui),
            MutFileObjectTypeInterface::Place(obj) => PlaceEditor { place: obj }.ui(ui),
        };
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

impl ProjectEditor {
    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("project tree panel").show(ctx, |ui| {
            self.draw_tree(ui);
        });

        // render the tab view
        DockArea::new(&mut self.dock_state)
            .allowed_splits(egui_dock::AllowedSplits::None)
            .show_leaf_collapse_buttons(false)
            .show(
                ctx,
                &mut TabViewer {
                    project: &mut self.project,
                },
            )
    }

    fn draw_tree(&mut self, ui: &mut egui::Ui) {
        let (_response, actions) = TreeView::new(ui.make_persistent_id("project tree"))
            .allow_multi_selection(false)
            .show(ui, |builder| {
                self.project.build_tree(builder);
            });

        for action in actions {
            match action {
                Action::SetSelected(selected_file_ids) => {
                    // Open nodes when they're selected
                    if let Some(file_id) = selected_file_ids.get(0) {
                        if let Some(tab_position) = self.dock_state.find_tab(file_id) {
                            // We've already opened this, just select it
                            self.dock_state.set_active_tab(tab_position);
                        } else {
                            // New file object, open it for editing
                            self.dock_state.push_to_first_leaf(file_id.clone());
                        }
                    }
                }
                Action::Move(drag_and_drop) => {
                    if let Some(source) = drag_and_drop.source.get(0) {
                        // Don't move one of the roots
                        if *source == self.project.text_id
                            || *source == self.project.characters_id
                            || *source == self.project.worldbuilding_id
                        {
                            continue;
                        }

                        let index: usize = match drag_and_drop.position {
                            egui_ltreeview::DirPosition::First => 0,
                            egui_ltreeview::DirPosition::Last => self
                                .project
                                .objects
                                .get(&drag_and_drop.target)
                                .expect("objects in the tree must be in the object map")
                                .get_base()
                                .children
                                .len(),
                            egui_ltreeview::DirPosition::Before(node) => self
                                .project
                                .objects
                                .get(&node)
                                .expect("objects in the tree must be in the object map")
                                .get_base()
                                .index
                                .expect("nodes in the tree should always have indexes"),
                            egui_ltreeview::DirPosition::After(node) => {
                                self.project
                                    .objects
                                    .get(&node)
                                    .expect("objects in the tree must be in the object map")
                                    .get_base()
                                    .index
                                    .expect("nodes in the tree should always have indexes")
                                    + 1
                            }
                        };

                        let mut source_parent: Option<String> = None;

                        for object in self.project.objects.values() {
                            if object.get_base().children.contains(source) {
                                source_parent = Some(object.get_base().metadata.id.clone());
                            }
                        }

                        if let Err(err) = move_child(
                            &source,
                            &source_parent.expect("moving item's parent should be in tree"),
                            &drag_and_drop.target,
                            index,
                            &mut self.project.objects,
                        ) {
                            log::error!("error encountered while moving file object: {err:?}");
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn new(project: Project) -> Self {
        Self {
            project,
            dock_state: DockState::new(vec![]),
        }
    }

    pub fn save(&mut self) {
        if let Err(err) = self.project.save() {
            log::error!("encountered error while saving project: {err}");
        }
    }
}
