pub mod file_object_editor;
mod file_tree;

use crate::components::Project;
use crate::components::file_objects::base::FileObjectCreation;
use crate::components::file_objects::{FileObject, from_file, run_with_file_object};
use crate::ui::project_tracker::ProjectTracker;
use egui::{Key, Modifiers};
use egui_dock::{DockArea, DockState};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebouncedEvent, Debouncer, RecommendedCache, new_debouncer};
use spellbook::Dictionary;
use std::ops::Range;

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

#[derive(Debug)]
pub struct ProjectEditor {
    pub project: Project,
    dock_state: DockState<String>,
    /// Possibly a temporary hack, need to find a reasonable way to update this when it's change
    /// in the project metadata editor as well
    title_needs_update: bool,
    editor_context: EditorContext,
    file_event_rx: WatcherReceiver,
    /// We don't need to do anything to the watcher, but we stop getting events if it's dropped
    _watcher: RecommendedDebouncer,
    tracker: Option<ProjectTracker>,
}

#[derive(Debug)]
pub struct EditorContext {
    pub dictionary: Option<Dictionary>,
    pub spellcheck_status: SpellCheckStatus,
    pub typing_status: TypingStatus,
}

pub enum TabMove {
    Previous,
    Next,
}

pub struct TabViewer<'a> {
    pub project: &'a mut Project,
    pub editor_context: &'a mut EditorContext,
    pub tab_move: &'a mut Option<TabMove>,
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
        let ctrl_shift_tab = egui::KeyboardShortcut {
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
            logical_key: Key::Tab,
        };

        let ctrl_tab = egui::KeyboardShortcut {
            modifiers: Modifiers::CTRL,
            logical_key: Key::Tab,
        };

        if ui.input_mut(|i| i.consume_shortcut(&ctrl_shift_tab)) {
            *self.tab_move = Some(TabMove::Previous);
        } else if ui.input_mut(|i| i.consume_shortcut(&ctrl_tab)) {
            *self.tab_move = Some(TabMove::Next);
        }

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

        if let Some(file_object) = self.project.objects.get_mut(tab) {
            file_object.as_editor_mut().ui(ui, self.editor_context);
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

fn create_watcher() -> notify::Result<(RecommendedDebouncer, WatcherReceiver)> {
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
                    file_tree::ui(self, ui);
                });
        });

        // Before rendering the tab view, clear out any deleted scenes
        self.dock_state
            .retain_tabs(|tab_id| self.project.objects.contains_key(tab_id));

        let mut tab_move_option: Option<TabMove> = None;

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
                    tab_move: &mut tab_move_option,
                },
            );

        if let Some(tab_move) = tab_move_option {
            let open_tabs: Vec<_> = self.get_open_tabs();

            // Make sure we have something to do
            if open_tabs.len() > 1 {
                if let Some((_, current_tab)) = self.dock_state.find_active_focused() {
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

                    let new_tab_id = open_tabs.get(new_pos).unwrap();

                    self.dock_state
                        .set_active_tab(self.dock_state.find_tab(new_tab_id).unwrap());
                }
            }
        }
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
                                self.process_modify_event(event);
                            }
                            EventKind::Modify(_modify_kind) => {
                                self.process_modify_event(event);
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

        if let Some(tracker) = &mut self.tracker
            && tracker.snapshot_time.elapsed().as_secs() >= 60 * 15
        {
            if let Err(err) = tracker.snapshot("Autosave") {
                log::warn!("Failed to track changes: {err}");
            }
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
                    // We expect a bunch of git events, but other events are unexpected
                    log::debug!(
                        "invalid modify/create path not in project folders: {modify_path:?}"
                    );
                }
                return;
            }

            match self.project.find_object_by_path(modify_path) {
                Some(id) => {
                    run_with_file_object(&id, &mut self.project.objects, |file_object, _objects| {
                        if let Err(err) = file_object.reload_file() {
                            log::warn!(
                                "Error loading file {}: {err}",
                                file_object.get_base().metadata.id
                            );
                        }
                    })
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

                        let parent_object = self.project.objects.get_mut(&parent_id).unwrap();

                        let new_index = parent_object.get_base().children.len();

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

                        let (new_object, descendents): (Box<dyn FileObject>, _) = match new_object {
                            FileObjectCreation::Scene(parent, children) => {
                                (Box::new(parent), children)
                            }
                            FileObjectCreation::Character(parent, children) => {
                                (Box::new(parent), children)
                            }
                            FileObjectCreation::Folder(parent, children) => {
                                (Box::new(parent), children)
                            }
                            FileObjectCreation::Place(parent, children) => {
                                (Box::new(parent), children)
                            }
                        };

                        // Add to the parent's list of children
                        parent_object
                            .get_base_mut()
                            .children
                            .push(new_object.get_base().metadata.id.clone());

                        // Add the parent object to the object list
                        self.project
                            .objects
                            .insert(new_object.get_base().metadata.id.clone(), new_object);

                        // Add all of the descendents to the list
                        for (id_string, object) in descendents {
                            self.project.objects.insert(id_string, object);
                        }
                    }
                }
            };
        }
    }

    // fn draw_tree(&mut self, ui: &mut egui::Ui)

    pub fn new(project: Project, open_tabs: Vec<String>, dictionary: Option<Dictionary>) -> Self {
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

        Self {
            project,
            dock_state: DockState::new(open_tabs),
            title_needs_update: true,
            editor_context: EditorContext {
                dictionary,
                spellcheck_status: SpellCheckStatus::default(),
                typing_status: TypingStatus::default(),
            },
            file_event_rx,
            _watcher: watcher,
            tracker,
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
