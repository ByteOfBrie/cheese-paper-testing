#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use clap::{Parser, Subcommand};

mod components;
mod tiny_markdown;
mod ui;

use crate::ui::CheesePaperApp;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    cli: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Command line interface to inspect editor files
    Cli {},
}

fn main() {
    let args = Args::parse();

    match &args.cli {
        Some(cli) => {
            println!("Using CLI interface")
        }
        None => {
            env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
            let options = eframe::NativeOptions::default();

            eframe::run_native(
                "Cheese Paper Rust",
                options,
                Box::new(|cc| Ok(Box::new(CheesePaperApp::new(cc)))),
            )
            .unwrap()
        }
    }
}
