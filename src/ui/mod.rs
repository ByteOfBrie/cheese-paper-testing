mod base_text_editor;
mod editor_base;
mod file_object_editor;

// File Object specific editors
mod character_editor;
mod folder_editor;
mod place_editor;
mod scene_editor;

pub use base_text_editor::BaseTextEditor;

use character_editor::CharacterEditor;
use folder_editor::FolderEditor;
use place_editor::PlaceEditor;
pub use scene_editor::SceneTextEditor;

pub use editor_base::CheesePaperApp;
