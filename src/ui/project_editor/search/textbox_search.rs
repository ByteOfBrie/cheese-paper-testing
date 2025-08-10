use crate::components::Text;

#[derive(Debug, Default)]
pub struct TextBoxSearchResult {
    // File object that this text box is in
    pub file_object_id: String,

    pub box_name: String,

    // sorted list of search matches in the text
    pub finds: Vec<WordFind>,

    // version of the text that this was computed for
    pub text_version: usize,
}

#[derive(Debug)]
pub struct WordFind {
    pub start: usize,
    pub end: usize,
    preview: WordFindPreview,
}

#[derive(Debug)]
struct WordFindPreview {
    word: String,
}

impl WordFind {
    pub fn ui(&self, ui: &mut egui::Ui) {
        ui.label(&self.preview.word);
    }
}

pub fn search(
    text: &Text,
    file_object_id: &str,
    box_name: &str,
    search_term: &str,
) -> TextBoxSearchResult {
    let mut finds = Vec::new();

    for (start, m) in text.text.match_indices(search_term) {
        let preview = WordFindPreview {
            word: m.to_string(),
        };
        let end = start + m.len();
        finds.push(WordFind {
            start,
            end,
            preview,
        });
    }

    TextBoxSearchResult {
        file_object_id: file_object_id.to_string(),
        box_name: box_name.to_string(),
        finds,
        text_version: text.version,
    }
}
