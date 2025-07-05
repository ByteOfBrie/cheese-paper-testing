#[cfg(test)]
use crate::components::file_objects::{
    FileInfo, FileObject, FileObjectMetadata, FileObjectStore, Folder, from_file,
};
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
    assert_eq!(std::fs::read_dir(base_dir.path())?.count(), 0);

    let mut project = Project::new(base_dir.path().to_path_buf(), project_name.to_string())?;
    project.save()?;

    println!(
        "{:?}",
        read_dir(base_dir.path()).unwrap().collect::<Vec<_>>()
    );

    assert_eq!(read_dir(base_dir.path())?.count(), 1);
    assert!(project_path.exists());
    assert_eq!(read_dir(&project_path)?.count(), 4);

    let project_toml_contents = read_to_string(project_path.join("project.toml"))?;

    // Ensure that the file is populated at least
    assert!(project_toml_contents.len() != 0);

    Ok(())
}
