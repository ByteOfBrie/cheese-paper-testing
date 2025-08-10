use egui::Color32;

use super::textbox_search::TextBoxSearchResult;
use super::*;
use crate::components::Project;

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct GlobalSearch {
    pub active: bool,

    pub find_text: String,

    replace_text: String,

    pub redo_search: bool,

    pub search_results: Option<HashMap<TextUID, TextBoxSearchResult>>,

    pub version: usize,
}

impl GlobalSearch {
    pub fn toggle(&mut self) {
        self.active = !self.active;
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

    println!("search complete with {} finds", search_results.len());

    for tbsr in search_results.iter() {
        println!("{tbsr:#?}");
    }

    ctx.global_search.search_results = Some(search_results);
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

    ui.label("replace");

    ui.add_sized(
        egui::vec2(ui.available_width(), min_height),
        egui::TextEdit::singleline(&mut gs.replace_text),
    );

    let search_clicked = ui.button("search").clicked();

    if search_clicked {
        gs.redo_search = true;
    }

    if let Some(search_results) = &mut ctx.global_search.search_results {
        let mut items: Vec<(String, &TextBoxSearchResult)> = search_results
            .iter()
            .filter_map(|(_, tbsr)| {
                let file_object_name = project.objects.get(&tbsr.file_object_id)?.get_title();
                Some((file_object_name, tbsr))
            })
            .filter(|(_, tbsr)| !tbsr.finds.is_empty())
            .collect();

        items.sort_by_key(|(file_object_name, tbsr)| (file_object_name.clone(), &tbsr.box_name));

        let mut file_object_id: Option<String> = None;

        for (file_object_name, tbsr) in items {
            if file_object_id.as_ref() != Some(&file_object_name) {
                file_object_id = Some(file_object_name.clone());
                ui.colored_label(Color32::LIGHT_GREEN, &file_object_name);
            }

            ui.colored_label(Color32::LIGHT_BLUE, &tbsr.box_name);

            for word_find in &tbsr.finds {
                word_find.ui(ui);
            }
        }
    }
}
