// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

mod components;
mod tiny_markdown;
mod ui;

use crate::components::file_objects::FileObject;
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

        let file = FileObject::from_file(show_path, 0, None);

        println!("file(s): {file:#?}");
    } else if let Some(show_path) = args.show_ui.as_deref() {
        let mut files = FileObject::from_file(show_path, 0, None).unwrap();

        let mut file = files.pop().unwrap();

        let mut file_text = match file.underlying_obj.get_body() {
            Some(val) => val,
            None => {
                println!("No underlying data to view");
                return Ok(());
            }
        };

        use crate::ui::BaseTextEditor;

        eframe::run_native(
            "Cheese Paper Rust Single File",
            Default::default(),
            Box::new(|_cc| {
                Ok(Box::new(CheesePaperApp {
                    editor: BaseTextEditor::new(file_text),
                }))
            }),
        )
        .unwrap()
    } else {
        let options = eframe::NativeOptions::default();

        let mut default_text = crate::ui::DEFAULT_TEXT.trim().to_owned();

        eframe::run_native(
            "Cheese Paper Rust",
            options,
            Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc, &mut default_text)))),
        )
        .unwrap()
    }
    Ok(())
}
