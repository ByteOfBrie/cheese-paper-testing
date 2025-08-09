use crate::components::Text;

#[derive(Debug, Default)]
pub struct TextBoxSearchResult {
    // File object that this text box is in
    file_object_name: String,

    box_name: String,

    // sorted list of search matches in the text
    finds: Vec<WordFind>,
}

#[derive(Debug)]
struct WordFind {
    start: usize,
    end: usize,
    preview: WordFindPreview,
}

#[derive(Debug)]
struct WordFindPreview {
    word: String,
}

pub fn search(text: &Text, file_object_name: &str, box_name: &str, search_term: &str) -> TextBoxSearchResult {

    let mut finds = Vec::new();

    for (start, m) in text.text.match_indices(search_term) {
        let preview = WordFindPreview{ word: m.to_string() };
        let end = start + m.len();
        finds.push(WordFind { start, end, preview });
    }

    TextBoxSearchResult {
        file_object_name: file_object_name.to_string(),
        box_name: box_name.to_string(),
        finds,
    }
}