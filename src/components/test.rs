#[cfg(test)]
use crate::components::file_objects::{
    Character, FileInfo, FileObject, FileObjectMetadata, FileObjectStore, Folder, Place, Scene,
    from_file,
};
#[cfg(test)]
use crate::components::project::Project;
#[cfg(test)]
use std::ffi::OsString;
#[cfg(test)]
use std::fs::{read_dir, read_to_string};
#[cfg(test)]
use std::io::{Error, ErrorKind, Result};
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[test]
/// Ensure that projects are created properly
fn test_basic_create_project() -> Result<()> {
    let base_dir = tempfile::TempDir::new()?;
    let project_name = "test_project";
    let project_path = base_dir.path().join(project_name);

    assert!(!project_path.exists());
    assert_eq!(read_dir(base_dir.path())?.count(), 0);

    let mut project = Project::new(base_dir.path().to_path_buf(), project_name.to_string())?;
    project.save()?;

    assert_eq!(read_dir(base_dir.path())?.count(), 1);
    assert!(project_path.exists());
    assert_eq!(read_dir(&project_path)?.count(), 4);

    let project_toml_contents = read_to_string(project_path.join("project.toml"))?;

    // Ensure that the file is populated at least
    assert!(project_toml_contents.len() != 0);

    Ok(())
}

#[test]
/// Ensure that file_objects are created properly
fn test_basic_create_file_object() -> Result<()> {
    let base_dir = tempfile::TempDir::new()?;

    let scene = Scene::new(base_dir.path().to_path_buf(), 0)?;
    let character = Character::new(base_dir.path().to_path_buf(), 0)?;
    let folder = Folder::new(base_dir.path().to_path_buf(), 0)?;
    let place = Place::new(base_dir.path().to_path_buf(), 0)?;

    assert_eq!(read_dir(base_dir.path())?.count(), 4);
    assert_eq!(
        scene.get_base().file.basename,
        OsString::from("000-New_Scene.md")
    );
    assert_eq!(
        character.get_base().file.basename,
        OsString::from("000-New_Character.toml")
    );
    assert_eq!(
        folder.get_base().file.basename,
        OsString::from("000-New_Folder")
    );
    assert_eq!(
        place.get_base().file.basename,
        OsString::from("000-New_Place")
    );

    Ok(())
}
