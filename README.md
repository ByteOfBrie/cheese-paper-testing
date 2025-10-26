# Cheese Paper

Text editor for writing prose, especially fiction. Cheese Paper attempts to have metadata (especially notes and summaries) specific to individual scenes, all of it tied together. Unlike any other project's that I'm aware of, cheese-paper attempts to provide that while retaining a simple file format that can be easily synced and remain easily meaningful outside the editor, including being reasonable to edit on a phone.

The underlying text is all Markdown, so the file format is simple. Metadata is added to the underlying format in a TOML header, once again simple and easy to edit. Files created outside the editor are parsed in with missing values initialized to the defaults. This allows for files to be created outside the editor, not just modified. There is also a system for syncing

For more complete similar projects, check out [Manuskript](https://github.com/olivierkes/manuskript) (FOSS) or [Scrivener](https://www.literatureandlatte.com/scrivener/overview) (closed source, paid). I've used both extensively, although neither of these quite met my use case, which is why cheese-paper exists. Once this project is more complete, a feature comparison matrix will be added, but this project isn't quite ready for general use (at least needing some cleanup around spellcheck and distribution/packaging)

