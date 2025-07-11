mod base_text_editor;
mod editor_base;
mod file_object_editor;

// File Object specific editors
mod character_editor;
mod folder_editor;
mod place_editor;
mod scene_editor;

mod project_editor;

use base_text_editor::BaseTextEditor;

use character_editor::CharacterEditor;
use folder_editor::FolderEditor;
use place_editor::PlaceEditor;
use scene_editor::SceneEditor;

pub use editor_base::CheesePaperApp;
