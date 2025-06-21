// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

mod components;
mod tiny_markdown;
mod ui;

use crate::components::file_objects::FileObject;
use crate::components::file_objects::FileType;
use crate::ui::CheesePaperApp;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// File to show information about
    #[arg(long)]
    show: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if let Some(show_path) = args.show.as_deref() {
        println!("Using CLI interface");
        println!("{show_path:?}");
        let basename = show_path.file_name().unwrap();
        let dirname = show_path.parent().unwrap();

        let mut file = FileObject::new(
            FileType::Scene,
            dirname.to_owned(),
            basename.to_owned(),
            0,
            None,
        );

        file.load_file()?;

        println!("{file:#?}");
    } else {
        env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
        let options = eframe::NativeOptions::default();

        eframe::run_native(
            "Cheese Paper Rust",
            options,
            Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc)))),
        )
        .unwrap()
    }
    Ok(())
}
