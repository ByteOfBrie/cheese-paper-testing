mod base_text_editor;
mod editor_base;
mod file_object_editor;
mod tiny_markdown_highlighter;

// File Object specific editors
mod character_editor;
mod folder_editor;
mod place_editor;
mod scene_editor;

mod project_editor;

use base_text_editor::BaseTextEditor;

pub use editor_base::CheesePaperApp;
pub use file_object_editor::FileObjectEditor;
pub use project_editor::EditorContext;
