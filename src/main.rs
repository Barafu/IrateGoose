mod app_gui;
mod config_manager;
mod descriptions;
mod file_manager;
mod icon_loader;

use clap::Parser;
use eframe::egui::{Style, Visuals};
use log::error;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, path::PathBuf, process};

use crate::descriptions::Descriptions;
use app_gui::AppGUI;
use config_manager::ConfigManager;
use file_manager::FileManager;

// UI texts
const NO_WAVEFILE_PATH: &str = "Could not determine path to wave files";

/// Global development mode flag
/// True in debug builds, false in release builds
static DEV_MODE: AtomicBool = AtomicBool::new(cfg!(debug_assertions));

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

    // Override DEV_MODE if dry_run is true
    if args.dry_run {
        DEV_MODE.store(true, Ordering::Relaxed);
    }

    // Determine path to search for wave files
    let wavefiles_path = args
        .path
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| {
            show_warning(NO_WAVEFILE_PATH);
            panic!("{NO_WAVEFILE_PATH}")
        });

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

    let config_manager = match ConfigManager::new() {
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
