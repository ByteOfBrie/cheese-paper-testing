// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

mod components;
mod tiny_markdown;
mod ui;

use crate::components::file_objects::from_file;

use crate::ui::CheesePaperApp;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// File to show information about
    #[arg(long)]
    show_cli: Option<PathBuf>,

    #[arg(long)]
    show_ui: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let args = Args::parse();

    if let Some(show_path) = args.show_cli.as_deref() {
        println!("Using CLI interface");
        println!("{show_path:?}");

        let file_object_creation = from_file(show_path, 0);

        println!("file(s): {:#?}", file_object_creation.unwrap().object);
    } else if let Some(show_path) = args.show_ui.as_deref() {
        let mut files = from_file(show_path, 0).unwrap();

        let file = &mut files.object;

        eframe::run_native(
            "Cheese Paper Rust Single File",
            Default::default(),
            Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc, file)))),
        )
        .unwrap()
    }
    Ok(())
}
