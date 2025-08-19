use super::ProjectEditor;

use crate::components::file_objects::move_child;
use crate::ui::prelude::*;

use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView};

/// Context menu actions for file objects, should only be constructed by file objects
enum ContextMenuActions {
    Delete {
        parent: FileID,
        deleting: FileID,
    },
    Add {
        parent: FileID,
        position: DirPosition<FileID>,
        file_type: FileType,
    },
}

// TODO: scale off of font size
const NODE_HEIGHT: f32 = 26.0;

impl dyn FileObject {
    fn build_tree(
        &self,
        objects: &FileObjectStore,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, Page>,
        actions: &mut Vec<ContextMenuActions>,
        parent_id: Option<FileID>,
    ) {
        let node_name = if self.get_base().metadata.name.is_empty() {
            self.empty_string_name().to_string()
        } else {
            self.get_base().metadata.name.clone()
        };

        // first, construct the node. we avoid a lot of duplication by putting it into a variable
        // before sticking it in the nodebuilder
        let base_node_id: Page = self.id().clone().into();
        let base_node_builder = if self.is_folder() {
            NodeBuilder::dir(base_node_id)
        } else {
            NodeBuilder::leaf(base_node_id)
        };

        // compute some stuff for our context menu:
        let (add_parent, position) = if self.is_folder() {
            (Some(self.id().clone()), DirPosition::Last)
        } else {
            (parent_id.clone(), DirPosition::After(self.id().clone()))
        };

        let node = base_node_builder
            .height(NODE_HEIGHT)
            .label(node_name)
            .context_menu(|ui| {
                // We can safely call unwrap on parent here because children can't be root nodes
                if ui.button("New Scene").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.as_ref().unwrap().clone(),
                        position: position.clone(),
                        file_type: FileType::Scene,
                    });
                    ui.close();
                }
                if ui.button("New Character").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.as_ref().unwrap().clone(),
                        position: position.clone(),
                        file_type: FileType::Character,
                    });
                    ui.close();
                }
                if ui.button("New Folder").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.as_ref().unwrap().clone(),
                        position: position.clone(),
                        file_type: FileType::Folder,
                    });
                    ui.close();
                }
                if ui.button("New Place").clicked() {
                    actions.push(ContextMenuActions::Add {
                        parent: add_parent.as_ref().unwrap().clone(),
                        position: position.clone(),
                        file_type: FileType::Place,
                    });
                    ui.close();
                }

                ui.separator();

                if let Some(parent) = parent_id.clone()
                    && ui.button("Delete").clicked()
                {
                    actions.push(ContextMenuActions::Delete {
                        parent,
                        deleting: self.id().clone(),
                    });
                }
            });

        builder.node(node);

        if self.is_folder() {
            for child in self.children(objects) {
                child
                    .borrow_mut()
                    .build_tree(objects, builder, actions, Some(self.id().clone()));
            }

            builder.close_dir();
        }
    }
}

impl Project {
    fn build_tree(
        &mut self,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, Page>,
        actions: &mut Vec<ContextMenuActions>,
    ) {
        // Add special project metadata to the tree
        builder.node(
            NodeBuilder::leaf(Page::ProjectMetadata)
                .label("Project")
                .height(NODE_HEIGHT),
        );

        // Create the rest of the top level tree
        self.objects
            .get(&self.text_id)
            .unwrap()
            .borrow_mut()
            .build_tree(&self.objects, builder, actions, None);
        self.objects
            .get(&self.characters_id)
            .unwrap()
            .borrow_mut()
            .build_tree(&self.objects, builder, actions, None);
        self.objects
            .get(&self.worldbuilding_id)
            .unwrap()
            .borrow_mut()
            .build_tree(&self.objects, builder, actions, None);
    }
}

pub fn ui(editor: &mut ProjectEditor, ui: &mut egui::Ui) {
    let mut context_menu_actions: Vec<ContextMenuActions> = Vec::new();
    let (_response, actions) = TreeView::new(ui.make_persistent_id("project tree"))
        .allow_multi_selection(false)
        .show_state(ui, &mut editor.tree_state, |builder| {
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
                // Moves only make sense if the source and target are both file objects.
                // This logic only allows for moving individual file objects,
                if let Some(source) = drag_and_drop.source.first()
                    && let Page::FileObject(source_file_id) = source
                    && let Page::FileObject(target_file_id) = &drag_and_drop.target
                {
                    // Don't move one of the roots
                    if *source_file_id == editor.project.text_id
                        || *source_file_id == editor.project.characters_id
                        || *source_file_id == editor.project.worldbuilding_id
                    {
                        continue;
                    }

                    let index: usize = match drag_and_drop.position {
                        egui_ltreeview::DirPosition::First => 0,
                        egui_ltreeview::DirPosition::Last => editor
                            .project
                            .objects
                            .get(target_file_id)
                            .expect("objects in the tree must be in the object map")
                            .borrow()
                            .get_base()
                            .children
                            .len(),
                        egui_ltreeview::DirPosition::Before(node) => {
                            if let Page::FileObject(node_id) = node {
                                editor
                                    .project
                                    .objects
                                    .get(&node_id)
                                    .expect("objects in the tree must be in the object map")
                                    .borrow()
                                    .get_base()
                                    .index
                                    .expect("nodes in the tree should always have indexes")
                            } else {
                                log::error!(
                                    "Encountered invalid move to {target_file_id:?}: found file object with\
                                    a child that was not a file object"
                                );
                                continue;
                            }
                        }
                        egui_ltreeview::DirPosition::After(node) => {
                            if let Page::FileObject(node_id) = node {
                                let node_position = editor
                                    .project
                                    .objects
                                    .get(&node_id)
                                    .expect("objects in the tree must be in the object map")
                                    .borrow()
                                    .get_base()
                                    .index
                                    .expect("nodes in the tree should always have indexes");

                                node_position + 1
                            } else {
                                log::error!(
                                    "Encountered invalid move to {target_file_id:?}: found file object with\
                                    a child that was not a file object"
                                );
                                continue;
                            }
                        }
                    };

                    for object in editor.project.objects.values() {
                        if object.borrow().get_base().children.contains(source_file_id) {
                            match move_child(
                                source_file_id,
                                object.borrow().id(),
                                target_file_id,
                                index,
                                &editor.project.objects,
                            ) {
                                Ok(()) => continue,
                                Err(err) => {
                                    log::error!(
                                        "error encountered while moving file object: {err:?}"
                                    );
                                }
                            }
                        }
                    }

                    // If the move above was successful, we would continue, so this will only be
                    // reached in error scenarios
                    log::error!(
                        "failed to move {source_file_id} to {target_file_id}: could not find source \
                        object parent in tree"
                    );
                }
            }
            _ => {}
        }
    }

    for action in context_menu_actions {
        match action {
            ContextMenuActions::Delete { parent, deleting } => {
                // Delete the actual file object (removes from other objects and file on disk)
                if let Err(err) =
                    <dyn FileObject>::remove_child(&deleting, &parent, &mut editor.project.objects)
                {
                    log::error!(
                        "Encountered error while trying to delete element: {deleting:?}: {err}"
                    );
                }
            }
            ContextMenuActions::Add {
                parent,
                position,
                file_type,
            } => {
                let result = editor
                    .project
                    .objects
                    .get(&parent)
                    .unwrap()
                    .borrow_mut()
                    .create_child(file_type, position, &editor.project.objects);

                match result {
                    Ok(new_child) => editor.project.add_object(new_child),
                    Err(err) => {
                        log::error!("Encountered error while trying to add child: {err}")
                    }
                }
            }
        }
    }
}
