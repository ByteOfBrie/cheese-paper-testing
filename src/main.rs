// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

mod components;
mod tiny_markdown;
mod ui;

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
        let file: String = fs::read_to_string(show_path)?;
        println!("{file}");

        let metadata = fs::metadata(show_path)?;
        println!("{:?}", metadata.modified());
        match metadata
            .modified()?
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
        {
            Ok(n) => println!("{:?}", n),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
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
