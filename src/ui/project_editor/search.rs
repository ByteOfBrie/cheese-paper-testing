pub mod global_search;
pub mod textbox_search;

use crate::ui::project_editor::search::textbox_search::WordFind;
use crate::{components::project::ProjectMetadata, ui::prelude::*};
use textbox_search::TextBoxSearchResult;

#[derive(Debug, Default)]
pub struct Search {
    pub active: bool,

    /// Search has just been activated, take focus
    pub request_ui_focus: bool,

    pub find_text: String,

    pub replace_text: String,

    pub redo_search: bool,

    pub search_results: Option<HashMap<TextUID, TextBoxSearchResult>>,

    pub focus: Option<(TextUID, WordFind)>,

    pub goto_focus: bool,
}

type SearchableIterValue<'a> = (Page, Searchable<'a>);
pub enum Searchable<'a> {
    FileObject(&'a RefCell<dyn FileObject>),
    ProjectMetadata(&'a ProjectMetadata),
}

impl Searchable<'_> {
    pub fn search(&self, search_function: &mut dyn FnMut(&Text, &'static str)) {
        match self {
            Searchable::FileObject(file_object) => {
                file_object
                    .borrow()
                    .as_editor()
                    .for_each_textbox(search_function);
            }
            Searchable::ProjectMetadata(metadata) => metadata.for_each_textbox(search_function),
        }
    }
}

impl ProjectEditor {
    pub fn get_searchable(&'_ self) -> impl Iterator<Item = SearchableIterValue<'_>> {
        let object_iter =
            self.project.objects.iter().map(|(id, file_object)| {
                (Page::from_file_id(id), Searchable::FileObject(file_object))
            });

        let metadata_iter = std::iter::once((
            Page::ProjectMetadata,
            Searchable::ProjectMetadata(&self.project.metadata),
        ));

        object_iter.chain(metadata_iter)
    }

    pub fn search(&mut self) {
        let mut search_results = HashMap::new();

        for (key, object) in self.get_searchable() {
            object.search(&mut |text, box_name| {
                let search_result = textbox_search::search(
                    text,
                    &key,
                    box_name,
                    &self.editor_context.search.find_text,
                );
                search_results.insert(text.id(), search_result);
            });
        }

        self.editor_context.search.search_results = Some(search_results);
        self.editor_context.search.clear_focus();

        // trigger a formatting refresh
        self.editor_context.version += 1;
    }
}
