pub mod global_search;
pub mod textbox_search;

use crate::ui::prelude::*;

type SearchableIterValue<'a> = (Tab, Searchable<'a>);
pub enum Searchable<'a> {
    FileObject(&'a RefCell<dyn FileObject>),
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
        }
    }
}

impl ProjectEditor {
    pub fn get_searchable(&'_ self) -> impl Iterator<Item = SearchableIterValue<'_>> {
        self.project
            .objects
            .iter()
            .map(|(id, file_object)| (Tab::from_file_id(id), Searchable::FileObject(file_object)))

        // TODO: add other searchable objects to this with iterator.chain
    }

    pub fn search(&mut self) {
        let mut search_results = HashMap::new();

        for (key, object) in self.get_searchable() {
            object.search(&mut |text, box_name| {
                let search_result = textbox_search::search(
                    text,
                    &key,
                    box_name,
                    &self.editor_context.global_search.find_text,
                );
                search_results.insert(text.id(), search_result);
            });
        }

        self.editor_context.global_search.search_results = Some(search_results);
        self.editor_context.global_search.clear_focus();
        self.editor_context.global_search.version += 1;
    }
}
