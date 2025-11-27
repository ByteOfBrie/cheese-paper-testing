use crate::components::file_objects::reference::ObjectReference;
use crate::{components::file_objects::base::CompileStatus, ui::prelude::*};

use super::FileObjectEditor;
use crate::components::file_objects::Scene;
use crate::components::file_objects::base::IncludeOptions;

use egui::Id;
use egui::ScrollArea;

#[derive(Debug, Default, PartialEq)]
pub enum SidebarTab {
    #[default]
    Notes,
    Export,
}

#[derive(Debug, Default)]
pub struct SceneData {
    sidebar_tab: SidebarTab,
}

pub type Store = RenderDataStore<FileID, SceneData>;

impl FileObjectEditor for Scene {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        let sidebar_ids = egui::SidePanel::right("metadata sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(50.0..)
            .show_inside(ui, |ui| self.show_sidebar(ui, ctx))
            .inner;

        let mut ids = egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_text_editor(ui, ctx))
            .inner;

        ids.extend(sidebar_ids);
        ids
    }

    fn for_each_textbox<'a>(&'a self, f: &mut dyn FnMut(&Text, &'static str)) {
        f(&self.metadata.summary, "Summary");
        f(&self.metadata.notes, "Notes");
        f(&self.text, "text");
    }

    fn for_each_textbox_mut<'a>(&'a mut self, f: &mut dyn FnMut(&mut Text, &'static str)) {
        f(&mut self.metadata.summary, "Summary");
        f(&mut self.metadata.notes, "Notes");
        f(&mut self.text, "text");
    }
}

impl Scene {
    fn show_text_editor(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        ScrollArea::vertical()
            .id_salt("text")
            .auto_shrink(egui::Vec2b { x: false, y: false })
            .show(ui, |ui| {
                let response =
                    ui.add_sized(ui.available_size(), |ui: &'_ mut Ui| self.text.ui(ui, ctx));

                self.process_response(&response);
                vec![response.id]
            })
            .inner
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<Id> {
        let rdata = ctx.stores.scene.get(&self.get_base().metadata.id);
        let mut scene_data = rdata.borrow_mut();

        let mut ids = Vec::new();

        egui::TopBottomPanel::bottom("word_count").show_inside(ui, |ui| {
            ui.add_space(4.0);
            let words = self.text.word_count(ctx);
            let text = format!("{words} Words");
            ui.vertical_centered(|ui| {
                ui.label(text);
            });
        });

        ScrollArea::vertical().id_salt("metadata").show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.get_base_mut().metadata.name)
                    .id_salt("name")
                    .hint_text("Scene Name")
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            self.process_response(&response);
            ids.push(response.id);

            let text_box_height = response.rect.height().abs();

            // Tab selection
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut scene_data.sidebar_tab,
                    SidebarTab::Notes,
                    "Summary/Notes",
                );
                ui.selectable_value(&mut scene_data.sidebar_tab, SidebarTab::Export, "Export");
            });

            ui.separator();

            let sidebar_other_ids = match scene_data.sidebar_tab {
                SidebarTab::Notes => self.show_sidebar_metadata(ui, ctx, text_box_height),
                SidebarTab::Export => self.show_sidebar_export(ui),
            };

            ids.extend(sidebar_other_ids);
        });
        ids
    }

    fn show_sidebar_metadata(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut EditorContext,
        text_box_height: f32,
    ) -> Vec<Id> {
        let mut ids = Vec::new();

        // I am doing horrible things here but the borrow checker must be satisifed
        let changed = {
            let mut object_pov = self.metadata.pov.borrow_mut();
            let mut pov = object_pov.clone();

            ui.horizontal(|ui| {
                ui.label("POV: ");
                egui::ComboBox::from_id_salt("metadata pov")
                    .selected_text(match &pov {
                        ObjectReference::Known(known_current_pov) => {
                            if let Some(current_pov_name) =
                                ctx.references.characters.get(known_current_pov)
                            {
                                current_pov_name.clone()
                            } else {
                                format!("Ref: {known_current_pov}")
                            }
                        }
                        ObjectReference::Unknown(unknown_reference) => {
                            if unknown_reference.name.is_empty() {
                                format!("Ref: {}", unknown_reference.id)
                            } else {
                                format!("Ref: {}|{}", unknown_reference.name, unknown_reference.id)
                            }
                        }
                        ObjectReference::None => "None".to_string(),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut pov, ObjectReference::None, "None");
                        for (character_reference, name) in ctx.references.characters.iter() {
                            ui.selectable_value(
                                &mut pov,
                                ObjectReference::Known(character_reference.clone()),
                                name,
                            );
                        }
                    });
            });

            // We don't have an actual response here so we have to manually process
            if pov != *object_pov {
                *object_pov = pov;
                true
            } else {
                false
            }
        };

        if changed {
            self.get_base_mut().file.modified = true;
        }

        // half of the available height should go to each widget
        let widget_space = ui.available_height() / 2.0;

        // we assume that the widget metadata itself will take up slightly more room than the text box
        let metadata_text_space = widget_space - text_box_height * 1.2;

        // make sure we don't go smaller than one line (which would be meaningless)
        let min_height = metadata_text_space.max(text_box_height);

        egui::CollapsingHeader::new("Summary")
            .default_open(true)
            .show(ui, |ui| {
                let response = ui.add_sized(
                    egui::vec2(ui.available_width(), min_height),
                    |ui: &'_ mut Ui| self.metadata.summary.ui(ui, ctx),
                );
                self.process_response(&response);
                ids.push(response.id);
            });

        egui::CollapsingHeader::new("Notes")
            .default_open(true)
            .show(ui, |ui| {
                let response = ui.add_sized(
                    egui::vec2(ui.available_width(), min_height),
                    |ui: &'_ mut Ui| self.metadata.notes.ui(ui, ctx),
                );
                self.process_response(&response);
                ids.push(response.id);
            });
        ids
    }

    fn show_sidebar_export(&mut self, ui: &mut egui::Ui) -> Vec<Id> {
        let mut ids = Vec::new();
        // Check box for including this file entirely
        let mut export_include = self
            .metadata
            .compile_status
            .contains(CompileStatus::INCLUDE);
        let response = ui.checkbox(&mut export_include, "Include in export");
        if response.changed() {
            self.metadata
                .compile_status
                .set(CompileStatus::INCLUDE, export_include);
        }
        self.process_response(&response);
        ids.push(response.id);

        // The rest of the checkboxes have no effect if export isn't included
        ui.add_enabled_ui(export_include, |ui| {
            let mut include_title = self.metadata.compile_status.include_title();
            let include_title_before = include_title;

            ui.horizontal(|ui| {
                const INCLUDE_TITLE_MESSAGE: &str =
                    "If the title of this folder/scene will be included
                default - this will come from the settings in the export tab
                always - include the title for this, even if the project export settings differ
                never - do not include the title for this, even if the export settings differ";

                ui.label("Include Title  ℹ")
                    .on_hover_text(INCLUDE_TITLE_MESSAGE);

                let title_combobox_response = egui::ComboBox::from_id_salt("Include Title")
                    .selected_text(format!("{include_title:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut include_title, IncludeOptions::Default, "Default");
                        ui.selectable_value(&mut include_title, IncludeOptions::Always, "Always");
                        ui.selectable_value(&mut include_title, IncludeOptions::Never, "Never");
                    });

                // We want to be able to tab to the box, but it doesn't get a process_response
                // call because that needs to be handled below
                ids.push(title_combobox_response.response.id);
            });

            // We don't have an actual response here so we have to manually process
            if include_title != include_title_before {
                self.metadata
                    .compile_status
                    .set_include_title(include_title);
                self.get_base_mut().file.modified = true;
            }

            // same thing but for the break
            let mut break_at_end = self.metadata.compile_status.break_at_end();
            let break_at_end_before = break_at_end;

            ui.horizontal(|ui| {
                const INCLUDE_BREAK_MESSAGE: &str =
                    "If this is followed by a scene, should there be a divider?
                    default - this will come from the settings in the export tab
                    always - include a divider after this, even if the project export settings differ
                    never - do not include a divider after this, even if the export settings differ";

                ui.label("Break at End  ℹ")
                    .on_hover_text(INCLUDE_BREAK_MESSAGE);

                let break_combobox_response = egui::ComboBox::from_id_salt("Break at End")
                    .selected_text(format!("{break_at_end:?}"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut break_at_end, IncludeOptions::Default, "Default");
                        ui.selectable_value(&mut break_at_end, IncludeOptions::Always, "Always");
                        ui.selectable_value(&mut break_at_end, IncludeOptions::Never, "Never");
                    });

                // We want to be able to tab to the box, but it doesn't get a process_response
                // call because that needs to be handled below
                ids.push(break_combobox_response.response.id);
            });

            // We don't have an actual response here so we have to manually process
            if break_at_end != break_at_end_before {
                self.metadata.compile_status.set_break_at_end(break_at_end);
                self.get_base_mut().file.modified = true;
            }
        });

        ids
    }
}
