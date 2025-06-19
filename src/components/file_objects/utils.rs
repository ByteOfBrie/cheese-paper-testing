/// Generic file utilities
use regex::Regex;

/// make sure the name fits within a specified length, trying to split on word boundaries
pub fn truncate_name(name: &str, max_length: usize) -> &str {
    let name = name.trim();
    // if the name is already short enough, we're done
    if name.len() <= max_length {
        return &name;
    }

    // split by word, go with increasingly fewer words
    let split_name: Vec<&str> = name.split(" ").collect();
    for number_of_words in (1..split_name.len()).rev() {
        // construct the split text into a temporary string
        let shortened = &split_name[..number_of_words].join(" ");
        if shortened.len() < max_length && shortened != "" {
            // return a slice of the actual name so it retains the original lifetime
            return &name[..shortened.len()];
        }
    }

    // we the first word is longer than `max_length`, give up on being smart
    &name[..max_length]
}

#[test]
fn test_truncate_name() {
    assert_eq!(truncate_name("Hello World", 30), "Hello World");
    assert_eq!(truncate_name("Hello World", 9), "Hello");
    assert_eq!(truncate_name("Hello World", 11), "Hello World");
    assert_eq!(truncate_name("Hello World", 5), "Hello");
    assert_eq!(truncate_name("Hello World", 4), "Hell");
    assert_eq!(truncate_name(" Hello World", 2), "He");
    assert_eq!(truncate_name("Hello World   ", 30), "Hello World");
}
/// Translates a name into something we can put on disk
pub fn process_name_for_filename(name: &str) -> String {
    // get rid of spaces in names for editing convenience
    let name = name.replace(" ", "_");
    let name = name.replace("'", "");

    // Characters that might be annoying to escape/handle sometimes, avoid including them at all
    let dangerous_character_filter = Regex::new(r#"[/\?%*:|"<>\x7F\x00-\x1F]"#).unwrap();
    dangerous_character_filter
        .replace_all(&name, "-")
        .into_owned()
}

#[test]
fn test_process_name_for_filename() {
    assert_eq!(process_name_for_filename(r"hello world"), "hello_world");
    assert_eq!(process_name_for_filename(r"possessive's"), "possessives");
    assert_eq!(process_name_for_filename(r"asdf?'?s"), "asdf--s");
}

/// Just adds an index to a name, no real logic
pub fn add_index_to_name(name: &str, index: u32) -> String {
    format!("{index:03}-{name}")
}
