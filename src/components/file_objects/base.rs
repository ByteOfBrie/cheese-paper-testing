mod implementation;
pub use implementation::*;

use bitflags::bitflags;
use uuid::Uuid;

use super::{BaseFileObject, FileObject, FileObjectMetadata};
use crate::cheese_error;
use crate::components::file_objects::utils::{
    add_index_to_name, get_index_from_name, process_name_for_filename, truncate_name,
};
// use crate::components::file_objects::{Character, Folder, Place, Scene};
use crate::components::schema::{FileType, Schema};
use crate::util::CheeseError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::Debug;
use std::fs::create_dir;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;
use toml_edit::{DocumentMut, TableLike};

/// the maximum length of a name before we start trying to truncate it
const FILENAME_MAX_LENGTH: usize = 30;

/// filename of the object within a folder containing its metadata (without extension)
pub const FOLDER_METADATA_FILE_NAME: &str = "metadata.toml";

/// Value that splits the header of any file that contains non-metadata content
const HEADER_SPLIT: &str = "++++++++";

impl Default for FileObjectMetadata {
    fn default() -> Self {
        Self {
            version: 1u64,
            name: String::new(),
            id: Rc::new(Uuid::new_v4().as_hyphenated().to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Path of the directory containing this file
    /// `/foo/bar/` -> `/foo`
    pub dirname: PathBuf,
    /// Path of the file within the dirname
    /// `/foo/bar/` -> `bar`
    pub basename: OsString,
    /// Modified time if the file exists
    pub modtime: Option<SystemTime>,
    pub modified: bool,
}

bitflags! {

    /// The presence of this particular object in the export.
    ///
    /// The `INCLUDE` will (likely) represent an "include in story" checkbox,
    /// if it's not set, none of the other options have any meaningful value.
    /// We (currently) accept bools as well, since that's most of the normal
    /// configuration.
    ///
    /// At a data level, it would probably make more sense to have (at least)
    /// three separate variables here (include, include_title, and break_at_end),
    /// where the latter two will have an enum with (default, always, and never),
    /// which is how it'll likely be presented in the UI, but doing it this way is
    /// more compact in the file.
    #[derive(Debug)]
    pub struct CompileStatus: u64 {
        const INCLUDE                = 0b0000_0000_0000_0001;
        const OVERRIDE_INCLUDE_TITLE = 0b0000_0000_0000_0010;
        const INCLUDE_TITLE          = 0b0000_0000_0000_0100;
        const OVERRIDE_BREAK_AT_END  = 0b0000_0000_0000_1000;
        const BREAK_AT_END           = 0b0000_0000_0001_0000;

        // allow for any bits, in case a future version of cheese-paper sets more
        const _ = !0;
    }
}

impl Default for CompileStatus {
    fn default() -> Self {
        CompileStatus::INCLUDE
    }
}

impl CompileStatus {
    pub fn include_title(&self) -> IncludeOptions {
        if self.contains(CompileStatus::INCLUDE_TITLE | CompileStatus::OVERRIDE_INCLUDE_TITLE) {
            IncludeOptions::Always
        } else if self.contains(CompileStatus::OVERRIDE_INCLUDE_TITLE) {
            IncludeOptions::Never
        } else {
            IncludeOptions::Default
        }
    }

    pub fn set_include_title(&mut self, options: IncludeOptions) {
        match options {
            IncludeOptions::Default => self.set(CompileStatus::OVERRIDE_INCLUDE_TITLE, false),
            IncludeOptions::Always => {
                self.set(CompileStatus::OVERRIDE_INCLUDE_TITLE, true);
                self.set(CompileStatus::INCLUDE_TITLE, true);
            }
            IncludeOptions::Never => {
                self.set(CompileStatus::OVERRIDE_INCLUDE_TITLE, true);
                self.set(CompileStatus::INCLUDE_TITLE, false);
            }
        }
    }

    pub fn break_at_end(&self) -> IncludeOptions {
        if self.contains(CompileStatus::BREAK_AT_END | CompileStatus::OVERRIDE_BREAK_AT_END) {
            IncludeOptions::Always
        } else if self.contains(CompileStatus::OVERRIDE_BREAK_AT_END) {
            IncludeOptions::Never
        } else {
            IncludeOptions::Default
        }
    }

    pub fn set_break_at_end(&mut self, options: IncludeOptions) {
        match options {
            IncludeOptions::Default => self.set(CompileStatus::OVERRIDE_BREAK_AT_END, false),
            IncludeOptions::Always => {
                self.set(CompileStatus::OVERRIDE_BREAK_AT_END, true);
                self.set(CompileStatus::BREAK_AT_END, true);
            }
            IncludeOptions::Never => {
                self.set(CompileStatus::OVERRIDE_BREAK_AT_END, true);
                self.set(CompileStatus::BREAK_AT_END, false);
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum IncludeOptions {
    #[default]
    Default,
    Always,
    Never,
}

pub fn metadata_extract_u64(
    table: &dyn TableLike,
    field_name: &str,
    allow_bool: bool,
) -> Result<Option<u64>, CheeseError> {
    match table.get(field_name) {
        Some(value) => {
            if let Some(value) = value.as_integer() {
                Ok(Some(value as u64))
            } else if allow_bool && let Some(value) = value.as_bool() {
                Ok(Some(value as u64))
            } else {
                Err(cheese_error!("{field_name} was not an integer"))
            }
        }
        None => Ok(None),
    }
}

pub fn metadata_extract_string(
    table: &dyn TableLike,
    field_name: &str,
) -> Result<Option<String>, CheeseError> {
    Ok(match table.get(field_name) {
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| cheese_error!("{field_name} was not string"))?
                .to_owned(),
        ),
        None => None,
    })
}

pub fn metadata_extract_bool(
    table: &dyn TableLike,
    field_name: &str,
) -> Result<Option<bool>, CheeseError> {
    Ok(match table.get(field_name) {
        Some(value) => Some(
            value
                .as_bool()
                .ok_or_else(|| cheese_error!("{field_name} was not bool"))?,
        ),
        None => None,
    })
}

/// Reads the contents of a file from disk
pub fn read_file_contents(file_to_read: &Path) -> Result<(String, Option<String>), CheeseError> {
    let extension = match file_to_read.extension() {
        Some(val) => val,
        None => return Err(cheese_error!("value was not string")),
    };

    let file_data = std::fs::read_to_string(file_to_read)?;

    let (metadata_str, file_content): (&str, Option<&str>) = if extension == "md" {
        match file_data.split_once(HEADER_SPLIT) {
            None => ("", Some(&file_data)),
            Some((start, end)) => (start, Some(end)),
        }
    } else {
        (&file_data, None)
    };

    Ok((
        metadata_str.to_owned(),
        file_content.map(|s| s.trim().to_owned()),
    ))
}

/// Given a freshly read metadata dictionary, read it into the file objects, setting modified as
/// appropriate
pub fn load_base_metadata(
    metadata_table: &dyn TableLike,
    metadata_object: &mut FileObjectMetadata,
    file_info: &mut FileInfo,
) -> Result<(), CheeseError> {
    match metadata_extract_u64(metadata_table, "file_format_version", false)? {
        Some(version) => metadata_object.version = version,
        None => file_info.modified = true,
    }

    match metadata_extract_string(metadata_table, "name")? {
        Some(name) => metadata_object.name = name,
        None => file_info.modified = true,
    }

    match metadata_extract_string(metadata_table, "id")? {
        Some(id) => metadata_object.id = Rc::new(id),
        None => file_info.modified = true,
    }

    Ok(())
}

fn parent_contains(parent_id: &FileID, checking_id: &FileID, objects: &FileObjectStore) -> bool {
    let parent = objects.get(parent_id).unwrap();

    for child_id in &parent.borrow().get_base().children {
        // directly check if this is object we're looking for
        if child_id == checking_id {
            return true;
        }

        // check all of the children
        if parent_contains(child_id, checking_id, objects) {
            return true;
        }
    }

    false
}

/// Move a child between two folders, `source_file_id` and `dest_file_id`
///
/// This can't be part of the FileObject trait because ownership is complicated between
/// the
pub fn move_child(
    moving_file_id: &FileID,
    source_file_id: &FileID,
    dest_file_id: &FileID,
    new_index: usize,
    objects: &FileObjectStore,
) -> Result<(), CheeseError> {
    // Check for it being a valid move:
    // * can't move to one of your own children
    if parent_contains(moving_file_id, dest_file_id, objects) {
        return Err(cheese_error!(
            "attempted to move {moving_file_id} into itself"
        ));
    }

    // * can't move something without an index
    let moving = objects
        .get(moving_file_id)
        .expect("objects should contain moving file id");

    let moving_index = match moving.borrow().get_base().index {
        Some(index) => index,
        None => {
            return Err(cheese_error!(
                "attempted to move {moving_file_id:} into itself"
            ));
        }
    };
    // * shouldn't move something where it already is
    if source_file_id == dest_file_id && moving_index == new_index {
        log::debug!("attempted to move {moving_file_id} to itself, skipping");
        return Ok(());
    }

    // We know it's a valid move (or at least think we do), go ahead with the move

    // From this point until the call to fix indexing, we have state that we can't safely recover
    // from with an error, so we should always panic instead
    create_index_and_move_on_disk(
        moving_file_id,
        source_file_id,
        dest_file_id,
        new_index,
        objects,
    );

    // if we're moving within an object, we already fixed indexing a few lines above
    if source_file_id != dest_file_id {
        // We just need to clean up and re-index the source to fill in the gap we left
        objects
            .get(source_file_id)
            .unwrap()
            .borrow_mut()
            .fix_indexing(objects);
    }

    Ok(())
}

/// Calculates the filename for a particular object
fn calculate_filename(file_type: FileType, base_info: &BaseFileObject) -> OsString {
    let base_name: &str = match base_info.metadata.name.is_empty() {
        false => &base_info.metadata.name,
        true => file_type.empty_string_name(),
    };

    let mut basename = match base_info.index {
        Some(index) => {
            let truncated_name = truncate_name(base_name, FILENAME_MAX_LENGTH);
            let file_safe_name = process_name_for_filename(truncated_name);
            let final_name = add_index_to_name(&file_safe_name, index);

            OsString::from(final_name)
        }
        None => OsString::from(process_name_for_filename(base_name)),
    };

    if !file_type.is_folder() {
        basename.push(".");
        basename.push(file_type.extension());
    }

    basename
}

/// Helper function called by move_child for the parts that are not safe to return early (including
/// errors). If something goes wrong, it will panic
fn create_index_and_move_on_disk(
    moving_file_id: &FileID,
    source_file_id: &FileID,
    dest_file_id: &FileID,
    new_index: usize,
    objects: &FileObjectStore,
) {
    // Create index "gap" in destination (helpful to do first in case we're moving "up" and this
    // changes the path of the object being moved)
    objects
        .get(dest_file_id)
        .unwrap()
        .borrow_mut()
        .create_index_gap(new_index, objects)
        .unwrap();

    // Remove the moving object from it's current parent
    let source = objects
        .get(source_file_id)
        .expect("objects should contain source file id");

    let child_id_position = source
        .borrow()
        .get_base()
        .children
        .iter()
        .position(|val| moving_file_id == val)
        .unwrap_or_else(|| {
            panic!(
                "Children should only be removed from their parents. \
                child id: {moving_file_id}, parent: {source_file_id}"
            )
        });

    let child_id_string = source
        .borrow_mut()
        .get_base_mut()
        .children
        .remove(child_id_position);

    // Object is now removed from it's current parent, although still actually there on disk
    // We should also stop using `source` or the (scary) borrow checker will get mad at us
    // update: we have added some refcells everywhere and the borrow checker is nicer now

    let dest = objects.get(dest_file_id).unwrap();

    let insertion_index = std::cmp::min(new_index, dest.borrow().get_base().children.len());
    // Move the object into the children of dest (at the proper place)
    dest.borrow_mut()
        .get_base_mut()
        .children
        .insert(insertion_index, child_id_string);

    let child = objects.get(moving_file_id).unwrap();

    // Move the actual child on disk
    if let Err(err) = child
        .borrow_mut()
        .move_object(new_index, dest.borrow().get_path(), objects)
    {
        // We don't pass enough information around to meaninfully recover here
        log::error!("Encountered error while trying to move {moving_file_id}");
        panic!("Encountered unrecoverable error while trying to move {moving_file_id}: {err}");
    }

    // Fix indexing in the destination (now that it has the child)
    dest.borrow_mut().fix_indexing(objects);
}

/// Load an arbitrary file object from a file on disk into objects
pub fn load_file(
    schema: &dyn Schema,
    filename: &Path,
    objects: &mut FileObjectStore,
) -> Result<FileID, CheeseError> {
    if !filename.exists() {
        return Err(cheese_error!(
            "from_file cannot load file that does not exist: {filename:?}"
        ));
    }

    // We process every dir, but only `.toml` or `.md` files
    if !filename.is_dir()
        && filename
            .extension()
            .is_none_or(|extension| extension != "toml" && extension != "md")
    {
        return Err(cheese_error!(
            "from_file cannot load file {filename:?} with unknown extension"
        ));
    }

    // Create the file info components right at the start
    let dirname = filename
        .parent()
        .ok_or(cheese_error!(
            "filename supplied to from_file should have a dirname component",
        ))?
        .to_path_buf();

    let basename = filename
        .file_name()
        .ok_or(cheese_error!(
            "filename supplied to from_file should have a basename component",
        ))?
        .to_owned();

    let mut modified = false;

    // If the filename is a directory, we need to look for the underlying file
    let underlying_file = match filename.is_dir() {
        true => filename.join(FOLDER_METADATA_FILE_NAME),
        false => filename.to_path_buf(),
    };

    let (metadata_str, file_body) = read_file_contents(&underlying_file).or_else(|err| {
        if filename.is_dir() {
            Ok(("".to_string(), None))
        } else {
            Err(cheese_error!(
                "Failed to read file {underlying_file:?}: {err}"
            ))
        }
    })?;

    let mut metadata = FileObjectMetadata::default();

    let toml_header = metadata_str
        .parse::<DocumentMut>()
        .map_err(|err| cheese_error!("Error parsing {underlying_file:?}: {err}"))?;

    if !toml_header.contains_key("name") {
        let file_name = PathBuf::from(&basename)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let name_to_parse = if let Some((prefix, suffix)) = file_name.split_once('-') {
            match prefix.parse::<i64>() {
                Ok(_) => suffix,
                Err(_) => file_name.as_str(),
            }
        } else {
            file_name.as_str()
        };

        metadata.name = name_to_parse.replace("_", " ").trim().to_string();
        if !metadata.name.is_empty() {
            modified = true;
        }
    }

    let file_type_identifier = match toml_header.get("file_type") {
        Some(file_type_toml_item) => match file_type_toml_item.as_str() {
            Some(file_type_str) => Some(file_type_str),
            None => {
                return Err(cheese_error!(
                    "file header contained non-string value for file_type: {filename:?}"
                ));
            }
        },
        None => None,
    };

    let file_type: FileType = schema.resolve_type(filename, file_type_identifier)?;

    let mut children = Vec::new();

    // Load children of this file object
    if file_type.is_folder() {
        if !filename.is_dir() {
            return Err(cheese_error!(
                "{filename:?} has a folder-based file_type, but isn't actually a directory",
            ));
        }

        // We rescan and fix the indexing at the end when returning a folder or place, so we can read
        // the files in any order here. The only files that won't ever be affected by this are the
        // roots, which don't have indexing anyway
        for file in std::fs::read_dir(filename).map_err(|err| {
            cheese_error!("Error while attempt to read folder {filename:?}: {err}")
        })? {
            match file {
                Ok(file) => {
                    // We've already read this file, nothing to do
                    if file.file_name() == FOLDER_METADATA_FILE_NAME {
                        continue;
                    }

                    let file_path = file.path();

                    // Just read the children in any order, we'll clean it up later
                    match load_file(schema, &file_path, objects) {
                        Ok(child_id) => children.push(child_id.clone()),
                        Err(err) => log::debug!("Could not load child {file:?}: {err}"),
                    }
                }
                Err(err) => log::warn!("Could not read file {filename:?}: {err}"),
            }
        }
    }

    let index = get_index_from_name(&basename.to_string_lossy());

    // Check if we're loading a file object that we already know about
    if let Some(existing_file_id) = toml_header
        .get("id")
        .and_then(|id_item| id_item.as_str())
        .map(|id_str| FileID::new(id_str.to_owned()))
        && objects.contains_key(&existing_file_id)
    {
        // we just update the object in place
        let mut file_object = objects.get(&existing_file_id).unwrap().borrow_mut();
        file_object.get_base_mut().children = children;

        file_object.get_base_mut().file.dirname = dirname;
        file_object.get_base_mut().file.basename = basename;

        file_object.get_base_mut().index = index;

        file_object.reload_file()?;

        Ok(existing_file_id)
    } else {
        // we need to create a new object

        let mut file_info = FileInfo {
            dirname,
            basename,
            modtime: None,
            modified,
        };

        load_base_metadata(toml_header.as_table(), &mut metadata, &mut file_info)
            .map_err(|err| cheese_error!("Error while parsing metadata for {filename:?}: {err}"))?;

        let base = BaseFileObject {
            metadata,
            index,
            file: file_info,
            toml_header,
            children,
        };

        let file_id = base.metadata.id.clone();

        let boxed_object = schema.load_file_object(file_type, base, file_body)?;

        boxed_object.borrow_mut().rescan_indexing(objects);

        objects.insert(file_id.clone(), boxed_object);

        Ok(file_id)
    }
}

pub fn create_file(
    file_type: FileType,
    schema: &dyn Schema,
    dirname: PathBuf,
    index: usize,
) -> Result<Box<RefCell<dyn FileObject>>, CheeseError> {
    let base = BaseFileObject::new(dirname, Some(index));

    let file_object = schema.init_file_object(file_type, base)?;

    let mut fo_mut = file_object.borrow_mut();

    fo_mut.get_base_mut().file.basename = fo_mut.calculate_filename();

    if file_type.is_folder() {
        create_dir(fo_mut.get_path())?;
    }

    fo_mut.save(&HashMap::new())?;

    drop(fo_mut);

    Ok(file_object)
}

/// Creates a top level folder (one that doesn't have an index) based on the name. The name will
/// be used directly in the metadata, but convereted to lowercase for the version on disk
pub fn create_top_level_folder(
    schema: &dyn Schema,
    dirname: PathBuf,
    name: &str,
) -> Result<Box<RefCell<dyn FileObject>>, CheeseError> {
    let file_type = schema.get_top_level_folder_type();
    assert!(file_type.is_folder());

    let mut base = BaseFileObject::new(dirname, None);

    base.metadata.name = name.to_string();
    base.file.basename = OsString::from(name.to_lowercase());

    let file_object = schema.init_file_object(file_type, base)?;

    let mut fo_mut = file_object.borrow_mut();

    create_dir(fo_mut.get_path())
        .map_err(|err| cheese_error!("Failed to create top-level directory: {}: {err}", name))?;

    fo_mut.save(&HashMap::new()).map_err(|err| {
        cheese_error!(
            "Failed to save newly created top level directory: {}: {err}",
            name
        )
    })?;

    drop(fo_mut);

    Ok(file_object)
}

impl BaseFileObject {
    /// Create a new file object in a folder
    pub fn new(dirname: PathBuf, index: Option<usize>) -> Self {
        Self {
            metadata: FileObjectMetadata::default(),
            index,
            file: FileInfo {
                dirname,
                basename: OsString::new(),
                modtime: None,
                modified: true, // Newly added files are modified (they don't exist on disk)
            },
            toml_header: DocumentMut::new(),
            children: Vec::new(),
        }
    }

    fn write_metadata(&mut self) {
        self.toml_header["file_format_version"] = toml_edit::value(self.metadata.version as i64);
        self.toml_header["name"] = toml_edit::value(&self.metadata.name);
        self.toml_header["id"] = toml_edit::value(&*self.metadata.id);
    }
}
impl std::fmt::Display for dyn FileObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[File Object | name=\"{}\" | id={}]",
            self.get_title(),
            self.id()
        )
    }
}
