use super::ProjectEditor;

use crate::components::Project;
use crate::components::file_objects::base::FileType;
use crate::components::file_objects::{
    FileObject, FileObjectStore, move_child, run_with_file_object,
};

use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView};

enum ContextMenuActions {
    Delete {
        parent: String,
        deleting: String,
    },
    Add {
        parent: String,
        position: DirPosition<String>,
        file_type: FileType,
    },
}

impl dyn FileObject {
    fn build_tree(
        &self,
        objects: &mut FileObjectStore,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
        actions: &mut Vec<ContextMenuActions>,
        parent_id: Option<&str>,
    ) {
        // TODO: scale off of font size
        const NODE_HEIGHT: f32 = 26.0;
        let node_name = if self.get_base().metadata.name.is_empty() {
            self.empty_string_name().to_string()
        } else {
            self.get_base().metadata.name.clone()
        };

        // first, construct the node. we avoid a lot of duplication by putting it into a variable
        // before sticking it in the nodebuilder
        let base_node = if self.is_folder() {
            NodeBuilder::dir(self.get_base().metadata.id.clone())
        } else {
            NodeBuilder::leaf(self.get_base().metadata.id.clone())
        };

        // compute some stuff for our context menu:
        let (add_parent, position) = if self.is_folder() {
            (
                Some(self.get_base().metadata.id.as_str()),
                DirPosition::Last,
            )
        } else {
            (
                parent_id,
                DirPosition::After(self.get_base().metadata.id.clone()),
            )
        };

        let node = base_node
            .height(NODE_HEIGHT)
            .label(node_name)
            .context_menu(|ui| {
                // We can safely call unwrap on parent here because children can't be root nodes
                if ui.button("New Scene").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.unwrap().to_string(),
                        position: position.clone(),
                        file_type: FileType::Scene,
                    });
                    ui.close();
                }
                if ui.button("New Character").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.unwrap().to_string(),
                        position: position.clone(),
                        file_type: FileType::Character,
                    });
                    ui.close();
                }
                if ui.button("New Folder").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.unwrap().to_string(),
                        position: position.clone(),
                        file_type: FileType::Folder,
                    });
                    ui.close();
                }
                if ui.button("New Place").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.unwrap().to_string(),
                        position: position.clone(),
                        file_type: FileType::Place,
                    });
                    ui.close();
                }

                ui.separator();

                if let Some(parent) = parent_id {
                    if ui.button("Delete").clicked() {
                        actions.push(ContextMenuActions::Delete {
                            parent: parent.to_string(),
                            deleting: self.get_base().metadata.id.clone(),
                        });
                    }
                }
            });

        builder.node(node);

        if self.is_folder() {
            for child_id in self.get_base().children.iter() {
                run_with_file_object(child_id, objects, |child, objects| {
                    child.build_tree(
                        objects,
                        builder,
                        actions,
                        Some(self.get_base().metadata.id.as_str()),
                    );
                });
            }

            builder.close_dir();
        }
    }
}

impl Project {
    fn build_tree(
        &mut self,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
        actions: &mut Vec<ContextMenuActions>,
    ) {
        run_with_file_object(&self.text_id, &mut self.objects, |text, objects| {
            text.build_tree(objects, builder, actions, None)
        });
        run_with_file_object(
            &self.characters_id,
            &mut self.objects,
            |characters, objects| characters.build_tree(objects, builder, actions, None),
        );
        run_with_file_object(
            &self.worldbuilding_id,
            &mut self.objects,
            |worldbuilding, objects| worldbuilding.build_tree(objects, builder, actions, None),
        );
    }
}

pub fn ui(editor: &mut ProjectEditor, ui: &mut egui::Ui) {
    let mut context_menu_actions: Vec<ContextMenuActions> = Vec::new();
    let (_response, actions) = TreeView::new(ui.make_persistent_id("project tree"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            editor
                .project
                .build_tree(builder, &mut context_menu_actions);
        });

    for action in actions {
        match action {
            Action::SetSelected(selected_file_ids) => {
                // Open nodes when they're selected
                if let Some(file_id) = selected_file_ids.first() {
                    editor.set_editor_tab(file_id);
                }
            }
            Action::Move(drag_and_drop) => {
                if let Some(source) = drag_and_drop.source.first() {
                    // Don't move one of the roots
                    if *source == editor.project.text_id
                        || *source == editor.project.characters_id
                        || *source == editor.project.worldbuilding_id
                    {
                        continue;
                    }

                    let index: usize = match drag_and_drop.position {
                        egui_ltreeview::DirPosition::First => 0,
                        egui_ltreeview::DirPosition::Last => editor
                            .project
                            .objects
                            .get(&drag_and_drop.target)
                            .expect("objects in the tree must be in the object map")
                            .get_base()
                            .children
                            .len(),
                        egui_ltreeview::DirPosition::Before(node) => editor
                            .project
                            .objects
                            .get(&node)
                            .expect("objects in the tree must be in the object map")
                            .get_base()
                            .index
                            .expect("nodes in the tree should always have indexes"),
                        egui_ltreeview::DirPosition::After(node) => {
                            editor
                                .project
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

                    for object in editor.project.objects.values() {
                        if object.get_base().children.contains(source) {
                            source_parent = Some(object.get_base().metadata.id.clone());
                        }
                    }

                    if let Err(err) = move_child(
                        source,
                        &source_parent.expect("moving item's parent should be in tree"),
                        &drag_and_drop.target,
                        index,
                        &mut editor.project.objects,
                    ) {
                        log::error!("error encountered while moving file object: {err:?}");
                    }
                }
            }
            _ => {}
        }
    }

    for action in context_menu_actions {
        match action {
            ContextMenuActions::Delete { parent, deleting } => {
                // TODO: find better way of doing this, prune elements before calling the viewer?
                if let Some(tab_position) = editor.dock_state.find_tab(&deleting) {
                    editor.dock_state.remove_tab(tab_position);
                }
                run_with_file_object(
                    parent.as_str(),
                    &mut editor.project.objects,
                    |parent, objects| {
                        if let Err(err) = parent.remove_child(&deleting, objects) {
                            log::error!(
                                "Encountered error while trying to delete element: {deleting}: {err}"
                            );
                        }
                    },
                )
            }
            ContextMenuActions::Add {
                parent,
                position,
                file_type,
            } => {
                match run_with_file_object(
                    &parent,
                    &mut editor.project.objects,
                    |parent, objects| parent.create_child(file_type, position, objects),
                ) {
                    Ok(new_child) => editor.project.add_object(new_child),
                    Err(err) => {
                        log::error!("Encountered error while trying to add child: {err}")
                    }
                }
            }
        }
    }
}
