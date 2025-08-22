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

    /// When search is closed, signal to the rest of the editor that it needs to redraw
    /// (which is also responsible for unsetting this)
    pub exiting_search: bool,
}

impl Search {
    pub fn show(&mut self) {
        self.active = true;
        self.request_ui_focus = true;
    }

    pub fn hide(&mut self) {
        self.active = false;
        self.exiting_search = true;
    }

    pub fn clear_focus(&mut self) {
        self.focus = None;
        self.goto_focus = false;
    }
}

pub enum Searchable<'a> {
    FileObject(&'a RefCell<dyn FileObject>),
    ProjectMetadata(&'a ProjectMetadata),
}

impl Searchable<'_> {
    pub fn search(&self, page: &Page, search: &mut Search) {
        let mut search_function = |text: &'_ Text, box_name: &'_ str| {
            let search_result = textbox_search::search(text, page, box_name, &search.find_text);
            search
                .search_results
                .as_mut()
                .unwrap()
                .insert(text.id(), search_result);
        };

        match self {
            Searchable::FileObject(file_object) => {
                file_object
                    .borrow()
                    .as_editor()
                    .for_each_textbox(&mut search_function);
            }
            Searchable::ProjectMetadata(metadata) => {
                metadata.for_each_textbox(&mut search_function)
            }
        }
    }
}

impl ProjectEditor {
    pub fn search(&mut self) {
        self.editor_context.search.search_results = Some(HashMap::new());

        let object_iter =
            self.project.objects.iter().map(|(id, file_object)| {
                (Page::from_file_id(id), Searchable::FileObject(file_object))
            });

        let metadata_iter = std::iter::once((
            Page::ProjectMetadata,
            Searchable::ProjectMetadata(&self.project.metadata),
        ));

        for (key, object) in object_iter.chain(metadata_iter) {
            object.search(&key, &mut self.editor_context.search);
        }

        self.editor_context.search.clear_focus();

        // trigger a formatting refresh
        self.editor_context.version += 1;
    }
}

impl Project {
    pub fn get_searchable<'a>(&'a self, page: &Page) -> Option<Searchable<'a>> {
        match page {
            Page::FileObject(file_id) => {
                Some(Searchable::FileObject(self.objects.get(file_id).unwrap()))
            }
            Page::ProjectMetadata => Some(Searchable::ProjectMetadata(&self.metadata)),
            Page::Export => None,
        }
    }
}
