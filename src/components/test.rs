#[cfg(test)]
use crate::components::file_objects::base::{FileObjectCreation, FileType};
#[cfg(test)]
use crate::components::file_objects::{
    Character, FileObject, FileObjectTypeInterface, Folder, MutFileObjectTypeInterface, Place,
    Scene, from_file, write_with_temp_file,
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

#[test]
fn test_create_child() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();
    let scene = project.text.create_child(FileType::Scene).unwrap();
    let character = project.text.create_child(FileType::Character).unwrap();
    let folder = project.text.create_child(FileType::Folder).unwrap();
    let place = project.text.create_child(FileType::Place).unwrap();

    // Four file objects plus the metadata
    assert_eq!(read_dir(project.text.get_path()).unwrap().count(), 5);
    assert!(scene.get_file().exists());
    assert_eq!(scene.get_base().index, Some(0));
    assert!(character.get_file().exists());
    assert_eq!(character.get_base().index, Some(1));
    assert!(folder.get_file().exists());
    assert_eq!(folder.get_base().index, Some(2));
    assert!(place.get_file().exists());
    assert_eq!(place.get_base().index, Some(3));
}

#[test]
fn test_set_index_folders() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();
    let mut top_level_folder = project.text.create_child(FileType::Folder).unwrap();
    let mut mid_level_folder = top_level_folder.create_child(FileType::Folder).unwrap();
    let child_scene = mid_level_folder.create_child(FileType::Scene).unwrap();
    let child_scene_id = child_scene.get_base().metadata.id.clone();

    assert!(child_scene.get_file().exists());
    assert_eq!(child_scene.get_base().index, Some(0));

    project.add_object(mid_level_folder);
    project.add_object(child_scene);

    top_level_folder.set_index(1, &mut project.objects).unwrap();

    let (_child_string, child) = project.objects.remove_entry(&child_scene_id).unwrap();

    assert_eq!(child.get_base().index, Some(0));
    assert!(child.get_file().exists());

    assert!(
        child
            .get_path()
            .ends_with("000-New_Folder/000-New_Scene.md")
    );
}

#[test]
/// Run save on a folder and scene without changing anything, ensure that they don't get re-writtten
fn test_avoid_pointless_save() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut scene = Scene::new(base_dir.path().to_path_buf(), 0).unwrap();
    let scene_old_modtime = scene.get_base().file.modtime;
    // Check that we get the correct modtime
    assert_eq!(scene.get_base().file.modtime, scene_old_modtime);

    // Try to save again, we shouldn't do anything
    scene.save(&mut HashMap::new()).unwrap();
    assert_eq!(scene.get_base().file.modtime, scene_old_modtime);

    let mut folder = Folder::new(base_dir.path().to_path_buf(), 1).unwrap();
    let folder_old_modtime = folder.get_base().file.modtime;
    folder.save(&mut HashMap::new()).unwrap();
    assert_eq!(folder.get_base().file.modtime, folder_old_modtime);
}

#[test]
fn test_save_in_folder() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let sample_text = "sample body";

    let mut folder = Folder::new(base_dir.path().to_path_buf(), 0).unwrap();
    let mut scene = folder.create_child(FileType::Scene).unwrap();

    match scene.get_file_type_mut() {
        MutFileObjectTypeInterface::Scene(scene) => {
            scene.text.push_str(sample_text);
        }
        _ => panic!(),
    }
    scene.get_base_mut().file.modified = true;

    let scene_id = scene.get_base().metadata.id.clone();

    let mut map: HashMap<String, Box<dyn FileObject>> = HashMap::new();
    map.insert(scene.get_base().metadata.id.clone(), scene);

    folder.save(&mut map).unwrap();

    let scene = map.get(&scene_id).unwrap();
    assert!(!scene.get_base().file.modified);
    assert!(scene.get_file().exists());
    assert!(
        read_to_string(scene.get_file())
            .unwrap()
            .contains(sample_text)
    );
}

#[test]
fn test_reload_project() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let sample_body = "sample body";
    let character_appearance = "tall";
    let folder_notes = "this is a folder";
    let place_description = "lots and lots of trees!";

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();
    let mut scene = project.text.create_child(FileType::Scene).unwrap();
    let scene_id = scene.get_base().metadata.id.clone();
    let mut character = project
        .characters
        .create_child(FileType::Character)
        .unwrap();
    let character_id = character.get_base().metadata.id.clone();
    let mut folder = project.text.create_child(FileType::Folder).unwrap();
    let folder_id = folder.get_base().metadata.id.clone();
    let mut place = project.worldbuilding.create_child(FileType::Place).unwrap();
    let place_id = place.get_base().metadata.id.clone();

    // modify the file objects:
    match scene.get_file_type_mut() {
        MutFileObjectTypeInterface::Scene(scene) => {
            scene.text.push_str(sample_body);
        }
        _ => panic!(),
    }
    scene.get_base_mut().file.modified = true;

    match character.get_file_type_mut() {
        MutFileObjectTypeInterface::Character(character) => {
            character.metadata.appearance = character_appearance.to_string();
        }
        _ => panic!(),
    }
    character.get_base_mut().file.modified = true;

    match folder.get_file_type_mut() {
        MutFileObjectTypeInterface::Folder(folder) => {
            folder.metadata.notes = folder_notes.to_string()
        }
        _ => panic!(),
    }
    folder.get_base_mut().file.modified = true;

    match place.get_file_type_mut() {
        MutFileObjectTypeInterface::Place(place) => {
            place.metadata.description = place_description.to_string()
        }
        _ => panic!(),
    }
    place.get_base_mut().file.modified = true;

    project.add_object(scene);
    project.add_object(character);
    project.add_object(folder);
    project.add_object(place);

    project.save().unwrap();

    let project_path = project.get_path();

    drop(project);

    let project = Project::load(project_path).unwrap();
    let scene = project.objects.get(&scene_id).unwrap();
    let character = project.objects.get(&character_id).unwrap();
    let folder = project.objects.get(&folder_id).unwrap();
    let place = project.objects.get(&place_id).unwrap();

    // Go through each folder:
    // Text (scene, folder)
    assert_eq!(read_dir(project.text.get_path()).unwrap().count(), 3);

    assert!(scene.get_file().exists());
    assert_eq!(scene.get_base().index, Some(0));
    assert!(scene.get_body().contains(sample_body));

    assert!(folder.get_file().exists());
    assert_eq!(folder.get_base().index, Some(1));
    match folder.get_file_type() {
        FileObjectTypeInterface::Folder(folder) => {
            assert_eq!(folder.metadata.notes, folder_notes);
        }
        _ => panic!(),
    }

    // Characters (character)
    assert_eq!(read_dir(project.characters.get_path()).unwrap().count(), 2);
    assert!(character.get_file().exists());
    assert_eq!(character.get_base().index, Some(0));
    match character.get_file_type() {
        FileObjectTypeInterface::Character(character) => {
            assert_eq!(character.metadata.appearance, character_appearance);
        }
        _ => panic!(),
    }

    // Worldbuilding (place)
    assert_eq!(
        read_dir(project.worldbuilding.get_path()).unwrap().count(),
        2
    );
    assert!(place.get_file().exists());
    assert_eq!(place.get_base().index, Some(0));
    match place.get_file_type() {
        FileObjectTypeInterface::Place(place) => {
            assert_eq!(place.metadata.description, place_description);
        }
        _ => panic!(),
    }
}

/// Make sure that a `.md` file gets loaded without a text editor
#[test]
fn test_load_markdown() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let sample_body = "sample body";

    // open and immediately drop the project (just creating the files)
    Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    write_with_temp_file(
        &Path::join(base_dir.path(), "test_project/text/000-New_Scene.md"),
        sample_body.as_bytes(),
    )
    .unwrap();

    let project = Project::load(base_dir.path().join("test_project")).unwrap();

    let mut values: Vec<_> = project.objects.values().collect();
    let scene = values.pop().unwrap();
    assert_eq!(scene.get_body().trim(), sample_body);
}

/// Make sure metadata gets filled in
#[test]
fn test_load_partial_metadata() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let file_text = r#"name = "Other title"
summary = """multiline block inside
another multiline block
"""
++++++++
contents1
"#;
    // open and immediately drop the project (just creating the files)
    Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    write_with_temp_file(
        &Path::join(base_dir.path(), "test_project/text/000-New_Scene.md"),
        file_text.as_bytes(),
    )
    .unwrap();

    let mut project = Project::load(base_dir.path().join("test_project")).unwrap();
    project.save().unwrap();

    let mut values: Vec<_> = project.objects.values().collect();
    let scene = values.pop().unwrap();
    assert_eq!(scene.get_body().trim(), "contents1");
    match scene.get_file_type() {
        FileObjectTypeInterface::Scene(scene) => {
            assert_eq!(
                scene.metadata.summary,
                "multiline block inside\nanother multiline block\n"
            );
        }
        _ => panic!(),
    }

    assert!(
        read_to_string(scene.get_file())
            .unwrap()
            .contains(r#"notes = """#)
    );
}

/// Make sure that the filename is kept when an object gets renamed
#[test]
fn test_name_from_filename() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let text_path = Folder::new_top_level(base_dir.path().to_path_buf(), "text".to_string())
        .unwrap()
        .get_path();

    write_with_temp_file(
        &text_path.join("4-scene2.md"),
        "contents1".to_string().as_bytes(),
    )
    .unwrap();

    match from_file(&text_path, None).unwrap() {
        FileObjectCreation::Folder(mut folder, mut contents) => {
            folder.save(&mut contents).unwrap();
            assert!(folder.get_path().join("000-scene2.md").exists());
        }
        _ => panic!(),
    };
}

/// Load various files with indexes out of order (and some missing) and verify that they all get indexed correctly
#[test]
fn test_fix_indexing_on_load() {
    // Create files with known id for convenience, verify that the children dict ends up in the expect place
    // after loading

    let base_dir = tempfile::TempDir::new().unwrap();

    let text_path = Folder::new_top_level(base_dir.path().to_path_buf(), "text".to_string())
        .unwrap()
        .get_path();

    write_with_temp_file(
        &text_path.join("4-scene2.md"),
        r#"id = "0"
++++++++"#
            .to_string()
            .as_bytes(),
    )
    .unwrap();

    std::fs::create_dir(text_path.join("05-dir")).unwrap();

    write_with_temp_file(
        &text_path.join("05-dir/metadata.toml"),
        r#"id = "1""#.to_string().as_bytes(),
    )
    .unwrap();

    write_with_temp_file(
        &text_path.join("05-dir/2-scene.md"),
        r#"id = "1-0"
++++++++
contents123"#
            .to_string()
            .as_bytes(),
    )
    .unwrap();

    write_with_temp_file(
        &text_path.join("10-scene2.md"),
        r#"id = "2"
++++++++"#
            .to_string()
            .as_bytes(),
    )
    .unwrap();

    write_with_temp_file(
        &text_path.join("scene_no_index.md"),
        r#"id = "3"
++++++++"#
            .to_string()
            .as_bytes(),
    )
    .unwrap();

    match from_file(&text_path, None).unwrap() {
        FileObjectCreation::Folder(mut folder, mut contents) => {
            folder.save(&mut contents).unwrap();
            assert_eq!(folder.base.children, vec!["0", "1", "2", "3"]);
            let child = contents.get("1-0").unwrap();
            assert_eq!(child.get_base().index, Some(0));
            assert_eq!(child.get_body(), "contents123\n");
        }
        _ => panic!(),
    };
}

/// Try to delete a file object, verifying it gets removed from disk
#[test]
fn test_delete() {
    unimplemented!()
}

/// Try to delete a file object in the middle of a folder, ensuring indexing works correctly afterwards
#[test]
fn test_delete_middle() {
    unimplemented!()
}

/// Simple move, move a scene from the end of one folder to the end of another
#[test]
fn test_move_simple() {
    unimplemented!()
}

/// Move a folder that contains things
#[test]
fn test_move_folder_contents() {
    unimplemented!()
}

/// Move an object within a folder (forwards and backwards)
#[test]
fn test_move_within_folder() {
    unimplemented!()
}

/// Move an object to its parent
#[test]
fn test_move_to_parent() {
    unimplemented!()
}

/// Move something where it already is (should be no-op)
#[test]
fn test_move_to_self() {
    unimplemented!()
}

/// Try to move a folder into one of it's (distant) children, verify that it does not allow it
#[test]
fn test_move_to_child() {
    unimplemented!()
}

/// Make sure places can nest
#[test]
fn test_place_nesting() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut text =
        Folder::new_top_level(base_dir.path().to_path_buf(), "text".to_string()).unwrap();

    let mut place1 = text.create_child(FileType::Place).unwrap();

    let place2 = place1.create_child(FileType::Place).unwrap();

    assert!(place2.get_file().exists());
    assert_eq!(place1.get_base().index, Some(0));
    assert_eq!(place2.get_base().index, Some(0));
}
