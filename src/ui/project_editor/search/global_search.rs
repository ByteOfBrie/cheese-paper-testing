use super::*;
use super::textbox_search::TextBoxSearchResult;
use crate::components::Project;

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct GlobalSearch {
    pub active: bool,

    find_text: String,

    replace_text: String,

    pub redo_search: bool,

    search_results: HashMap<TextUID, TextBoxSearchResult>,
}

impl GlobalSearch {
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }
}

pub fn search(project: &Project, ctx: &mut EditorContext) {

    for (key, object) in project.objects.iter() {
        object.as_editor().for_each_textbox(&mut |text, box_name|{
            let gs = &mut ctx.global_search;

            let search_result = textbox_search::search(text, key, box_name, &gs.find_text);
            gs.search_results.insert(text.id(), search_result);
        });
    }
}

pub fn ui(ui: &mut Ui, ctx: &mut EditorContext) {

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
}
