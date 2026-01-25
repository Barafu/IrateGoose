mod app_gui;
mod config_manager;
mod descriptions;
mod file_manager;

use eframe::egui::{Style, Visuals};
use log::error;
use std::process::Command;
use std::sync::LazyLock;
use std::{env, path::PathBuf, process};

use app_gui::AppGUI;
use config_manager::ConfigManager;
use crate::descriptions::Descriptions;
use file_manager::FileManager;

// UI texts
const NO_WAVEFILE_PATH: &str = "Could not determine path to wave files";

/// Global development mode flag
/// True in debug builds, false in release builds
static DEV_MODE: LazyLock<bool> = LazyLock::new(|| {
    cfg!(debug_assertions)
});

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
            let err = format!(
                "Can not find wave files in {}. Reason: {}",
                wavefiles_path.to_str().unwrap_or_default(),
                e
            );
            show_warning(&err);
            process::exit(1);
        }
    }

    // Config manager, writes and deletes the PipeWire config
    let config_manager;
    match ConfigManager::new() {
        Ok(v) => config_manager = v,
        Err(e) => {
            let err = format!("Can not process config file. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    }

    // Descriptions, loads HRTF descriptions from embedded CSV
    let descriptions;
    match Descriptions::new() {
        Ok(v) => descriptions = v,
        Err(e) => {
            let err = format!("Can not load HRTF descriptions. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    }

    // EGUI style - detect system theme
    let visuals = match Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "'prefer-light'" {
                Visuals::light()
            } else {
                Visuals::dark()
            }
        }
        _ => Visuals::dark(), // default to dark on any error
    };
    let style = Style {
        visuals,
        ..Style::default()
    };

    let eframe_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "IrateGoose - Surround Sound Configurator",
        eframe_options,
        Box::new(|cc| {
            cc.egui_ctx.set_style(style);
            Ok(Box::new(AppGUI::new(
                cc,
                &mut file_manager,
                &config_manager,
                &descriptions,
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
