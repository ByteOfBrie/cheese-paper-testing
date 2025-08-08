mod base;
mod format;
mod spellcheck;

type SavedRegex = std::sync::LazyLock<regex::Regex>;
