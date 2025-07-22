use crate::components::Project;
use crate::components::file_objects::base::{FileType, read_file_contents};
use crate::components::file_objects::{
    FileObject, FileObjectStore, MutFileObjectTypeInterface, move_child, run_with_file_object,
};
use crate::ui::{CharacterEditor, FolderEditor, PlaceEditor, SceneEditor};
use egui::Widget;
use egui_dock::{DockArea, DockState};
use egui_ltreeview::{Action, DirPosition, NodeBuilder, TreeView};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebouncedEvent, Debouncer, RecommendedCache, new_debouncer};
use spellbook::Dictionary;
use toml_edit::DocumentMut;

#[derive(Debug, Default)]
pub struct SpellCheckStatus {
    pub selected_word: String,
    pub correct: bool,
    pub suggestions: Vec<String>,
}

#[derive(Debug)]
pub struct ProjectEditor {
    pub project: Project,
    dock_state: DockState<String>,
    /// Possibly a temporary hack, need to find a reasonable way to update this when it's change
    /// in the project metadata editor as well
    title_needs_update: bool,
    dictionary: Option<Dictionary>,
    spellcheck_status: SpellCheckStatus,
    file_event_rx: std::sync::mpsc::Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    /// We don't need to do anything to the watcher, but we stop getting events if it's dropped
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
}

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
                run_with_file_object(&child_id, objects, |child, objects| {
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

struct TabViewer<'a> {
    project: &'a mut Project,
    dictionary: Option<&'a mut Dictionary>,
    spellcheck_status: &'a mut SpellCheckStatus,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = String;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::from(tab.clone())
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        if let Some(object) = self.project.objects.get(tab) {
            if object.get_base().metadata.name.is_empty() {
                object.empty_string_name().to_string()
            } else {
                object.get_base().metadata.name.clone()
            }
            .into()
        } else {
            "<Deleted>".into()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        if let Some(file_object) = self.project.objects.get_mut(tab) {
            match file_object.get_file_type_mut() {
                MutFileObjectTypeInterface::Scene(obj) => SceneEditor {
                    scene: obj,
                    dictionary: &self.dictionary,
                    spellcheck_status: self.spellcheck_status,
                }
                .ui(ui),
                MutFileObjectTypeInterface::Character(obj) => CharacterEditor {
                    character: obj,
                    dictionary: &self.dictionary,
                    spellcheck_status: self.spellcheck_status,
                }
                .ui(ui),
                MutFileObjectTypeInterface::Folder(obj) => FolderEditor {
                    folder: obj,
                    dictionary: &self.dictionary,
                    spellcheck_status: self.spellcheck_status,
                }
                .ui(ui),
                MutFileObjectTypeInterface::Place(obj) => PlaceEditor {
                    place: obj,
                    dictionary: &self.dictionary,
                    spellcheck_status: self.spellcheck_status,
                }
                .ui(ui),
            };
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

fn create_watcher() -> notify::Result<(
    Debouncer<RecommendedWatcher, RecommendedCache>,
    std::sync::mpsc::Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
)> {
    let (tx, rx) = std::sync::mpsc::channel();

    let watcher = new_debouncer(std::time::Duration::from_secs(2), None, tx)?;

    Ok((watcher, rx))
}

impl ProjectEditor {
    pub fn panels(&mut self, ctx: &egui::Context) {
        self.process_state(ctx);

        egui::SidePanel::left("project tree panel").show(ctx, |ui| {
            egui::ScrollArea::both()
                .id_salt("tree scroll")
                .show(ui, |ui| {
                    self.draw_tree(ui);
                });
        });

        // Before rendering the tab view, clear out any deleted scenes
        self.dock_state
            .retain_tabs(|tab_id| self.project.objects.contains_key(tab_id));

        // render the tab view
        DockArea::new(&mut self.dock_state)
            .allowed_splits(egui_dock::AllowedSplits::None)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show(
                ctx,
                &mut TabViewer {
                    project: &mut self.project,
                    dictionary: self.dictionary.as_mut(),
                    spellcheck_status: &mut self.spellcheck_status,
                },
            )
    }

    fn process_state(&mut self, ctx: &egui::Context) {
        if self.title_needs_update {
            // Set the window title properly
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "Cheese Paper - {}",
                self.project.base_metadata.name
            )));
            self.title_needs_update = false;
        }

        if let Ok(response) = self.file_event_rx.try_recv() {
            match response {
                Ok(events) => {
                    for event in events {
                        use notify::EventKind;
                        match event.kind {
                            EventKind::Create(_create_kind) => {
                                // Somewhat tricky, we probably need to rescan that part
                                // entire part of the tree. We don't necessarily know which
                                // parents exist, and I can't trust that the events happened
                                // in order.

                                // For other solutions, we could scan every element in the
                                // tree and find the longest path that matches, then do a
                                // `from_file` one level below that. That's the least amount
                                // of code, but also requires the most allocations

                                // For a middle-ish ground, I could keep track of all of the
                                // know files in a list and match against that. That saves
                                // having to do a bunch of allocations but I don't know if it
                                // matters enough to bother keeping track of another thing,
                                // especially considering I have to remove from it at the same
                                // time
                            }
                            EventKind::Modify(_modify_kind) => {
                                // Try to read the file, if it has an ID, look up that ID
                                // and call reload file, otherwise give up (it might come in as
                                // a different event, but we don't care about modifications
                                // to files we don't know)
                                let modify_path = match event.paths.get(0) {
                                    Some(path) => path,
                                    None => {
                                        log::warn!("No path from modify event: {event:?}");
                                        continue;
                                    }
                                };

                                if *modify_path == self.project.get_path() {
                                    match self.project.reload_file() {
                                        Ok(_) => {}
                                        Err(err) => {
                                            log::warn!("Could not reload project info file: {err}")
                                        }
                                    }
                                } else {
                                    let header = match read_file_contents(&modify_path) {
                                        Ok((header, _contents)) => header,
                                        Err(err) => {
                                            log::warn!(
                                                "Could not read modified file: {event:?}: {err}"
                                            );
                                            continue;
                                        }
                                    };

                                    let header_toml = match header.parse::<DocumentMut>() {
                                        Ok(header_toml) => header_toml,
                                        Err(err) => {
                                            log::debug!("Could not read modified file: {err}");
                                            continue;
                                        }
                                    };

                                    let id = match header_toml
                                        .get("id")
                                        .and_then(|val| val.as_str())
                                    {
                                        Some(id) => id,
                                        None => {
                                            log::debug!(
                                                "File event: {event:?} does not contain ID (for modify), skipping"
                                            );
                                            continue;
                                        }
                                    };

                                    if !self.project.objects.contains_key(id) {
                                        log::debug! {"File event: {event:?} contains a key not in the file, skipping"};
                                        continue;
                                    }

                                    run_with_file_object(
                                        id,
                                        &mut self.project.objects,
                                        |file_object, _objects| match file_object.reload_file() {
                                            Ok(()) => {}
                                            Err(err) => {
                                                log::warn!(
                                                    "Error loading file {}: {err}",
                                                    file_object.get_base().metadata.id
                                                );
                                            }
                                        },
                                    )
                                }
                            }
                            EventKind::Remove(_remove_kind) => {
                                // Search for file_objects by looking through all of their
                                // paths, we can't do better.
                                // Might need to update remove_child function to check for
                                // existence before deleting
                            }
                            _ => {}
                        }
                    }
                }
                Err(err) => log::warn!("Error while trying to watch files: {err:?}"),
            }
        }
    }

    fn draw_tree(&mut self, ui: &mut egui::Ui) {
        let mut context_menu_actions: Vec<ContextMenuActions> = Vec::new();
        let (_response, actions) = TreeView::new(ui.make_persistent_id("project tree"))
            .allow_multi_selection(false)
            .show(ui, |builder| {
                self.project.build_tree(builder, &mut context_menu_actions);
            });

        for action in actions {
            match action {
                Action::SetSelected(selected_file_ids) => {
                    // Open nodes when they're selected
                    if let Some(file_id) = selected_file_ids.get(0) {
                        if *file_id != self.project.text_id
                            && *file_id != self.project.characters_id
                            && *file_id != self.project.worldbuilding_id
                        {
                            if let Some(tab_position) = self.dock_state.find_tab(file_id) {
                                // We've already opened this, just select it
                                self.dock_state.set_active_tab(tab_position);
                            } else {
                                // New file object, open it for editing
                                self.dock_state.push_to_first_leaf(file_id.clone());
                            }
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

        for action in context_menu_actions {
            match action {
                ContextMenuActions::Delete { parent, deleting } => {
                    // TODO: find better way of doing this, prune elements before calling the viewer?
                    if let Some(tab_position) = self.dock_state.find_tab(&deleting) {
                        self.dock_state.remove_tab(tab_position);
                    }
                    run_with_file_object(
                        parent.as_str(),
                        &mut self.project.objects,
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
                        &mut self.project.objects,
                        |parent, objects| parent.create_child(file_type, position, objects),
                    ) {
                        Ok(new_child) => self.project.add_object(new_child),
                        Err(err) => {
                            log::error!("Encountered error while trying to add child: {err}")
                        }
                    }
                }
            }
        }
    }

    pub fn new(project: Project, open_tabs: Vec<String>, dictionary: Option<Dictionary>) -> Self {
        // this might later get wrapped in an optional block or something but not worth it right now
        let (mut watcher, file_event_rx) =
            create_watcher().expect("Should always be able to create a watcher");

        watcher
            .watch(&project.get_path(), RecursiveMode::Recursive)
            .unwrap();

        Self {
            project,
            dock_state: DockState::new(open_tabs),
            title_needs_update: true,
            dictionary,
            spellcheck_status: SpellCheckStatus::default(),
            file_event_rx,
            _watcher: watcher,
        }
    }

    pub fn get_open_tabs(&self) -> Vec<String> {
        // the indexes provided to use are meaningless (I think), just put all the tabs in the
        // order it gave us.
        self.dock_state
            .iter_all_tabs()
            .map(|((_, _), tab_id)| (*tab_id).clone())
            .collect()
    }

    pub fn save(&mut self) {
        if let Err(err) = self.project.save() {
            log::error!("encountered error while saving project: {err}");
        }
    }
}
