use egui::{FontFamily, FontId, TextStyle};
use egui_ltreeview::TreeView;
use std::time::{Duration, SystemTime};

use crate::ui::project_editor::ProjectEditor;
use crate::ui::{CharacterEditor, FolderEditor, PlaceEditor, SceneEditor};

use crate::components::Project;
use crate::ui::file_object_editor::FileObjectEditorType;

pub enum FileEditor<'a> {
    Scene(SceneEditor<'a>),
    Character(CharacterEditor<'a>),
    Folder(FolderEditor<'a>),
    Place(PlaceEditor<'a>),
}

pub struct CheesePaperApp {
    pub project_editor: ProjectEditor,
    last_write: SystemTime,
}

impl eframe::App for CheesePaperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.project_editor.panels(ctx);

        let current_time = SystemTime::now();
        if current_time.duration_since(self.last_write).unwrap() > Duration::from_secs(5) {
            println!("Writing at: {:#?}", current_time);
            self.last_write = current_time;
        }
    }
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::{Monospace, Proportional};

    // TODO: when configs are read, scale all of these off of the configured font size
    let text_styles: std::collections::BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::new(28.0, Proportional)),
        (TextStyle::Body, FontId::new(24.0, Proportional)),
        (TextStyle::Monospace, FontId::new(24.0, Monospace)),
        (TextStyle::Button, FontId::new(20.0, Proportional)),
        (TextStyle::Small, FontId::new(20.0, Proportional)),
    ]
    .into();

    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}

impl CheesePaperApp {
    pub fn new(cc: &eframe::CreationContext<'_>, project: Project) -> Self {
        configure_text_styles(&cc.egui_ctx);

        Self {
            project_editor: ProjectEditor { project },
            last_write: SystemTime::now(),
        }
    }
}
