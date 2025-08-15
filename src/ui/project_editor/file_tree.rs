use std::rc::Rc;

use super::ProjectEditor;

use crate::components::Project;
use crate::components::file_objects::base::{FileID, FileType};
use crate::components::file_objects::{FileObject, FileObjectStore, move_child};

use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView};

enum ContextMenuActions {
    Delete {
        parent: FileID,
        deleting: FileID,
    },
    Add {
        parent: FileID,
        position: DirPosition<String>,
        file_type: FileType,
    },
}

impl dyn FileObject {
    fn build_tree(
        &self,
        objects: &FileObjectStore,
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
        actions: &mut Vec<ContextMenuActions>,
        parent_id: Option<FileID>,
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
            NodeBuilder::dir(self.id().to_string())
        } else {
            NodeBuilder::leaf(self.id().to_string())
        };

        // compute some stuff for our context menu:
        let (add_parent, position) = if self.is_folder() {
            (Some(self.id().clone()), DirPosition::Last)
        } else {
            (parent_id.clone(), DirPosition::After(self.id().to_string()))
        };

        let node = base_node
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
        builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
        actions: &mut Vec<ContextMenuActions>,
    ) {
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
                if let Some(source) = drag_and_drop.source.first() {
                    // Don't move one of the roots
                    if *source == *editor.project.text_id
                        || *source == *editor.project.characters_id
                        || *source == *editor.project.worldbuilding_id
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
                            .borrow()
                            .get_base()
                            .children
                            .len(),
                        egui_ltreeview::DirPosition::Before(node) => editor
                            .project
                            .objects
                            .get(&node)
                            .expect("objects in the tree must be in the object map")
                            .borrow()
                            .get_base()
                            .index
                            .expect("nodes in the tree should always have indexes"),
                        egui_ltreeview::DirPosition::After(node) => {
                            editor
                                .project
                                .objects
                                .get(&node)
                                .expect("objects in the tree must be in the object map")
                                .borrow()
                                .get_base()
                                .index
                                .expect("nodes in the tree should always have indexes")
                                + 1
                        }
                    };

                    let source: FileID = Rc::new(source.clone());
                    // TODO finish replacing the Strings with Rc everywhere to save on unnecessary clones (low priority)
                    let mut source_parent: Option<FileID> = None;

                    for object in editor.project.objects.values() {
                        if object.borrow().get_base().children.contains(&source) {
                            source_parent = Some(object.borrow().id().clone());
                        }
                    }

                    let target = Rc::new(drag_and_drop.target.to_string());

                    if let Err(err) = move_child(
                        &source,
                        &source_parent.expect("moving item's parent should be in tree"),
                        &target,
                        index,
                        &editor.project.objects,
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
                if let Err(err) =
                    <dyn FileObject>::remove_child(&deleting, &parent, &mut editor.project.objects)
                {
                    log::error!(
                        "Encountered error while trying to delete element: {deleting}: {err}"
                    );
                }
            }
            ContextMenuActions::Add {
                parent,
                position,
                file_type,
            } => {
                let resut = editor
                    .project
                    .objects
                    .get(&parent)
                    .unwrap()
                    .borrow_mut()
                    .create_child(file_type, position, &editor.project.objects);
                match resut {
                    Ok(new_child) => editor.project.add_object(new_child),
                    Err(err) => {
                        log::error!("Encountered error while trying to add child: {err}")
                    }
                }
            }
        }
    }
}
