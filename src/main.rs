mod app_gui;
mod config_manager;
mod descriptions;
mod file_manager;
mod goose;
mod settings;
mod wav_file_index;

use clap::{ArgGroup, Parser};
use log::error;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::{path::PathBuf, process};

use crate::descriptions::Descriptions;
use crate::settings::AppSettings;
use app_gui::AppGUI;
use config_manager::ConfigManager;
use file_manager::FileManager;

/// Command line arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(group = ArgGroup::new("install_uninstall").multiple(false).conflicts_with_all(["path"]))]
struct CliArgs {
    /// Path to directory containing WAV files
    #[arg(long)]
    path: Option<PathBuf>,

    /// Dev-mode (hidden),makes app to write Pipewire config in /tmp instead of proper placement.
    #[arg(long, hide = true)]
    dev_mode: bool,

    /// Install the application to the system menu
    #[arg(long, group = "install_uninstall")]
    install: bool,

    /// Remove the application from the system menu
    #[arg(long, group = "install_uninstall")]
    uninstall: bool,
}

fn main() {
    // Initialise logger
    env_logger::init();

    // Parse CLI arguments
    let args = CliArgs::parse();

    // Handle install/uninstall commands (exclusive with other arguments)
    if args.install {
        if let Err(e) = goose::install_goose() {
            log::error!("Installation failed: {}", e);
            process::exit(1);
        }
        process::exit(0);
    }
    if args.uninstall {
        if let Err(e) = goose::uninstall_goose() {
            log::error!("Uninstallation failed: {}", e);
            process::exit(1);
        }
        process::exit(0);
    }

    // Create a temporary settings instance with the determined dev mode
    let mut temp_settings = AppSettings::default();
    temp_settings.dev_mode = args.dev_mode;

    // Load application settings using the temp settings to determine path
    let loaded_settings = match temp_settings.load() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to load settings: {}, using defaults", e);
            temp_settings
        }
    };

    let settings = Rc::new(RefCell::new(loaded_settings));

    if let Some(p) = args.path {
        settings.borrow_mut().set_temp_wav_directory(p);
    }

    // Descriptions, loads HRTF descriptions from embedded CSV
    let descriptions = match Descriptions::new() {
        Ok(v) => v,
        Err(e) => {
            let err = format!("Can not load HRTF descriptions. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    };

    // File manager, scans for WAV files.
    let mut file_manager = FileManager::new(settings.clone(), descriptions);

    // Config manager, writes and deletes the PipeWire config
    let config_manager = match ConfigManager::new(settings.clone()) {
        Ok(v) => v,
        Err(e) => {
            let err = format!("Can not process config file. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    };

    let _ = eframe::run_native(
        "IrateGoose - Surround Sound Configurator",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            // Theme will be set by AppGUI constructor
            Ok(Box::new(AppGUI::new(
                cc,
                settings.clone(),
                &mut file_manager,
                &config_manager,
            )))
        }),
    );
}

/// Tries to show message on CLI and GUI too.
fn show_warning(msg: &str) {
    error!("{msg}");

    // Try to send desktop notification using notify-send
    let _ = Command::new("notify-send")
        .args([
            "--urgency=critical",
            "--app-name=IrateGoose",
            "--icon=audio-volume-muted",
            msg,
        ])
        .spawn();
}
