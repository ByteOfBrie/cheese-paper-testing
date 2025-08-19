mod render_data;

mod editor_base;
mod settings;
mod text_box;

mod project_editor;
mod project_tracker;

mod prelude;

pub use editor_base::CheesePaperApp;
pub use project_editor::page::FileObjectEditor;

#[cfg(feature = "metrics")]
mod metrics;
