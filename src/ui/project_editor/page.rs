mod file_object_editor;
mod project_metadata_editor;

pub use file_object_editor::FileObjectEditor;

use crate::ui::prelude::*;

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
}

impl Page {
    const PROJECT_METADATA_ID: &str = "project_metadata";

    /// Get an id from a string. This (and its reverse, `get_id`) could be replaced by `From`
    /// (and `Into`), but this seems like it might be more explicit?
    pub fn from_id(id: &str) -> Self {
        match id {
            Self::PROJECT_METADATA_ID => Page::ProjectMetadata,
            _ => Page::FileObject(FileID::new(id.to_owned())),
        }
    }

    pub fn get_id(&self) -> &str {
        match self {
            Page::ProjectMetadata => Self::PROJECT_METADATA_ID,
            Page::FileObject(id) => id,
        }
    }

    pub fn from_file_id(file_id: &FileID) -> Self {
        Page::FileObject(file_id.clone())
    }

    pub fn is_file_object(&self) -> bool {
        matches!(self, Page::FileObject(_))
    }

    pub fn ui(&self, ui: &mut Ui, project: &mut Project, ctx: &mut EditorContext) {
        match self {
            Page::ProjectMetadata => {
                project.metadata_ui(ui, ctx);
            }
            Page::FileObject(file_object_id) => {
                if let Some(file_object) = project.objects.get(file_object_id) {
                    file_object.borrow_mut().as_editor_mut().ui(ui, ctx);
                }
            }
        }
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
            Self::PROJECT_METADATA_ID => Page::ProjectMetadata,
            _ => Page::FileObject(id),
        }
    }
}
