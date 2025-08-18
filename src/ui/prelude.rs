pub use {
    crate::components::{
        file_objects::{FileID, FileObject, FileObjectStore, FileType},
        project::Project,
        text::{Text, TextUID},
    },
    crate::ui::project_editor::{EditorContext, ProjectEditor, Tab},
    egui::{Response, Ui},
    regex::Regex,
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};

pub type SavedRegex = std::sync::LazyLock<Regex>;
