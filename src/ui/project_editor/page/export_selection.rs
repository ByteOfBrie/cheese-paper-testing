use egui::ScrollArea;
use egui_ltreeview::{NodeBuilder, TreeView};

use crate::{components::file_objects::MutFileObjectTypeInterface, ui::prelude::*};

//This probably shouldn't be a part of Project but it's easy enough right now
impl Project {
    pub fn export_ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| self.show_export_selection(ui, ctx))
            .response
    }

    fn show_export_selection(&mut self, ui: &mut egui::Ui, _ctx: &mut EditorContext) {
        ui.label("Project Export Selction");

        let font_size = ui
            .style()
            .text_styles
            .get(&egui::TextStyle::Body)
            .unwrap()
            .size;
        let node_height = (font_size * 1.1).ceil();

        ScrollArea::vertical()
            .id_salt("export tree scroll area")
            .show(ui, |ui| {
                TreeView::new(ui.make_persistent_id("export tree")).show(ui, |builder| {
                    let text = self.objects.get(&self.text_id).unwrap().borrow();

                    for child in text.children(&self.objects) {
                        build_export_tree(child, node_height, &self.objects, builder);
                    }
                });
            });
    }
}

fn build_export_tree(
    object: &RefCell<dyn FileObject>,
    node_height: f32,
    objects: &FileObjectStore,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, String>,
) {
    match object.borrow_mut().get_file_type_mut() {
        MutFileObjectTypeInterface::Scene(scene) => {
            let node = NodeBuilder::leaf((scene as &dyn FileObject).id().to_string())
                .height(node_height)
                .label((scene as &dyn FileObject).get_title());

            builder.node(node);
        }
        MutFileObjectTypeInterface::Folder(folder) => {
            let node = NodeBuilder::dir((folder as &dyn FileObject).id().to_string())
                .height(node_height)
                .label((folder as &dyn FileObject).get_title());

            builder.node(node);

            for child in (folder as &dyn FileObject).children(objects) {
                build_export_tree(child, node_height, objects, builder);
            }

            builder.close_dir();
        }
        _ => {}
    }
}
