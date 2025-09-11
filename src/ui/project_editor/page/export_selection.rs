use egui::Vec2;

use crate::ui::prelude::*;

//This probably shouldn't be a part of Project but it's easy enough right now
impl Project {
    pub fn export_ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_export_selection(ui, ctx))
            .response
    }

    fn show_export_selection(&mut self, ui: &mut egui::Ui, _ctx: &mut EditorContext) {
        ui.label("Project Export Selction");

        egui::Grid::new("Export Options")
            .num_columns(2).spacing(Vec2{x: 5.0, y:10.0})
            .show(ui, |ui| {
                ui.checkbox(
                    &mut self.metadata.export.include_all_folder_titles,
                    "Include All Folder Titles",
                )
                .on_hover_text(
                    "If this is checked, the title from every folder will be included \
                    in the export (as headings)",
                );
                ui.end_row();

                const FOLDER_DEPTH_MESSAGE: &str = "If all folder titles are not being included, \
                    the depth at which they will be included. 0 means no folder titles will be included, \
                    1 means that only top level folder titles will be included, etc.";

                ui.add_enabled_ui(!self.metadata.export.include_all_folder_titles, |ui| {
                    ui.label("Include Folder Title Depth  ℹ")
                        .on_disabled_hover_text(FOLDER_DEPTH_MESSAGE)
                        .on_hover_text(FOLDER_DEPTH_MESSAGE);
                });

                // Same enable conditions, but in a separate block so egui can do the grid properly
                ui.add_enabled_ui(!self.metadata.export.include_all_folder_titles, |ui| {
                    ui.add(egui::DragValue::new(
                        &mut self.metadata.export.include_folder_title_depth,
                    ));
                });
                ui.end_row();


                ui.checkbox(
                    &mut self.metadata.export.include_all_scene_titles,
                    "Include All Scene Titles",
                )
                .on_hover_text(
                    "If this is checked, the title of every scene will be included \
                    in the export (as headings)",
                );
                ui.end_row();


                const SCENE_DEPTH_MESSAGE: &str = "If all scene titles are not being included, \
                    the depth at which they will be included. 0 means no scene titles will be included, \
                    1 means that only top level scene titles will be included, etc.";

                ui.add_enabled_ui(!self.metadata.export.include_all_scene_titles, |ui| {
                    ui.label("Include Scene Title Depth  ℹ")
                        .on_disabled_hover_text(SCENE_DEPTH_MESSAGE)
                        .on_hover_text(SCENE_DEPTH_MESSAGE);
                });

                // Same enable conditions, but in a separate block so egui can do the grid properly
                ui.add_enabled_ui(!self.metadata.export.include_all_scene_titles, |ui| {
                    ui.add(egui::DragValue::new(
                        &mut self.metadata.export.include_scene_title_depth,
                    ));
                });
                ui.end_row();

                ui.checkbox(
                    &mut self.metadata.export.insert_break_at_end,
                    "Insert line breaks between consecutive scenes",
                );
            });
    }
}
