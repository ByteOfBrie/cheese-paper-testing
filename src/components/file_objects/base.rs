use serde::Deserialize;
use std::fs::File;

// in order to have defaults, there should be some null/empty value that can be set,
// which then means that default values will have to be implemented when using the
// config values in other places, not in the config itself. it's not super clean,
// but I don't see how I can realistically get default values that can be unset
// in the UI without implementing config reading by hand (or at least more manually
//
// maybe I should have a "config_file" and "config" concepts at different layers?
// that avoids potentially having a default defined in multiple places, but feels
// really ugly
//
// the hard problem here is that I have to write the values back, but I want to
// retain the ability to unset them

#[derive(Deserialize)]
struct CheesePaperConfig {
    font: String,
    font_size: i64,
}

// lowest level of organization, contains writing
struct Scene {
    file_pointer: File,
}

fn read_app_config() {
    // read config file to string from $XDG_CACHE_HOME
    //let cheese_paper_config: CheesePaperConfig
}