mod app_gui;
mod config_manager;
mod descriptions;
mod file_manager;
mod logging;
mod settings;
mod wav_file_index;

use log::error;
use std::cell::RefCell;
use std::fs;
use std::process::Command;
use std::rc::Rc;
use walkdir::WalkDir;

use crate::descriptions::Descriptions;
use crate::settings::AppSettings;
use app_gui::AppGUI;
use config_manager::ConfigManager;
use eframe::{egui::ViewportBuilder, icon_data::from_png_bytes};
use file_manager::FileManager;

fn main() {
    // Create shared log buffer
    let log_buffer = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let buffer_for_logging = std::sync::Arc::clone(&log_buffer);

    // Initialize log4rs with console and memory appenders
    if let Err(e) = logging::init_logging(buffer_for_logging) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    migrate_app_entry();

    let mut temp_settings = AppSettings::default();
    temp_settings.dev_mode = cfg!(debug_assertions);

    // Load application settings using the temp settings to determine path
    let loaded_settings = match temp_settings.load() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to load settings: {}, using defaults", e);
            temp_settings
        }
    };

    let settings = Rc::new(RefCell::new(loaded_settings));

    // Descriptions, loads HRTF descriptions from embedded CSV
    let descriptions = match Descriptions::new() {
        Ok(v) => v,
        Err(e) => {
            let err = format!("Can not load HRTF descriptions. Reason: {e}");
            show_warning(&err);
            std::process::exit(1);
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
            std::process::exit(1);
        }
    };

    // Load icon from embedded PNG bytes (same as used in goose.rs)
    let icon_bytes = include_bytes!("../data/IrateGoose256.png");
    let icon = match from_png_bytes(icon_bytes) {
        Ok(icon) => icon,
        Err(e) => {
            log::warn!("Failed to load window icon: {}, using default", e);
            eframe::egui::IconData::default()
        }
    };

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_app_id("irate_goose")
            .with_title("Irate Goose - Surround Sound Configurator")
            .with_icon(icon),
        ..eframe::NativeOptions::default()
    };

    let _ = eframe::run_native(
        "Irate Goose - Surround Sound Configurator",
        native_options,
        Box::new(|cc| {
            // Theme will be set by AppGUI constructor
            Ok(Box::new(AppGUI::new(
                cc,
                settings.clone(),
                &mut file_manager,
                &config_manager,
                log_buffer,
            )))
        }),
    );
}

/// Searches for an old desktop entry file installed by a previous version of
/// IrateGoose and removes it. This migrates away from the old CLI-based menu
/// integration. Only searches the per-user XDG applications directory tree.
fn migrate_app_entry() {
    let Some(base) = dirs::data_dir().map(|p| p.join("applications")) else {
        log::warn!("Could not determine data directory, skipping migration");
        return;
    };
    if !base.exists() {
        return;
    }

    let found = WalkDir::new(&base)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name() == "barafu-irategoose.desktop");

    let Some(entry) = found else {
        return;
    };

    let path = entry.path();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {}: {}", path.display(), e);
            return;
        }
    };

    if content.contains("Name=IrateGoose") {
        if let Err(e) = fs::remove_file(path) {
            log::warn!("Failed to remove {}: {}", path.display(), e);
        } else {
            log::info!("Removed old desktop entry at {}", path.display());
        }
    } else {
        log::info!(
            "Found {} but content did not match expected pattern, skipping",
            path.display()
        );
    }
}

/// Tries to show message on CLI and GUI too.
fn show_warning(msg: &str) {
    error!("{msg}");

    // Try to send desktop notification using notify-send
    let _ = Command::new("notify-send")
        .args([
            "--urgency=critical",
            "--app-name=Irate Goose",
            "--icon=audio-volume-muted",
            msg,
        ])
        .spawn();
}
