mod app_gui;
mod config_manager;
mod descriptions;
mod file_manager;
mod icon_loader;
mod settings;

use clap::Parser;
use eframe::egui::{Style, Visuals};
use log::error;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::{path::PathBuf, process};

use crate::descriptions::Descriptions;
use crate::settings::AppSettings;
use app_gui::AppGUI;
use config_manager::ConfigManager;
use file_manager::FileManager;

/// Command line arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Path to directory containing WAV files
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Dry-run mode (hidden),makes app to write Pipewire config in /tmp instead of proper placement.
    #[arg(long, hide = true)]
    dry_run: bool,
}

fn main() {
    // Initialise logger
    env_logger::init();

    // Parse CLI arguments
    let args = CliArgs::parse();

    // Create a temporary settings instance with the determined dev mode
    let mut temp_settings = AppSettings::default();
    temp_settings.dev_mode = args.dry_run;

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
        settings.borrow_mut().active_wav_directory = Some(p.clone());
    }

    // File manager, scans for WAV files.
    let mut file_manager;
    match FileManager::new(settings.clone()) {
        Ok(v) => file_manager = v,
        Err(e) => {
            let err = format!("Can not find wave files. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    }

    // Config manager, writes and deletes the PipeWire config
    let config_manager = match ConfigManager::new(settings.clone()) {
        Ok(v) => v,
        Err(e) => {
            let err = format!("Can not process config file. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    };

    // Descriptions, loads HRTF descriptions from embedded CSV

    let descriptions = match Descriptions::new() {
        Ok(v) => v,
        Err(e) => {
            let err = format!("Can not load HRTF descriptions. Reason: {e}");
            show_warning(&err);
            process::exit(1);
        }
    };

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

    // Load application icon
    let icon_data = icon_loader::load_icon();

    // Configure eframe options with icon using ViewportBuilder
    // This only works on X11, not on Wayland
    let mut eframe_options = eframe::NativeOptions::default();
    eframe_options.viewport.icon = Some(Arc::new(icon_data));

    let _ = eframe::run_native(
        "IrateGoose - Surround Sound Configurator",
        eframe_options,
        Box::new(|cc| {
            cc.egui_ctx.set_style(style);
            Ok(Box::new(AppGUI::new(
                cc,
                settings.clone(),
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
