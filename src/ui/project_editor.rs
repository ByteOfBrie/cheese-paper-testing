mod file_tree;
pub mod page;
pub mod search;
mod util;

use crate::ui::prelude::*;

use crate::components::file_objects::{
    base::FileObjectCreation, from_file, utils::process_name_for_filename,
};
use crate::ui::editor_base::EditorState;
use crate::ui::project_editor::search::global_search;
use crate::ui::project_tracker::ProjectTracker;

use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::path::PathBuf;

use egui::{Key, Modifiers};
use egui_dock::{DockArea, DockState};
use egui_ltreeview::TreeViewState;
use notify::event::RenameMode;
use notify::{EventKind, event::ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebouncedEvent, Debouncer, RecommendedCache, new_debouncer};
use rfd::FileDialog;
use spellbook::Dictionary;

type RecommendedDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;
type WatcherReceiver = std::sync::mpsc::Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>;

#[derive(Debug, Default)]
pub struct SpellCheckStatus {
    pub selected_word: String,
    pub correct: bool,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TypingStatus {
    pub is_new_word: bool,
    pub current_word: Range<usize>,
}

pub struct ProjectEditor {
    pub project: Project,

    /// List of tabs that are open (egui::Dock requires state to be stored this way)
    dock_state: DockState<Page>,

    /// Possibly a temporary hack, need to find a reasonable way to update this when it's change
    /// in the project metadata editor as well
    title_needs_update: bool,

    pub editor_context: EditorContext,

    file_event_rx: WatcherReceiver,

    /// We don't need to do anything to the watcher, but we stop getting events if it's dropped
    _watcher: RecommendedDebouncer,
    tracker: Option<ProjectTracker>,

    /// We need to keep track of the tree state to set selection
    tree_state: TreeViewState<Page>,

    /// Set by the tab viewer, used to sync the file tree
    current_open_tab: Option<Page>,
}

impl Debug for ProjectEditor {
    /// Manual implementation because TreeViewState doesn't implement debug
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectEditor")
            .field("project", &self.project)
            .field("dock_state", &self.dock_state)
            .field("title_needs_update", &self.title_needs_update)
            .field("editor_context", &self.editor_context)
            .field("file_event_rx", &self.file_event_rx)
            .field("_watcher", &self._watcher)
            .field("tracker", &self.tracker)
            .finish()
    }
}

#[derive(Debug)]
pub struct EditorContext {
    pub settings: Settings,
    pub dictionary: Option<Dictionary>,
    pub spellcheck_status: SpellCheckStatus,
    pub typing_status: TypingStatus,
    pub search: Search,
    pub stores: Stores,

    /// Duplicates the value from state.data, which is then more recent
    pub last_export_folder: PathBuf,

    /// version number. increment to trigger a project-wide formatting refresh
    pub version: usize,
}

#[derive(Debug, Default)]
pub struct Stores {
    pub text_box: crate::ui::text_box::Store,
    pub page: page::Store,
    pub scene: page::file_object_editor::scene_editor::Store,
    pub folder: page::file_object_editor::folder_editor::Store,
}

pub enum TabMove {
    Previous,
    Next,
}

pub struct TabViewer<'a> {
    pub project: &'a mut Project,
    pub editor_context: &'a mut EditorContext,
    pub open_tab: &'a mut Option<Page>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Page;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        tab.into()
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Page::ProjectMetadata => "Project Metadata".into(),
            Page::FileObject(file_id) => {
                if let Some(object) = self.project.objects.get(file_id) {
                    object.borrow().get_title().into()
                } else {
                    // any deleted scenes should be cleaned up before we get here, but we have this
                    // logic instead of panicking anyway
                    "<Deleted>".into()
                }
            }
            Page::Export => "Export".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // Tell the editor which tab we have open (so that the treeview selection can be updated)
        if self.open_tab.as_ref() != Some(tab) {
            *self.open_tab = Some(tab.clone());
        }

        // check for ctrl-shift-f for search
        if ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut {
                modifiers: Modifiers::CTRL | Modifiers::SHIFT,
                logical_key: Key::F,
            })
        }) {
            self.editor_context.search.show();
        }

        // lock tab presses to the current window
        if let Some(focused_tab) = ui.memory(|i| i.focused()) {
            ui.memory_mut(|i| {
                i.set_focus_lock_filter(
                    focused_tab,
                    egui::EventFilter {
                        tab: true,
                        ..Default::default()
                    },
                );
            });
        }

        if ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab)) {
            println!("pressed shift-tab");
        } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
            println!("pressed tab!");
        }

        // draw the actual UI for the tab open in the editor
        tab.ui(ui, self.project, self.editor_context);
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        // disable moving tabs into windows (untested, could maybe be supported later)
        false
    }
}

fn create_watcher() -> notify::Result<(RecommendedDebouncer, WatcherReceiver)> {
    let (tx, rx) = std::sync::mpsc::channel();

    let watcher = new_debouncer(std::time::Duration::from_secs(2), None, tx)?;

    Ok((watcher, rx))
}

/// Update the title of the project
fn update_title(project_name: &str, ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
        "Cheese Paper - {project_name}",
    )));
}

impl ProjectEditor {
    pub fn panels(&mut self, ctx: &egui::Context, state: &mut EditorState) {
        self.process_input(ctx);
        self.process_state(ctx);

        self.draw_menu(ctx, state);

        egui::SidePanel::left("project tree panel").show(ctx, |ui| {
            self.side_panel(ui);
        });

        // Before rendering the tab view, clear out any deleted scenes
        self.dock_state.retain_tabs(|tab| match tab {
            Page::ProjectMetadata => true,
            Page::Export => true,
            Page::FileObject(tab_id) => self.project.objects.contains_key(tab_id),
        });

        // render the tab view
        DockArea::new(&mut self.dock_state)
            .allowed_splits(egui_dock::AllowedSplits::None)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show(
                ctx,
                &mut TabViewer {
                    project: &mut self.project,
                    editor_context: &mut self.editor_context,
                    open_tab: &mut self.current_open_tab,
                },
            );

        // If there aren't any tabs open, reflect that state
        if self.dock_state.iter_all_tabs().next().is_none() {
            self.current_open_tab = None
        }

        if self.current_open_tab.as_ref() != self.tree_state.selected().first()
            && let Some(open_tab) = &self.current_open_tab
        {
            self.tree_state.set_one_selected(open_tab.clone());
        }
    }

    /// Get input that the project editor itself will read (hotkeys to switch or close tabs)
    fn process_input(&mut self, ctx: &egui::Context) {
        // close current tab if ctrl-w is pressed
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::W,
            })
        }) && let Some((_, current_tab_ref)) = self.dock_state.find_active_focused()
        {
            // We get an &mut reference so we have to clone it ;)
            let current_tab = current_tab_ref.clone();
            let tab_position = self.dock_state.find_tab(&current_tab).unwrap();
            self.dock_state.remove_tab(tab_position);
        }

        // Move between tabs (ctrl-tab or ctrl-shift-tab)
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut {
                modifiers: Modifiers::CTRL | Modifiers::SHIFT,
                logical_key: Key::Tab,
            })
        }) {
            // ctrl-shift-tab was pressed, move backwards
            self.move_tab(TabMove::Previous)
        } else if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::Tab,
            })
        }) {
            // ctrl-tab was pressed, move fowards
            self.move_tab(TabMove::Next)
        }
    }

    fn move_tab(&mut self, tab_move: TabMove) {
        // We could probably get around this by learning how dock_state works better, but
        // this is easy and reliable
        let open_tabs: Vec<_> = self.get_open_tabs();

        // Make sure we have something to do
        if open_tabs.len() > 1
            && let Some((_, current_tab)) = self.dock_state.find_active_focused()
        {
            let current_pos = open_tabs
                .iter()
                .position(|val| val == current_tab)
                .expect("focused tab should be in list of tabs");

            let new_pos = match tab_move {
                TabMove::Next => (current_pos + 1) % open_tabs.len(),
                TabMove::Previous => current_pos
                    .checked_sub(1)
                    .unwrap_or_else(|| open_tabs.len() - 1),
            };

            self.set_editor_tab(open_tabs.get(new_pos).unwrap());
        }
    }

    fn draw_menu(&mut self, ctx: &egui::Context, state: &mut EditorState) {
        egui::TopBottomPanel::top("menu_bar_panel")
            .show_separator_line(false)
            .show(ctx, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Close Project").clicked() {
                            state.closing_project = true;
                        }

                        ui.menu_button("Recent Projects", |ui| {
                            for project in state.data.recent_projects.iter() {
                                if ui.button(project.to_string_lossy()).clicked() {
                                    state.closing_project = true;
                                    state.next_project = Some(project.clone());
                                }
                            }
                        });

                        if ui.button("Export Story Text").clicked() {
                            self.set_editor_tab(&Page::Export);
                        }

                        if ui.button("Export Outline").clicked() {
                            let project_title = &self.project.base_metadata.name;
                            let suggested_title =
                                format!("{}_outline.md", process_name_for_filename(project_title));
                            let export_location_option = FileDialog::new()
                                .set_title(format!("Export {project_title} Outline"))
                                .set_directory(&state.data.last_export_folder)
                                .set_file_name(suggested_title)
                                .save_file();

                            if let Some(export_location) = export_location_option {
                                let outline_contents = self.project.export_outline();
                                if let Err(err) = std::fs::write(&export_location, outline_contents)
                                {
                                    log::error!("Error while attempting to write outline: {err}");
                                }

                                state.data.last_export_folder = export_location
                                    .parent()
                                    .map(|val| val.to_path_buf())
                                    .unwrap_or_default();
                            }
                        }

                        if ui.button("Quit").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button("Edit", |ui| {
                        if ui.button("Find (Global)").clicked() {
                            self.editor_context.search.show();
                        }
                    });
                });
            });
    }

    // the side panel containing the tree view or the global search
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        if self.editor_context.search.active {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.editor_context.search.hide();
            }
            egui::ScrollArea::vertical()
                .id_salt("search scroll")
                .max_height(ui.available_height())
                .show(ui, |ui| {
                    global_search::ui(ui, &self.project, &mut self.editor_context);
                });
        } else {
            egui::ScrollArea::both()
                .id_salt("tree scroll")
                .max_height(ui.available_height())
                .show(ui, |ui| {
                    file_tree::ui(self, ui);
                });
        }
    }

    fn process_state(&mut self, ctx: &egui::Context) {
        // update window title. silly that we have to do it here, but we can't set it when calling new()
        // since we don't have the `egui::Context`. This will also need to happen once we can actually
        // set project names
        if self.title_needs_update {
            update_title(&self.project.base_metadata.name, ctx);
            self.title_needs_update = false;
        }

        if self.editor_context.search.exiting_search {
            self.editor_context.version += 1;
        }

        // check for file system events and process them
        if let Ok(response) = self.file_event_rx.try_recv() {
            match response {
                Ok(events) => {
                    let mut file_objects_needing_rescan = HashSet::new();
                    let mut found_events = false;
                    for event in events {
                        let mut git_event = false;
                        for event_path in event.paths.iter() {
                            if event_path.iter().any(|component| component == ".git") {
                                git_event = true;
                            }
                        }
                        if git_event {
                            continue;
                        }
                        if let EventKind::Access(_) = event.kind {
                            continue;
                        }

                        // We now have an event that isn't noise from .git or file opens:
                        found_events = true;
                        log::debug!("found event: {event:?}");

                        match event.kind {
                            EventKind::Create(_create_kind) => {
                                self.process_modify_event(event);
                            }
                            EventKind::Modify(ModifyKind::Data(_data_change)) => {
                                self.process_modify_event(event);
                            }
                            EventKind::Modify(ModifyKind::Name(rename_mode)) => {
                                if let Some(need_rescan_vec) =
                                    self.process_rename_event(event, rename_mode)
                                {
                                    for need_rescan_id in need_rescan_vec {
                                        file_objects_needing_rescan.insert(need_rescan_id);
                                    }
                                }
                            }
                            EventKind::Remove(_remove_kind) => {
                                // Search for file_objects by looking through all of their
                                // paths, we can't do better.
                                // Might need to update remove_child function to check for
                                // existence before deleting

                                log::debug!("not (yet) processing deletion event: {event:?}");
                            }
                            _ => {}
                        }
                    }
                    for object_needing_rescan in file_objects_needing_rescan {
                        self.project
                            .objects
                            .get(&object_needing_rescan)
                            .unwrap()
                            .borrow_mut()
                            .rescan_indexing(&self.project.objects);
                    }
                    if found_events {
                        log::debug!("finished processing events");
                    }
                }
                Err(err) => log::warn!("Error while trying to watch files: {err:?}"),
            }
        }

        // automatically track progerss if we have a tracker
        if let Some(tracker) = &mut self.tracker
            && tracker.snapshot_time.elapsed().as_secs() >= 60 * 15
            && let Err(err) = tracker.snapshot("Autosave")
        {
            log::warn!("Failed to track changes: {err}");
        }

        if self.editor_context.search.redo_search {
            self.editor_context.search.redo_search = false;
            self.search();
        }

        // if one of the search results has been clicked, open that now
        // BY THE POWER OF IF LET CHAINS
        if self.editor_context.search.goto_focus
            && let Some((uid, _word_find)) = &self.editor_context.search.focus.as_ref()
            && let Some(search_results) = &self.editor_context.search.search_results.as_ref()
            && let Some(focused_text_box) = search_results.get(uid)
        {
            self.set_editor_tab(&focused_text_box.page.clone());
        }
    }

    /// `event` has to be modification, try to figure out the file and reload it if
    /// it's truly part of the project
    fn process_modify_event(&mut self, event: DebouncedEvent) {
        // Try to read the file, if it has an ID, look up that ID
        // and call reload file, otherwise give up (it might come in as
        // a different event, but we don't care about modifications
        // to files we don't know)
        let modify_path = match event.paths.first() {
            Some(path) => path,
            None => {
                log::warn!("No path from modify event: {event:?}");
                return;
            }
        };

        if modify_path.ends_with(".tmp") {
            // we write .tmp files and then immediately remove them, ignore this file
            return;
        }

        if !modify_path.exists() {
            log::debug!(
                "Attempted to process modification of a file that no longer exists: {modify_path:?}"
            );
            return;
        }

        if *modify_path == self.project.get_project_info_file() {
            match self.project.reload_file() {
                Ok(_) => {}
                Err(err) => {
                    log::warn!("Could not reload project info file: {err}")
                }
            }
        } else {
            let relative_path = match modify_path.strip_prefix(self.project.get_path()) {
                Ok(relative_path) => relative_path,
                Err(err) => {
                    log::error!("invalid modify/create path not in project: {err}");
                    return;
                }
            };

            if !(relative_path.starts_with("text")
                || relative_path.starts_with("characters")
                || relative_path.starts_with("worldbuilding"))
            {
                if !relative_path.starts_with(".git") {
                    // We expect a bunch of git events, but other events are unexpected, so log it
                    log::debug!(
                        "invalid modify/create path not in project folders: {modify_path:?}"
                    );
                }
                return;
            }

            log::debug!("processing modify events with path: {modify_path:?}");

            match self.project.find_object_by_path(modify_path) {
                Some(id) => {
                    let file_object = self.project.objects.get(&id).unwrap();
                    if let Err(err) = file_object.borrow_mut().reload_file() {
                        log::warn!("Error loading file {}: {err}", file_object.borrow());
                    }
                }
                None => {
                    let ancestors = modify_path.ancestors();

                    for ancestor in ancestors {
                        // We need to check if this object can be loaded, which means
                        // that its parent is already in the tree

                        let parent_path = match ancestor.parent() {
                            Some(parent) => parent,
                            None => {
                                log::error!(
                                    "unexpected result while processing event: {event:?}\
                                    parents should exist and the loop should always \
                                    finish before it escapes the project tree",
                                );
                                return;
                            }
                        };

                        let parent_id = match self.project.find_object_by_path(parent_path) {
                            Some(id) => id,
                            None => continue,
                        };

                        let parent_object = self.project.objects.get(&parent_id).unwrap();

                        let new_index = parent_object.borrow_mut().get_base().children.len();

                        // We've found a parent, which means that this object should
                        // have from_file called on it
                        let new_object = match from_file(ancestor, Some(new_index)) {
                            Ok(file_object_creation) => file_object_creation,
                            Err(err) => {
                                log::warn!(
                                    "Could not open file as part of processing modifications: {err}"
                                );
                                log::warn!("Giving up on processing event: {event:?}");
                                return;
                            }
                        };

                        let (new_object, descendents): (Box<RefCell<dyn FileObject>>, _) =
                            match new_object {
                                FileObjectCreation::Scene(parent, children) => {
                                    (Box::new(RefCell::new(parent)), children)
                                }
                                FileObjectCreation::Character(parent, children) => {
                                    (Box::new(RefCell::new(parent)), children)
                                }
                                FileObjectCreation::Folder(parent, children) => {
                                    (Box::new(RefCell::new(parent)), children)
                                }
                                FileObjectCreation::Place(parent, children) => {
                                    (Box::new(RefCell::new(parent)), children)
                                }
                            };

                        let id = new_object.borrow().id().clone();

                        // Add to the parent's list of children
                        parent_object
                            .borrow_mut()
                            .get_base_mut()
                            .children
                            .push(id.clone());

                        // Add the parent object to the object list
                        self.project.objects.insert(id, new_object);

                        // Add all of the descendents to the list
                        for (id_string, object) in descendents {
                            self.project.objects.insert(id_string, object);
                        }
                    }
                }
            };
        }
    }

    /// Processes rename events as best-effort, currently cannot handle complex cases well
    ///
    /// Returns a list of file objects that need to be rescanned for indexing
    fn process_rename_event(
        &mut self,
        event: DebouncedEvent,
        rename_mode: RenameMode,
    ) -> Option<Vec<FileID>> {
        if rename_mode != RenameMode::Both {
            // Give up, we don't want to make assumptoins at this stage
            //
            // Long term, we can probably do better. If it's only "from", we can check for that
            // file and treat it like a deletion. If it's just a "from", we can treat it like a
            // deletion. If it's just a "to", we can check for file IDs (in which case we know the
            // old path), and otherwise treat it like a creation
            log::warn!(
                "Processed rename event: {event:?}, but it does not contain both source and\
                destination, cannot meaningfully continue processing"
            );
            return None;
        }

        let source_path = event
            .paths
            .first()
            .expect("Rename event should have source");

        let dest_path = event
            .paths
            .last()
            .expect("Rename event should have destination");

        if source_path == dest_path {
            log::debug!("Rename event: {event:?} has the same source and dest, nothing to do");
            return None;
        }

        let moving_file_id = match self.project.find_object_by_path(source_path) {
            Some(fileid) => fileid,
            None => {
                log::debug!("Processed file rename for object with unknown source path: {event:?}");
                return None;
            }
        };
        let dest_name = dest_path.file_name().expect("dest should have a file name");

        let source_directory = source_path
            .parent()
            .expect("source should have a directory");
        let dest_directory = source_path.parent().expect("dest should have a directory");

        let source_parent_file_id = match self.project.find_object_by_path(source_directory) {
            Some(source_parent_id) => source_parent_id,
            None => {
                log::error!(
                    "Tried to move object but could not find it's parent: {source_directory:?}. \
                    Event: {event:?}"
                );
                return None;
            }
        };

        // Easy case: the file has been renamed within the directory it's in
        if source_directory == dest_directory {
            let mut object = self
                .project
                .objects
                .get(&moving_file_id)
                .unwrap()
                .borrow_mut();

            // Update the filename
            object.get_base_mut().file.basename = dest_name.to_owned();
            // propagate that to any children
            for child in object.children(&self.project.objects) {
                child
                    .borrow_mut()
                    .process_path_update(object.get_path(), &self.project.objects);
            }
            // Currently, we don't do anything to cleanup the directory or filename in this case.
            // It'll probably happen later, but we don't bother now (this is complicated enough already)
            return Some(vec![source_parent_file_id]);
        }

        // More complicated case: the file has been moved to another part of the tree. We're basically
        // processing a move, but without the actual move. This should probably be cleanup up later
        let dest_file_id = match self.project.find_object_by_path(dest_directory) {
            Some(dest_file_id) => dest_file_id,
            None => {
                log::warn!(
                    "File object is now in an unknown directory: {dest_directory:?}, \
                    unable to continue processing move/rename. Event: {event:?}"
                );
                return None;
            }
        };

        // Remove the moving object from it's current parent
        let source_parent = self
            .project
            .objects
            .get(&source_parent_file_id)
            .expect("objects should contain source file id");

        let child_id_position = source_parent
            .borrow()
            .get_base()
            .children
            .iter()
            .position(|val| moving_file_id == *val)
            .unwrap_or_else(|| {
                panic!(
                    "Children should only be removed from their parents.\
                child id: {moving_file_id}, parent: {source_parent_file_id}"
                )
            });

        let child_id_string = source_parent
            .borrow_mut()
            .get_base_mut()
            .children
            .remove(child_id_position);

        let dest_parent = self.project.objects.get(&dest_file_id).unwrap();

        // How do I find the proper place here?
        // Move the object into the children of dest (at the proper place)
        dest_parent
            .borrow_mut()
            .get_base_mut()
            .children
            .push(child_id_string);

        let child = self.project.objects.get(&moving_file_id).unwrap();

        child
            .borrow_mut()
            .process_path_update(dest_directory.to_path_buf(), &self.project.objects);

        Some(vec![source_parent_file_id, dest_file_id])
    }

    fn set_editor_tab(&mut self, tab: &Page) {
        // We don't want to open these, so just exit early
        if let Page::FileObject(id) = tab
            && (*id == self.project.text_id
                || *id == self.project.characters_id
                || *id == self.project.worldbuilding_id)
        {
            return;
        }

        if let Some(tab_position) = self.dock_state.find_tab(tab) {
            // We've already opened this, just select it
            self.dock_state.set_active_tab(tab_position);
        } else {
            // New file object, open it for editing
            self.dock_state.push_to_first_leaf(tab.clone());
        }
    }

    // last_export_folder probably should be wrapped in another object but I don't have a good object
    // to wrap it in, so it's here for now
    pub fn new(
        project: Project,
        open_tab_ids: Vec<String>,
        dictionary: Option<Dictionary>,
        settings: Settings,
        last_export_folder: PathBuf,
    ) -> Self {
        // this might later get wrapped in an optional block or something but not worth it right now
        let (mut watcher, file_event_rx) =
            create_watcher().expect("Should always be able to create a watcher");

        watcher
            .watch(project.get_path(), RecursiveMode::Recursive)
            .unwrap();

        let tracker = match ProjectTracker::new(&project.get_path()) {
            Ok(mut tracker) => {
                if let Err(err) = tracker.snapshot("Startup") {
                    log::warn!("Failed to snapshot tracker: {err}");
                };
                Some(tracker)
            }
            Err(err) => {
                log::warn!("failed to create project tracker: {err}");
                None
            }
        };

        let open_tabs = open_tab_ids
            .iter()
            .map(|tab_id| Page::from_id(tab_id))
            .collect();

        Self {
            project,
            dock_state: DockState::new(open_tabs),
            title_needs_update: true,
            editor_context: EditorContext {
                settings,
                dictionary,
                spellcheck_status: SpellCheckStatus::default(),
                typing_status: TypingStatus::default(),
                search: Search::default(),
                stores: Stores::default(),
                last_export_folder,
                version: 0,
            },
            file_event_rx,
            _watcher: watcher,
            tracker,
            tree_state: Default::default(),
            current_open_tab: None,
        }
    }

    pub fn get_open_tabs(&self) -> Vec<Page> {
        // the indexes provided to use are meaningless (I think), just put all the tabs in the
        // order it gave us.
        self.dock_state
            .iter_all_tabs()
            .map(|((_, _), tab)| tab.clone())
            .collect()
    }

    pub fn save(&mut self) {
        if let Err(err) = self.project.save() {
            log::error!("encountered error while saving project: {err}");
        }
    }
}
