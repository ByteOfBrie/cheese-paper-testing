pub use {
    crate::cheese_error,
    crate::components::{
        file_objects::{FileID, FileObject, FileObjectStore, FileType},
        project::Project,
        text::{Text, TextUID},
    },
    crate::ui::{
        project_editor::page::Page,
        project_editor::{EditorContext, ProjectEditor, search::Search},
        settings::Settings,
    },
    crate::util::CheeseError,
    egui::{Response, Ui},
    regex::Regex,
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};

pub type SavedRegex = std::sync::LazyLock<Regex>;
