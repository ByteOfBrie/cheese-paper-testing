use egui::{FontFamily, FontId, TextStyle};
use std::time::{Duration, Instant};

use crate::ui::project_editor::ProjectEditor;

use crate::components::Project;

pub struct CheesePaperApp {
    pub project_editor: ProjectEditor,

    /// Time for autosaves
    ///
    ///  Shockingly, it actually makes some amount of sense to keep the logic here (instead of in
    ///`ProjectEditor`), since we'll eventually want to save editor configs as well, and it's better
    /// to propagate the event downwards
    last_save: Instant,
}

impl eframe::App for CheesePaperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.project_editor.panels(ctx);

        let current_time = Instant::now();
        if current_time.duration_since(self.last_save) > Duration::from_secs(5) {
            self.project_editor.save();
            self.last_save = current_time;
        }
    }
}

impl Drop for CheesePaperApp {
    fn drop(&mut self) {
        self.project_editor.save();
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
            project_editor: ProjectEditor::new(project),
            last_save: Instant::now(),
        }
    }
}
