use egui::Color32;

use super::textbox_search::TextBoxSearchResult;
use super::*;
use crate::components::Project;
use crate::ui::project_editor::search::textbox_search::WordFind;

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct GlobalSearch {
    pub active: bool,

    pub find_text: String,

    pub replace_text: String,

    pub redo_search: bool,

    pub search_results: Option<HashMap<TextUID, TextBoxSearchResult>>,

    pub focus: Option<(TextUID, WordFind)>,

    pub goto_focus: bool,

    pub version: usize,
}

impl GlobalSearch {
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn clear_focus(&mut self) {
        self.focus = None;
        self.goto_focus = false;
    }
}

pub fn search(project: &Project, ctx: &mut EditorContext) {
    let mut search_results = HashMap::new();

    for (key, object) in project.objects.iter() {
        object.as_editor().for_each_textbox(&mut |text, box_name| {
            let search_result =
                textbox_search::search(text, key, box_name, &ctx.global_search.find_text);
            search_results.insert(text.id(), search_result);
        });
    }

    ctx.global_search.search_results = Some(search_results);
    ctx.global_search.clear_focus();
    ctx.global_search.version += 1;
}

pub fn ui(ui: &mut Ui, project: &Project, ctx: &mut EditorContext) {
    let gs = &mut ctx.global_search;

    let min_height = 30.0;

    ui.label("find");

    ui.add_sized(
        egui::vec2(ui.available_width(), min_height),
        egui::TextEdit::singleline(&mut gs.find_text),
    );

    ui.label("replace (not implemented)");

    ui.add_sized(
        egui::vec2(ui.available_width(), min_height),
        egui::TextEdit::singleline(&mut gs.replace_text),
    );

    let search_clicked = ui.button("search").clicked();

    if search_clicked {
        gs.redo_search = true;
    }

    if let Some(search_results) = &mut ctx.global_search.search_results {
        let mut items: Vec<(TextUID, String, &TextBoxSearchResult)> = search_results
            .iter()
            .filter_map(|(id, tbsr)| {
                let file_object_name = project.objects.get(&tbsr.file_object_id)?.get_title();
                Some((*id, file_object_name, tbsr))
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
                let clicked = word_find.ui(ui).clicked();
                if clicked {
                    ctx.global_search.focus = Some((id, word_find.clone()));
                    ctx.global_search.goto_focus = true;
                    ctx.global_search.version += 1;
                }
            }
        }
    }
}
