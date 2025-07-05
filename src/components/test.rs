#[cfg(test)]
use crate::components::file_objects::{
    Character, FileInfo, FileObject, FileObjectMetadata, FileObjectStore, Folder, Place, Scene,
    from_file,
};
#[cfg(test)]
use crate::components::project::Project;
#[cfg(test)]
use std::collections::HashMap;
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
fn test_basic_create_project() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let project_name = "test project";
    let project_path = base_dir.path().join("test_project");

    assert!(!project_path.exists());
    assert_eq!(read_dir(base_dir.path()).unwrap().count(), 0);

    let project = Project::new(base_dir.path().to_path_buf(), project_name.to_string()).unwrap();

    assert_eq!(project_path, project.get_path());

    assert_eq!(read_dir(base_dir.path()).unwrap().count(), 1);
    assert!(project_path.exists());
    assert_eq!(read_dir(&project_path).unwrap().count(), 4);

    // Ensure that the file is at least populated
    assert_ne!(
        read_to_string(project.get_project_info_file())
            .unwrap()
            .len(),
        0
    );
}

#[test]
/// Ensure that file_objects are created properly
fn test_basic_create_file_object() -> Result<()> {
    let base_dir = tempfile::TempDir::new()?;

    let scene = Scene::new(base_dir.path().to_path_buf(), 0).unwrap();
    let character = Character::new(base_dir.path().to_path_buf(), 0).unwrap();
    let folder = Folder::new(base_dir.path().to_path_buf(), 0).unwrap();
    let place = Place::new(base_dir.path().to_path_buf(), 0).unwrap();

    // Ensure that all four of the files exist in the proper place
    assert_eq!(read_dir(base_dir.path()).unwrap().count(), 4);
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

    // Ensure that folders are created with the metadata.toml file
    assert_eq!(read_dir(folder.get_path()).unwrap().count(), 1);
    assert_eq!(read_dir(place.get_path()).unwrap().count(), 1);

    // Ensure that the files contain stuff
    assert_ne!(read_to_string(scene.get_file()).unwrap().len(), 0);
    assert_ne!(read_to_string(character.get_file()).unwrap().len(), 0);
    assert_ne!(read_to_string(folder.get_file()).unwrap().len(), 0);
    assert_ne!(read_to_string(place.get_file()).unwrap().len(), 0);

    Ok(())
}

#[test]
/// Ensure that top level folders work the way we want
fn test_create_top_level_folder() -> Result<()> {
    let base_dir = tempfile::TempDir::new()?;

    let text = Folder::new_top_level(base_dir.path().to_path_buf(), "text".to_string())?;

    assert_eq!(read_dir(base_dir.path())?.count(), 1);
    assert_eq!(read_dir(text.get_path())?.count(), 1);

    assert_eq!(text.get_path().file_name().unwrap(), "text");
    assert_eq!(text.get_base().index, None);

    Ok(())
}

#[test]
/// Ensure names actually get truncated when saving (there are other tests that cover truncation
/// behavior in more depth), and that names get characters removed
fn test_complicated_file_object_names() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut scene = Scene::new(base_dir.path().to_path_buf(), 0).unwrap();
    let scene1 = Scene::new(base_dir.path().to_path_buf(), 1).unwrap();
    scene.get_base_mut().metadata.name =
        "This is a really long scene name that will have to be shortened".to_string();
    scene.get_base_mut().file.modified = true;

    scene.save(&mut HashMap::new()).unwrap();

    // This is probably getting too far into specifics of behavior for this test,
    // but I don't think it'll change very much, so I'm writing it like this now
    assert_eq!(
        scene.get_file().file_name().unwrap(),
        "000-This_is_a_really_long_scene.md"
    );

    assert_eq!(read_dir(base_dir.path()).unwrap().count(), 2);
    assert!(scene.get_file().exists());
    assert_ne!(read_to_string(scene.get_file()).unwrap().len(), 0);

    scene.get_base_mut().metadata.name = "Difficult(to)ParseName/Bad_ ".to_string();
    scene.get_base_mut().file.modified = true;
    scene.save(&mut HashMap::new()).unwrap();

    assert_eq!(read_dir(base_dir.path()).unwrap().count(), 2);
    assert!(scene.get_file().exists());
    assert_ne!(read_to_string(scene.get_file()).unwrap().len(), 0);

    assert_eq!(
        scene.get_file().file_name().unwrap(),
        "000-Difficult(to)ParseName-Bad_.md"
    );

    // At the end, ensure we didn't clobber the other scene somehow
    assert_eq!(
        scene1.get_base().file.basename,
        OsString::from("001-New_Scene.md")
    );
    assert!(scene1.get_file().exists());
    assert_ne!(read_to_string(scene1.get_file()).unwrap().len(), 0);
}

#[test]
fn test_change_index_scene() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let mut scene = Scene::new(base_dir.path().to_path_buf(), 0).unwrap();
    let scene1 = Scene::new(base_dir.path().to_path_buf(), 1).unwrap();

    scene.text = "sample scene text".to_string();
    scene.get_base_mut().file.modified = true;
    scene.save(&mut HashMap::new()).unwrap();

    scene.set_index(2, &mut HashMap::new()).unwrap();

    // Make sure the untouched scene didn't change somehow
    assert_eq!(
        scene1.get_base().file.basename,
        OsString::from("001-New_Scene.md")
    );
    assert!(scene1.get_file().exists());
    assert_ne!(read_to_string(scene1.get_file()).unwrap().len(), 0);

    // Make sure the moved scene is at the expected path
    assert_eq!(scene.get_base().index, Some(2));
    assert_eq!(
        scene.get_base().file.basename,
        OsString::from("002-New_Scene.md")
    );
    assert!(scene.get_file().exists());

    // Make sure the file contents moved with it
    let scene_text_full = read_to_string(scene.get_file()).unwrap();
    assert_ne!(scene_text_full.len(), 0);
    assert!(scene_text_full.contains("sample scene text"));
}
