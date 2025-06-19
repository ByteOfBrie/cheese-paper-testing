# Cheese Paper

Text editor for writing prose, especially fiction. Cheese Paper attempts to have metadata (especially notes and summaries) specific to individual scenes, all of it tied together. This is available in other editors, but this project attempts to provide that while retaining a simple file format that can be easily synced and remain easily meaningful outside this editor, including being somewhat reasonable to edit on a phone.

The underlying text is all Markdown, so the file format is simple. Metadata is added to the underlying format in a TOML header, once again simple and easy to edit. Files created outside the editor will be parsed in with missing values initialized to the defaults. This allows for files to be created outside the editor, not just modified.

For similar projects that are much more complete, check out [Manuskript](https://github.com/olivierkes/manuskript) (FOSS) or [Scrivener](https://www.literatureandlatte.com/scrivener/overview) (closed source, paid). Neither of these quite met my use case, which is why this project exists. Once this project is more complete, a feature comparison matrix will be added, but this project is still missing some very basic features (search, selecting POV character per scene, distribution).

# Rewrite

This version is a rewrite/reimplementation of [the original cheese-paper editor](https://gitlab.com/BrieVee/cheese-paper), which is written in Python. Both editors are designed around the same file format, at present, they should be capable of sharing work. In the future, I expect this version to have features not present in the Python implementation. As of 2025-06-19, the python editor is much more complete, but I expect that to change before too long (it's easier to write things the second time).

This isn't (primarily) about performance but instead:
* stop using tkinter (please, it was suffering)
* ease of distribution (spell check in particular was annoying)
* wayland support
* (theoretical) screen reader support
