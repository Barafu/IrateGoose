mod app_gui;
mod file_manager;
mod config_manager;

use std::{env, path::PathBuf, process};
use std::process::Command;
use log::error;

use app_gui::AppGUI;
use file_manager::FileManager;
use config_manager::ConfigManager;

// UI texts
const NO_WAVEFILE_PATH: &str = "Could not determine path to wave files";

fn main() {

    // Initialise logger
    env_logger::init();

    // CLI

    //Determine path to search for wave files
    let working_dir = env::current_dir().ok();
    let given_wavefiles_path = env::args().nth(1).map(|s| PathBuf::from(s));
    let wavefiles_path;
    match given_wavefiles_path.or(working_dir) {
        Some(v) => wavefiles_path = v,
        None => {
            show_warning(NO_WAVEFILE_PATH);
            panic!("{NO_WAVEFILE_PATH}")
        }
    }

    // File manager, scans for WAV files. 
    let mut file_manager;
    match FileManager::new(wavefiles_path.clone()) {
        Ok(v) => file_manager = v,
        Err(e) => {
            let err = format!("Can not find wave files in {}. Reason: {}", wavefiles_path.to_str().unwrap_or_default(),e);
            show_warning(&err);
            process::exit(1);
        },
    }

    // Config manager, writes and deletes the PipeWire config
    let config_manager;
    match ConfigManager::new() {
        Ok(v) => config_manager = v,
        Err(e) => {
            let err = format!("Can not process config file. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        },
    }

    let eframe_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Surroud Sound Configurator",
        eframe_options,
        Box::new(|cc| Ok(Box::new(AppGUI::new(cc, &mut file_manager, &config_manager)))),
    );
}

/// Tries to show message on CLI and GUI too.
fn show_warning(msg: &str) {
    error!("{msg}");
    
    // Try to send desktop notification using notify-send
    let _ = Command::new("notify-send")
        .args(["--urgency=critical", "--app-name=GUI for surround sound", "--icon=audio-volume-muted", msg])
        .spawn();
}
