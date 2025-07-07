// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

mod components;
mod tiny_markdown;
mod ui;

use crate::components::Project;
use crate::components::file_objects::base::FileObjectCreation;
use crate::components::file_objects::{FileObject, from_file};

use crate::ui::CheesePaperApp;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// File to show information about
    #[arg(long)]
    show_cli: Option<PathBuf>,

    #[arg(long)]
    show: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let args = Args::parse();

    if let Some(show_path) = args.show_cli.as_deref() {
        println!("Using CLI interface");
        println!("{show_path:?}");

        let file_object_creation = from_file(show_path, Some(0));

        println!("file(s): {:#?}", file_object_creation);
    } else if let Some(project_path) = args.show.as_deref() {
        let project = Project::load(project_path.to_owned())?;

        eframe::run_native(
            "Cheese Paper Rust Single File",
            Default::default(),
            Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc, project)))),
        )
        .unwrap()
    }
    Ok(())
}
