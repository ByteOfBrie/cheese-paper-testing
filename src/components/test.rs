#[cfg(test)]
use crate::components::file_objects::base::{FileObjectCreation, FileType};
#[cfg(test)]
use crate::components::file_objects::{
    Character, FileObject, Folder, Place, Scene, from_file, move_child, run_with_file_object,
    write_with_temp_file,
};
#[cfg(test)]
use crate::components::project::{Project, ProjectFolder};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::ffi::OsString;
#[cfg(test)]
use std::fs::{read_dir, read_to_string};
#[cfg(test)]
use std::io::Result;
#[cfg(test)]
use std::path::Path;

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

    scene.text = "sample scene text".to_string().into();
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

    // create the scenes
    let scene = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    let character = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Character)
        })
        .unwrap();
    let folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    let place = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Place)
        })
        .unwrap();

    // Four file objects plus the metadata
    assert_eq!(
        read_dir(project.run_with_folder(ProjectFolder::text, |text, _| { text.get_path() }))
            .unwrap()
            .count(),
        5
    );
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

    let mut top_level_folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();

    let mut mid_level_folder = top_level_folder
        .create_child_at_end(FileType::Folder)
        .unwrap();
    let child_scene = mid_level_folder
        .create_child_at_end(FileType::Scene)
        .unwrap();
    let child_scene_id = child_scene.get_base().metadata.id.clone();

    assert!(child_scene.get_file().exists());
    assert_eq!(child_scene.get_base().index, Some(0));

    project.add_object(mid_level_folder);
    project.add_object(child_scene);

    top_level_folder.set_index(1, &mut project.objects).unwrap();

    let child = project.objects.remove(&child_scene_id).unwrap();

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
    let mut scene = folder.create_child_at_end(FileType::Scene).unwrap();

    scene.load_body(sample_text.to_owned());
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
fn test_reload_objects() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let sample_body = "sample body";
    let character_appearance = "tall";
    let folder_notes = "this is a folder";
    let place_description = "lots and lots of trees!";

    let mut scene = Scene::new(base_dir.path().to_path_buf(), 0).unwrap();
    let mut folder = Folder::new(base_dir.path().to_path_buf(), 1).unwrap();
    let mut character = Character::new(base_dir.path().to_path_buf(), 2).unwrap();
    let mut place = Place::new(base_dir.path().to_path_buf(), 3).unwrap();

    scene.text = sample_body.to_string().into();
    scene.get_base_mut().file.modified = true;

    character.metadata.appearance = character_appearance.to_string().into();
    character.get_base_mut().file.modified = true;

    folder.metadata.notes = folder_notes.to_string().into();
    folder.get_base_mut().file.modified = true;

    place.metadata.description = place_description.to_string().into();
    place.get_base_mut().file.modified = true;

    // Save all of the objects
    scene.save(&mut HashMap::new()).unwrap();
    character.save(&mut HashMap::new()).unwrap();
    folder.save(&mut HashMap::new()).unwrap();
    place.save(&mut HashMap::new()).unwrap();

    // Keep track of paths
    let scene_path = scene.get_path();
    let character_path = character.get_path();
    let folder_path = folder.get_path();
    let place_path = place.get_path();

    // Drop all of the objects (just to make sure we're reloading them)
    drop(scene);
    drop(character);
    drop(folder);
    drop(place);

    match from_file(&scene_path, Some(0)).unwrap() {
        FileObjectCreation::Scene(scene, _) => {
            assert_eq!(*scene.text, sample_body);
        }
        _ => panic!(),
    }

    match from_file(&character_path, Some(1)).unwrap() {
        FileObjectCreation::Character(character, _) => {
            assert_eq!(*character.metadata.appearance, character_appearance);
        }
        _ => panic!(),
    }

    match from_file(&folder_path, Some(2)).unwrap() {
        FileObjectCreation::Folder(folder, _) => {
            assert_eq!(*folder.metadata.notes, folder_notes);
        }
        _ => panic!(),
    }

    match from_file(&place_path, Some(3)).unwrap() {
        FileObjectCreation::Place(place, _) => {
            assert_eq!(*place.metadata.description, place_description);
        }
        _ => panic!(),
    }
}

#[test]
fn test_reload_project() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let sample_body = "sample body";

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut scene = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    let scene_id = scene.get_base().metadata.id.clone();

    let character = project
        .run_with_folder(ProjectFolder::characters, |text, _| {
            text.create_child_at_end(FileType::Character)
        })
        .unwrap();
    let character_id = character.get_base().metadata.id.clone();

    let folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    let folder_id = folder.get_base().metadata.id.clone();

    let place = project
        .run_with_folder(ProjectFolder::worldbuilding, |text, _| {
            text.create_child_at_end(FileType::Place)
        })
        .unwrap();
    let place_id = place.get_base().metadata.id.clone();

    // modify the file objects:
    scene.load_body(sample_body.to_string());
    scene.get_base_mut().file.modified = true;

    project.add_object(scene);
    project.add_object(character);
    project.add_object(folder);
    project.add_object(place);

    project.save().unwrap();

    let project_path = project.get_path();

    drop(project);

    let mut project = Project::load(project_path).unwrap();

    // Verify the counts in each folder are correct:
    // Text (scene, folder + metadata)
    assert_eq!(
        read_dir(project.run_with_folder(ProjectFolder::text, |text, _| { text.get_path() }))
            .unwrap()
            .count(),
        3
    );

    // Characters (character + metadata)
    assert_eq!(
        read_dir(
            project.run_with_folder(ProjectFolder::characters, |characters, _| {
                characters.get_path()
            })
        )
        .unwrap()
        .count(),
        2
    );

    // Worldbuilding (place + metadata)
    assert_eq!(
        read_dir(
            project.run_with_folder(ProjectFolder::worldbuilding, |worldbuilding, _| {
                worldbuilding.get_path()
            })
        )
        .unwrap()
        .count(),
        2
    );

    // Now inspect the file objects
    let scene = project.objects.get(&scene_id).unwrap();
    let character = project.objects.get(&character_id).unwrap();
    let folder = project.objects.get(&folder_id).unwrap();
    let place = project.objects.get(&place_id).unwrap();

    // Go through each folder:
    // Text (scene, folder + metadata)
    assert!(scene.get_file().exists());
    assert_eq!(scene.get_base().index, Some(0));
    assert!(scene.get_body().contains(sample_body));

    assert!(folder.get_file().exists());
    assert_eq!(folder.get_base().index, Some(1));

    // Characters (character + metadata)
    assert!(character.get_file().exists());
    assert_eq!(character.get_base().index, Some(0));

    // Worldbuilding (place + metadata)
    assert!(place.get_file().exists());
    assert_eq!(place.get_base().index, Some(0));
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

    let mut project = Project::load(base_dir.path().join("test_project")).unwrap();

    let text_child = project.run_with_folder(ProjectFolder::text, |object, _| {
        object.get_base().children.first().unwrap().clone()
    });

    assert_eq!(
        project.objects.get(&text_child).unwrap().get_body().trim(),
        sample_body
    );
}

/// Make sure metadata gets filled in
#[test]
fn test_load_partial_metadata() {
    let base_dir = tempfile::TempDir::new().unwrap();
    let file_text = r#"id = "1"
name = "Other title"
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

    let scene_path = project.objects.get("1").unwrap().get_path();

    match from_file(&scene_path, Some(0)).unwrap() {
        FileObjectCreation::Scene(scene, _) => {
            assert_eq!(scene.get_body().trim(), "contents1");
            assert_eq!(
                *scene.metadata.summary,
                "multiline block inside\nanother multiline block\n"
            );
        }
        _ => panic!(),
    }

    assert!(
        read_to_string(scene_path)
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
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut scene1 = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene1.get_base_mut().metadata.name = "scene1".to_string();
    scene1.get_base_mut().file.modified = true;

    let mut scene2 = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene2.get_base_mut().metadata.name = "scene2".to_string();
    scene2.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let scene1_id = scene1.get_base().metadata.id.clone();
    let scene2_id = scene2.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(scene1);
    project.add_object(scene2);
    project.save().unwrap();

    // baseline:
    assert!(project.get_path().join("text/000-folder1/").exists());

    assert!(
        project
            .get_path()
            .join("text/000-folder1/000-scene1.md")
            .exists()
    );

    assert!(
        project
            .get_path()
            .join("text/000-folder1/001-scene2.md")
            .exists()
    );

    run_with_file_object(&folder1_id, &mut project.objects, |folder, objects| {
        folder.remove_child(&scene2_id, objects)
    })
    .unwrap();

    // we should have removed the ending scene, check on disk
    assert!(project.get_path().join("text/000-folder1/").exists());

    assert!(
        project
            .get_path()
            .join("text/000-folder1/000-scene1.md")
            .exists()
    );

    assert!(
        !project
            .get_path()
            .join("text/000-folder1/001-scene2.md")
            .exists()
    );

    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene1_id
    );

    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    assert!(!project.objects.contains_key(&scene2_id));

    // Now, try to remove the folder
    project
        .run_with_folder(ProjectFolder::text, |text, objects| {
            text.remove_child(&folder1_id, objects)
        })
        .unwrap();

    // we should have removed the ending scene, check on disk
    assert!(project.get_path().join("text/metadata.toml").exists());

    assert!(!project.get_path().join("text/000-folder1/").exists());

    assert!(
        !project
            .get_path()
            .join("text/000-folder1/001-scene2.md")
            .exists()
    );

    assert_eq!(
        project
            .objects
            .get(&project.text_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    assert!(!project.objects.contains_key(&scene1_id));
    assert!(!project.objects.contains_key(&folder1_id));
}

/// Try to delete a file object in the middle of a folder, ensuring indexing works correctly afterwards
#[test]
fn test_delete_middle() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut scene1 = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene1.get_base_mut().metadata.name = "scene1".to_string();
    scene1.get_base_mut().file.modified = true;

    let mut scene2 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    scene2.get_base_mut().metadata.name = "scene2".to_string();
    scene2.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let scene2_id = scene2.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(scene2);
    project.add_object(scene1);
    project.save().unwrap();

    project
        .run_with_folder(ProjectFolder::text, |text, objects| {
            text.remove_child(&folder1_id, objects)
        })
        .unwrap();

    assert!(!project.get_path().join("text/000-folder1/").exists());
    assert!(project.get_path().join("text/000-scene2.md").exists());

    assert_eq!(
        project
            .objects
            .get(&project.text_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    assert!(!project.objects.contains_key(&folder1_id));
    assert!(project.objects.contains_key(&scene2_id));
}

/// Simple move, move a scene from the end of one folder to the end of another
#[test]
fn test_move_simple() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut folder2 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder2.get_base_mut().metadata.name = "folder2".to_string();
    folder2.get_base_mut().file.modified = true;

    let mut scene_to_move = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene_to_move.get_base_mut().metadata.name = "scene1".to_string();
    scene_to_move.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let folder2_id = folder2.get_base().metadata.id.clone();
    let scene_id = scene_to_move.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(folder2);
    project.add_object(scene_to_move);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(!project_path.join("text/001-folder2/000-scene1.md").exists());

    // Do the move
    move_child(&scene_id, &folder1_id, &folder2_id, 0, &mut project.objects).unwrap();

    // Verify that the move happened on disk
    assert!(!project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(project_path.join("text/001-folder2/000-scene1.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );
}

/// Try moving file multiple times to make sure we're setting properties correctly
#[test]
fn test_move_multiple_times() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut folder2 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder2.get_base_mut().metadata.name = "folder2".to_string();
    folder2.get_base_mut().file.modified = true;

    let mut scene_to_move = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene_to_move.get_base_mut().metadata.name = "scene1".to_string();
    scene_to_move.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let folder2_id = folder2.get_base().metadata.id.clone();
    let scene_id = scene_to_move.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(folder2);
    project.add_object(scene_to_move);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(!project_path.join("text/001-folder2/000-scene1.md").exists());

    // Do the first move
    move_child(&scene_id, &folder1_id, &folder2_id, 0, &mut project.objects).unwrap();

    // Verify that the move happened on disk
    assert!(!project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(project_path.join("text/001-folder2/000-scene1.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    // Do the second move (back)
    move_child(&scene_id, &folder2_id, &folder1_id, 0, &mut project.objects).unwrap();

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    assert!(project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(!project_path.join("text/001-folder2/000-scene1.md").exists());
}

/// Move a folder that contains things. Almost the same as `test_move_simple`,
/// but moves the entire folder instead
#[test]
fn test_move_folder_contents() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let text_id = project.text_id.clone();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut folder2 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder2.get_base_mut().metadata.name = "folder2".to_string();
    folder2.get_base_mut().file.modified = true;

    let mut scene = folder2.create_child_at_end(FileType::Scene).unwrap();
    scene.get_base_mut().metadata.name = "scene1".to_string();
    scene.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let folder2_id = folder2.get_base().metadata.id.clone();
    let scene_id = scene.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(folder2);
    project.add_object(scene);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(!project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(project_path.join("text/001-folder2/000-scene1.md").exists());

    // Do the move (folder2 (which contains scene) into folder1)
    move_child(&folder2_id, &text_id, &folder1_id, 0, &mut project.objects).unwrap();

    // Verify that the move happened on disk:
    // 1. old folder isn't there
    assert!(!project_path.join("text/001-folder2").exists());
    // 2. folder got moved
    assert!(
        project_path
            .join("text/000-folder1/000-folder2/metadata.toml")
            .exists()
    );
    assert!(project_path.join("text/000-folder1/000-folder2").exists());
    // 3. scene got moved too
    assert!(
        project_path
            .join("text/000-folder1/000-folder2/000-scene1.md")
            .exists()
    );
    // 4. nothing happened to the folder2 metadata.toml
    assert!(project_path.join("text/000-folder1/metadata.toml").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    // Folder 1 contains folder2's ID
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &folder2_id
    );

    // Folder 2 has 1 child
    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    // Folder 2 contains scene's ID
    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene_id
    );
}

/// Move an object within a folder backwards (the easy case)
#[test]
fn test_move_within_folder_backwards() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let text_id = project.text_id.clone();

    let mut folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder.get_base_mut().metadata.name = "folder1".to_string();
    folder.get_base_mut().file.modified = true;

    let mut scene = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    scene.get_base_mut().metadata.name = "scene1".to_string();
    scene.get_base_mut().file.modified = true;

    let folder_id = folder.get_base().metadata.id.clone();
    let scene_id = scene.get_base().metadata.id.clone();

    project.add_object(folder);
    project.add_object(scene);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/").exists());
    assert!(project_path.join("text/001-scene1.md").exists());

    // Do the move, easy case first: moving scene1 backwards
    move_child(&scene_id, &text_id, &text_id, 0, &mut project.objects).unwrap();

    // Verify that the move happened on disk:
    assert!(project_path.join("text/000-scene1.md").exists());
    assert!(project_path.join("text/001-folder1/").exists());
    assert!(!project_path.join("text/000-folder1/").exists());
    assert!(!project_path.join("text/001-scene1.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project.objects.get(&folder_id).unwrap().get_base().index,
        Some(1)
    );
    assert_eq!(
        project.objects.get(&scene_id).unwrap().get_base().index,
        Some(0)
    );

    // Check that the values are properly ordered within the children
    assert_eq!(
        project.run_with_folder(ProjectFolder::text, |text, _| text
            .get_base()
            .children
            .get(1)
            .unwrap()
            .to_owned()),
        folder_id
    );

    assert_eq!(
        project.run_with_folder(ProjectFolder::text, |text, _| text
            .get_base()
            .children
            .first()
            .unwrap()
            .to_owned()),
        scene_id
    );
}

/// Move an object within a folder forwards (the hard case)
#[test]
fn test_move_within_folder_forwards() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let text_id = project.text_id.clone();

    let mut folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder.get_base_mut().metadata.name = "folder1".to_string();
    folder.get_base_mut().file.modified = true;

    let mut scene = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    scene.get_base_mut().metadata.name = "scene1".to_string();
    scene.get_base_mut().file.modified = true;

    let folder_id = folder.get_base().metadata.id.clone();
    let scene_id = scene.get_base().metadata.id.clone();

    project.add_object(folder);
    project.add_object(scene);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/").exists());
    assert!(project_path.join("text/001-scene1.md").exists());

    // Do the move
    move_child(&folder_id, &text_id, &text_id, 1, &mut project.objects).unwrap();

    // Verify that the move happened on disk:
    assert!(project_path.join("text/000-scene1.md").exists());
    assert!(project_path.join("text/001-folder1/").exists());
    assert!(!project_path.join("text/000-folder1/").exists());
    assert!(!project_path.join("text/001-scene1.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project.objects.get(&scene_id).unwrap().get_base().index,
        Some(0)
    );
    assert_eq!(
        project.objects.get(&folder_id).unwrap().get_base().index,
        Some(1)
    );

    // Check that the values are properly ordered within the children
    assert_eq!(
        project.run_with_folder(ProjectFolder::text, |text, _| text
            .get_base()
            .children
            .get(1)
            .unwrap()
            .to_owned()),
        folder_id
    );

    assert_eq!(
        project.run_with_folder(ProjectFolder::text, |text, _| text
            .get_base()
            .children
            .first()
            .unwrap()
            .to_owned()),
        scene_id
    );
}

/// Move an object from the middle of a folder to another folder and back
///
/// Starting layout:
///
/// ```
/// text
/// ├── 000-folder1
/// │   ├── 000-a.md
/// │   ├── 001-b.md
/// │   ├── 002-c.md
/// │   └── metadata.toml
/// ├── 001-folder2
/// │   └── metadata.toml
/// └── metadata.toml
/// ```
///
/// `b` moves to folder 2, leaving `000-a.md` and `001-c.md`,
/// then `b` moves to index 0 of folder 1, leaving `000-b.md`, `001-a.md` and `002-c.md`
///
/// Currently the most comprehensive test (at checking it's assumptions in multiple places),
/// probably because it was written last
#[test]
fn test_move_between_folder_contents() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut folder2 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder2.get_base_mut().metadata.name = "folder2".to_string();
    folder2.get_base_mut().file.modified = true;

    let mut scene_a = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene_a.get_base_mut().metadata.name = "a".to_string();
    scene_a.get_base_mut().file.modified = true;

    let mut scene_b = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene_b.get_base_mut().metadata.name = "b".to_string();
    scene_b.get_base_mut().file.modified = true;

    let mut scene_c = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene_c.get_base_mut().metadata.name = "c".to_string();
    scene_c.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let folder2_id = folder2.get_base().metadata.id.clone();
    let scene_a_id = scene_a.get_base().metadata.id.clone();
    let scene_b_id = scene_b.get_base().metadata.id.clone();
    let scene_c_id = scene_c.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(folder2);
    project.add_object(scene_a);
    project.add_object(scene_b);
    project.add_object(scene_c);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/000-a.md").exists());
    assert!(project_path.join("text/000-folder1/001-b.md").exists());
    assert!(project_path.join("text/000-folder1/002-c.md").exists());

    // Move b into folder 2
    move_child(
        &scene_b_id,
        &folder1_id,
        &folder2_id,
        0,
        &mut project.objects,
    )
    .unwrap();

    // Ensure the file got moved
    assert!(project_path.join("text/001-folder2/000-b.md").exists());
    assert!(!project_path.join("text/000-folder1/001-b.md").exists());

    // Ensure indexing is correct in the old folder
    assert!(project_path.join("text/000-folder1/000-a.md").exists());
    assert!(project_path.join("text/000-folder1/001-c.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        2
    );

    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        1
    );

    // children are in the correct spots in the folder
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene_a_id
    );

    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .get(1)
            .unwrap(),
        &scene_c_id
    );

    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene_b_id
    );

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project.objects.get(&scene_a_id).unwrap().get_base().index,
        Some(0)
    );
    assert_eq!(
        project.objects.get(&scene_c_id).unwrap().get_base().index,
        Some(1)
    );
    assert_eq!(
        project.objects.get(&scene_b_id).unwrap().get_base().index,
        Some(0)
    );

    assert!(
        project
            .objects
            .get(&scene_b_id)
            .unwrap()
            .get_path()
            .ends_with("text/001-folder2/000-b.md")
    );

    assert!(
        project
            .objects
            .get(&scene_a_id)
            .unwrap()
            .get_path()
            .ends_with("text/000-folder1/000-a.md")
    );

    assert!(
        project
            .objects
            .get(&scene_c_id)
            .unwrap()
            .get_path()
            .ends_with("text/000-folder1/001-c.md")
    );

    // Now, move b back into the start of folder 1
    move_child(
        &scene_b_id,
        &folder2_id,
        &folder1_id,
        0,
        &mut project.objects,
    )
    .unwrap();

    // Ensure indexing is correct in the new folder
    assert!(!project_path.join("text/001-folder2/000-b.md").exists());
    assert!(project_path.join("text/000-folder1/000-b.md").exists());
    assert!(project_path.join("text/000-folder1/001-a.md").exists());
    assert!(project_path.join("text/000-folder1/002-c.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        3
    );

    assert_eq!(
        project
            .objects
            .get(&folder2_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    // children are in the correct spots in the folder
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene_b_id
    );

    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .get(1)
            .unwrap(),
        &scene_a_id
    );

    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .get(2)
            .unwrap(),
        &scene_c_id
    );

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project.objects.get(&scene_b_id).unwrap().get_base().index,
        Some(0)
    );
    assert_eq!(
        project.objects.get(&scene_a_id).unwrap().get_base().index,
        Some(1)
    );
    assert_eq!(
        project.objects.get(&scene_c_id).unwrap().get_base().index,
        Some(2)
    );

    assert!(
        project
            .objects
            .get(&scene_b_id)
            .unwrap()
            .get_path()
            .ends_with("text/000-folder1/000-b.md")
    );

    assert!(
        project
            .objects
            .get(&scene_a_id)
            .unwrap()
            .get_path()
            .ends_with("text/000-folder1/001-a.md")
    );

    assert!(
        project
            .objects
            .get(&scene_c_id)
            .unwrap()
            .get_path()
            .ends_with("text/000-folder1/002-c.md")
    );
}

/// Move an object to its parent
#[test]
fn test_move_to_parent() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let text_id = project.text_id.clone();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut scene = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene.get_base_mut().metadata.name = "scene1".to_string();
    scene.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let scene_id = scene.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(scene);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(!project_path.join("text/001-scene1.md").exists());

    // Do the move (folder2 (which contains scene) into folder1)
    move_child(&scene_id, &folder1_id, &text_id, 1, &mut project.objects).unwrap();

    // Verify that the move happened on disk:
    assert!(project_path.join("text/000-folder1/metadata.toml").exists());
    assert!(!project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(project_path.join("text/001-scene1.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    // Text contains Folder 1
    assert_eq!(
        project
            .objects
            .get(&text_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &folder1_id
    );

    // Text has 2 children
    assert_eq!(
        project
            .objects
            .get(&text_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        2
    );

    // Text contains scene's ID
    assert_eq!(
        project
            .objects
            .get(&text_id)
            .unwrap()
            .get_base()
            .children
            .get(1)
            .unwrap(),
        &scene_id
    );
}

/// Move an object to its parent, where it currently is
#[test]
fn test_move_to_parent_current_position() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let text_id = project.text_id.clone();

    let mut folder1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder1.get_base_mut().metadata.name = "folder1".to_string();
    folder1.get_base_mut().file.modified = true;

    let mut scene = folder1.create_child_at_end(FileType::Scene).unwrap();
    scene.get_base_mut().metadata.name = "scene1".to_string();
    scene.get_base_mut().file.modified = true;

    let folder1_id = folder1.get_base().metadata.id.clone();
    let scene_id = scene.get_base().metadata.id.clone();

    project.add_object(folder1);
    project.add_object(scene);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/000-scene1.md").exists());
    assert!(!project_path.join("text/001-scene1.md").exists());

    // Do the move (folder2 (which contains scene) into folder1)
    move_child(&scene_id, &folder1_id, &text_id, 0, &mut project.objects).unwrap();

    // Verify that the move happened on disk:
    assert!(project_path.join("text/000-scene1.md").exists());
    assert!(project_path.join("text/001-folder1/metadata.toml").exists());
    assert!(!project_path.join("text/001-folder1/000-scene1.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&folder1_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        0
    );

    // Text children contains scene
    assert_eq!(
        project
            .objects
            .get(&text_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene_id
    );

    // Text contains Folder 1
    assert_eq!(
        project
            .objects
            .get(&text_id)
            .unwrap()
            .get_base()
            .children
            .get(1)
            .unwrap(),
        &folder1_id
    );

    // Text has 2 children
    assert_eq!(
        project
            .objects
            .get(&text_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        2
    );
}

/// Move something where it already is (should be no-op)
#[test]
fn test_move_to_self() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let text_id = project.text_id.clone();

    let mut folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    folder.get_base_mut().metadata.name = "folder1".to_string();
    folder.get_base_mut().file.modified = true;

    let mut scene = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    scene.get_base_mut().metadata.name = "scene1".to_string();
    scene.get_base_mut().file.modified = true;

    let folder_id = folder.get_base().metadata.id.clone();
    let scene_id = scene.get_base().metadata.id.clone();

    project.add_object(folder);
    project.add_object(scene);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-folder1/").exists());
    assert!(project_path.join("text/001-scene1.md").exists());

    let scene_original_modtime =
        run_with_file_object(&scene_id, &mut project.objects, |scene, _| {
            scene.get_base().file.modtime.unwrap()
        });

    let folder_original_modtime =
        run_with_file_object(&scene_id, &mut project.objects, |scene, _| {
            scene.get_base().file.modtime.unwrap()
        });

    // Do the move
    move_child(&folder_id, &text_id, &text_id, 0, &mut project.objects).unwrap();

    // Verify that nothing happened on disk:
    assert!(project_path.join("text/000-folder1/").exists());
    assert!(project_path.join("text/001-scene1.md").exists());
    assert!(!project_path.join("text/000-scene1.md").exists());
    assert!(!project_path.join("text/001-folder1/").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project.objects.get(&scene_id).unwrap().get_base().index,
        Some(1)
    );
    assert_eq!(
        project.objects.get(&folder_id).unwrap().get_base().index,
        Some(0)
    );

    // Check that the values are properly ordered within the children
    assert_eq!(
        project.run_with_folder(ProjectFolder::text, |text, _| text
            .get_base()
            .children
            .first()
            .unwrap()
            .to_owned()),
        folder_id
    );

    assert_eq!(
        project.run_with_folder(ProjectFolder::text, |text, _| text
            .get_base()
            .children
            .get(1)
            .unwrap()
            .to_owned()),
        scene_id
    );

    let scene_new_modtime = run_with_file_object(&scene_id, &mut project.objects, |scene, _| {
        scene.get_base().file.modtime.unwrap()
    });

    let folder_new_modtime = run_with_file_object(&scene_id, &mut project.objects, |scene, _| {
        scene.get_base().file.modtime.unwrap()
    });

    assert_eq!(scene_original_modtime, scene_new_modtime);
    assert_eq!(folder_original_modtime, folder_new_modtime);
}

/// Try to move a folder into one of it's (distant) children, verify that it does not allow it
#[test]
fn test_move_to_child() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut top_level_folder = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Folder)
        })
        .unwrap();
    let top_level_folder_id = top_level_folder.get_base().metadata.id.clone();
    top_level_folder.get_base_mut().metadata.name = String::from("top");
    top_level_folder.get_base_mut().file.modified = true;

    let mut mid_level_folder = top_level_folder
        .create_child_at_end(FileType::Folder)
        .unwrap();
    let mid_level_folder_id = mid_level_folder.get_base().metadata.id.clone();
    mid_level_folder.get_base_mut().metadata.name = String::from("mid");
    mid_level_folder.get_base_mut().file.modified = true;

    let mut child_folder = mid_level_folder
        .create_child_at_end(FileType::Scene)
        .unwrap();
    let child_folder_id = child_folder.get_base().metadata.id.clone();
    child_folder.get_base_mut().metadata.name = String::from("child");
    child_folder.get_base_mut().file.modified = true;

    assert!(child_folder.get_file().exists());
    assert_eq!(child_folder.get_base().index, Some(0));

    project.add_object(top_level_folder);
    project.add_object(mid_level_folder);
    project.add_object(child_folder);

    project.save().unwrap();

    // Try to move into a folder it directly contains:
    let immediate_move = move_child(
        &top_level_folder_id,
        &project.text_id,
        &mid_level_folder_id,
        1,
        &mut project.objects,
    );

    assert_eq!(
        immediate_move.err().unwrap().to_string(),
        format!("attempted to move {} into itself", &top_level_folder_id)
    );

    // Try to move into a folder contained within a child:
    let child_move = move_child(
        &top_level_folder_id,
        &project.text_id,
        &child_folder_id,
        1,
        &mut project.objects,
    );

    assert_eq!(
        child_move.err().unwrap().to_string(),
        format!("attempted to move {} into itself", &top_level_folder_id)
    );

    // Make sure nothing moved on disk:
    assert_eq!(
        project
            .run_with_folder(ProjectFolder::text, |text, _| text
                .get_base()
                .children
                .first()
                .unwrap()
                .to_owned())
            .as_str(),
        top_level_folder_id.as_str()
    );

    assert_eq!(
        run_with_file_object(&top_level_folder_id, &mut project.objects, |folder, _| {
            folder.get_base().children.first().unwrap().to_owned()
        })
        .as_str(),
        mid_level_folder_id.as_str()
    );

    assert_eq!(
        run_with_file_object(&mid_level_folder_id, &mut project.objects, |folder, _| {
            folder.get_base().children.first().unwrap().to_owned()
        })
        .as_str(),
        child_folder_id.as_str()
    );

    assert!(run_with_file_object(
        &child_folder_id,
        &mut project.objects,
        |folder, _| folder.get_path().ends_with("000-top/000-mid/000-child.md")
    ));

    assert!(run_with_file_object(
        &child_folder_id,
        &mut project.objects,
        |folder, _| folder.get_path().exists()
    ));
}

#[test]
fn test_move_no_clobber() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut project =
        Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    let mut scene1 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    scene1.get_base_mut().metadata.name = "a".to_string();
    scene1.get_base_mut().file.modified = true;

    let mut scene2 = project
        .run_with_folder(ProjectFolder::text, |text, _| {
            text.create_child_at_end(FileType::Scene)
        })
        .unwrap();
    scene2.get_base_mut().metadata.name = "a".to_string();
    scene2.get_base_mut().file.modified = true;

    let scene1_id = scene1.get_base().metadata.id.clone();
    let scene2_id = scene2.get_base().metadata.id.clone();

    project.add_object(scene1);
    project.add_object(scene2);
    project.save().unwrap();

    let project_path = project.get_path();

    // Check before the move
    assert!(project_path.join("text/000-a.md").exists());
    assert!(project_path.join("text/001-a.md").exists());

    // Move b into folder 2
    move_child(
        &scene1_id,
        &project.text_id,
        &project.text_id,
        1,
        &mut project.objects,
    )
    .unwrap();

    assert!(project_path.join("text/000-a.md").exists());
    assert!(project_path.join("text/001-a.md").exists());

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project
            .objects
            .get(&project.text_id)
            .unwrap()
            .get_base()
            .children
            .len(),
        2
    );

    assert_eq!(
        project
            .objects
            .get(&project.text_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene2_id
    );

    assert_eq!(
        project
            .objects
            .get(&project.text_id)
            .unwrap()
            .get_base()
            .children
            .first()
            .unwrap(),
        &scene2_id
    );

    // Make sure the file objects moved the children appropriately
    assert_eq!(
        project.objects.get(&scene1_id).unwrap().get_base().index,
        Some(1)
    );
    assert_eq!(
        project.objects.get(&scene2_id).unwrap().get_base().index,
        Some(0)
    );
}

/// Make sure places can nest
#[test]
fn test_place_nesting() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let mut text =
        Folder::new_top_level(base_dir.path().to_path_buf(), "text".to_string()).unwrap();

    let mut place1 = text.create_child_at_end(FileType::Place).unwrap();

    let place2 = place1.create_child_at_end(FileType::Place).unwrap();

    assert!(place2.get_file().exists());
    assert_eq!(place1.get_base().index, Some(0));
    assert_eq!(place2.get_base().index, Some(0));
}

#[test]
fn test_place_loading() {
    let base_dir = tempfile::TempDir::new().unwrap();

    let place_file_text = r#"id = "1"
file_type = "place""#;
    let worldbuilding_file_text = r#"id = "2"
file_type = "worldbuilding""#;

    // open and immediately drop the project (just creating the files)
    Project::new(base_dir.path().to_path_buf(), "test project".to_string()).unwrap();

    std::fs::create_dir(Path::join(
        base_dir.path(),
        "test_project/worldbuilding/000-place1/",
    ))
    .unwrap();

    write_with_temp_file(
        &Path::join(
            base_dir.path(),
            "test_project/worldbuilding/000-place1/metadata.toml",
        ),
        place_file_text.as_bytes(),
    )
    .unwrap();

    std::fs::create_dir(Path::join(
        base_dir.path(),
        "test_project/worldbuilding/001-place2/",
    ))
    .unwrap();

    write_with_temp_file(
        &Path::join(
            base_dir.path(),
            "test_project/worldbuilding/001-place2/metadata.toml",
        ),
        worldbuilding_file_text.as_bytes(),
    )
    .unwrap();

    let mut project = Project::load(base_dir.path().join("test_project")).unwrap();
    project.save().unwrap();

    let place_path = project.objects.get("1").unwrap().get_path();
    match from_file(&place_path, Some(0)).unwrap() {
        FileObjectCreation::Place(place, _) => {
            assert!(
                read_to_string(place.get_file())
                    .unwrap()
                    .contains(r#"notes = """#)
            );
        }
        _ => panic!(),
    }

    let worldbuilding_path = project.objects.get("2").unwrap().get_path();
    match from_file(&worldbuilding_path, Some(1)).unwrap() {
        FileObjectCreation::Place(place, _) => {
            assert!(
                read_to_string(place.get_file())
                    .unwrap()
                    .contains(r#"notes = """#)
            );
        }
        _ => panic!(),
    }
}
