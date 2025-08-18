use egui::{Color32, Response};

use crate::ui::prelude::*;

use super::textbox_search::TextBoxSearchResult;
use crate::ui::project_editor::search::textbox_search::WordFind;

#[derive(Debug, Default)]
pub struct GlobalSearch {
    pub active: bool,

    /// Search has just been activated, take focus
    pub request_ui_focus: bool,

    pub find_text: String,

    pub replace_text: String,

    pub redo_search: bool,

    pub search_results: Option<HashMap<TextUID, TextBoxSearchResult>>,

    pub focus: Option<(TextUID, WordFind)>,

    pub goto_focus: bool,

    pub version: usize,
}

impl GlobalSearch {
    pub fn show(&mut self) {
        self.active = true;
        self.request_ui_focus = true;
    }

    pub fn hide(&mut self) {
        self.active = false;
        // TODO: #85: do something about highlighters here
    }

    pub fn clear_focus(&mut self) {
        self.focus = None;
        self.goto_focus = false;
    }
}

/// Global search ui, returns
pub fn ui(ui: &mut Ui, project: &Project, ctx: &mut EditorContext) -> Response {
    let gs = &mut ctx.global_search;

    // Take up the entire area horizontally
    let response = ui.add_sized(
        egui::vec2(ui.available_width(), 0.0),
        egui::TextEdit::singleline(&mut gs.find_text).hint_text("find"),
    );

    ui.add_sized(
        egui::vec2(ui.available_width(), 0.0),
        egui::TextEdit::singleline(&mut gs.replace_text).hint_text("replace (not implemented)"),
    );

    if ui.button("search").clicked() {
        gs.redo_search = true;
    }

    if let Some(search_results) = &mut ctx.global_search.search_results {
        let mut items: Vec<(TextUID, String, &TextBoxSearchResult)> = search_results
            .iter()
            .filter_map(|(id, tbsr)| match &tbsr.tab {
                Tab::FileObject(tab_id) => {
                    let file_object_name = project.objects.get(tab_id)?.borrow().get_title();
                    Some((*id, file_object_name, tbsr))
                }
                Tab::ProjectMetadata => Some((*id, String::from("Project Metadata"), tbsr)),
            })
            .filter(|(_, _, tbsr)| !tbsr.finds.is_empty())
            .collect();

        items.sort_by_key(|(_, file_object_name, tbsr)| (file_object_name.clone(), &tbsr.box_name));

        let mut file_object_id: Option<String> = None;

        for (id, file_object_name, tbsr) in items {
            if file_object_id.as_ref() != Some(&file_object_name) {
                file_object_id = Some(file_object_name.clone());
                ui.colored_label(Color32::LIGHT_GREEN, &file_object_name);
            }

            ui.colored_label(Color32::LIGHT_BLUE, &tbsr.box_name);

            for word_find in &tbsr.finds {
                if word_find.ui(ui).clicked() {
                    ctx.global_search.focus = Some((id, word_find.clone()));
                    ctx.global_search.goto_focus = true;
                    ctx.global_search.version += 1;
                }
            }
        }
    }
    response
}
