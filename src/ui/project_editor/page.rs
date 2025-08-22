mod export_selection;
mod file_object_editor;
mod project_metadata_editor;

use crate::ui::prelude::*;

pub use file_object_editor::FileObjectEditor;

use egui::{Key, Modifiers};

/// An identifier for something that can be drawn as a tab
///
/// We currently have to have a string representation for every tab value so that
/// `update_open_tabs` can write the list of open tabs for them to be reopened next
/// time. If that requirement isn't present, we should be able to avoid having strings
/// entirely
#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub enum Page {
    ProjectMetadata,
    FileObject(FileID),
    Export,
}

impl Page {
    const PROJECT_METADATA_ID: &str = "project_metadata";
    const EXPORT_ID: &str = "export";

    /// Get an id from a string. This (and its reverse, `get_id`) could be replaced by `From`
    /// (and `Into`), but this seems like it might be more explicit?
    pub fn from_id(id: &str) -> Self {
        match id {
            Self::PROJECT_METADATA_ID => Self::ProjectMetadata,
            Self::EXPORT_ID => Self::Export,
            _ => Self::FileObject(FileID::new(id.to_owned())),
        }
    }

    pub fn get_id(&self) -> &str {
        match self {
            Self::ProjectMetadata => Self::PROJECT_METADATA_ID,
            Self::Export => Self::EXPORT_ID,
            Self::FileObject(id) => id,
        }
    }

    pub fn from_file_id(file_id: &FileID) -> Self {
        Self::FileObject(file_id.clone())
    }

    pub fn is_file_object(&self) -> bool {
        matches!(self, Self::FileObject(_))
    }

    pub fn is_searchable(&self) -> bool {
        match self {
            Self::Export => false,
            Self::FileObject(_) => true,
            Self::ProjectMetadata => true,
        }
    }
}

#[derive(Debug, Default)]
pub struct PageData {
    search: Search,
}

pub type Store = RenderDataStore<Page, PageData>;

impl Page {
    pub fn ui(&self, ui: &mut Ui, project: &mut Project, ctx: &mut EditorContext) {
        let rdata = ctx.stores.page.get(self);
        let page_data: &mut PageData = &mut rdata.borrow_mut();

        let page_search_active = if self.is_searchable() {
            self.process_page_search(page_data, ui, project, ctx)
        } else {
            false
        };

        match self {
            Self::ProjectMetadata => {
                project.metadata_ui(ui, ctx);
            }
            Self::FileObject(file_object_id) => {
                if let Some(file_object) = project.objects.get(file_object_id) {
                    file_object.borrow_mut().as_editor_mut().ui(ui, ctx);
                }
            }
            Self::Export => {
                project.export_ui(ui, ctx);
            }
        }

        // If this was swapped once, we need to put it back
        if page_search_active {
            std::mem::swap(&mut ctx.search, &mut page_data.search);
        }
    }

    /// Handle page search logic, including
    fn process_page_search(
        &self,
        page_data: &mut PageData,
        ui: &mut Ui,
        project: &mut Project,
        ctx: &mut EditorContext,
    ) -> bool {
        // check for ctrl-f for page search
        if ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::F,
            })
        }) {
            page_data.search.show();
        }

        if page_data.search.active {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut page_data.search.find_text)
                        .hint_text("find")
                        .return_key(None), // keep focus when Enter is pressed)
                );
                ui.add(
                    egui::TextEdit::singleline(&mut page_data.search.replace_text)
                        .hint_text("replace")
                        .return_key(None),
                );
                if ui.button("search").clicked() {
                    page_data.search.redo_search = true;
                }
                if ui.button("close").clicked() {
                    page_data.search.active = false;
                    ctx.version += 1;
                }
            });

            if page_data.search.redo_search {
                page_data.search.search_results = Some(HashMap::new());

                if let Some(searchable) = project.get_searchable(self) {
                    searchable.search(self, &mut page_data.search);
                }

                ctx.version += 1;
                page_data.search.redo_search = false;
            }
        }

        let page_search_active = page_data.search.active;
        if page_search_active {
            /* Hack-y solution: swap in the file search object for the file-local search */
            std::mem::swap(&mut ctx.search, &mut page_data.search);
        }
        page_search_active
    }
}

// Needs to be &mut Tab since `egui_dock::TabViewer::id` gives us a mut reference
impl From<&mut Page> for egui::Id {
    fn from(val: &mut Page) -> Self {
        egui::Id::new(val)
    }
}

impl From<Rc<String>> for Page {
    fn from(id: Rc<String>) -> Self {
        match id.as_str() {
            Self::PROJECT_METADATA_ID => Self::ProjectMetadata,
            Self::EXPORT_ID => Self::Export,
            _ => Self::FileObject(id),
        }
    }
}
