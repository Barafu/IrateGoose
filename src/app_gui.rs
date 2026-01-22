use eframe::egui;
use std::path::PathBuf;

use crate::file_manager::{FileManager, WaveSampleRate};
use crate::config_manager::ConfigManager;
use log::{error, info};

pub struct AppGUI<'a> {
    file_manager: &'a mut FileManager,
    config_manager: &'a ConfigManager,
    selected_index: Option<usize>,
    relative_paths: Vec<PathBuf>,
    sample_rate_filter: WaveSampleRate,
    config_installed: bool,
    status_message: String,
}

enum MessageLevel {
    Normal,
    Error,
}

impl<'a> AppGUI<'a> {
    pub fn new(_cc: &eframe::CreationContext<'_>, file_manager: &'a mut FileManager, config_manager: &'a ConfigManager) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        
        let config_installed = config_manager.config_exists();
        let relative_paths = file_manager.relative_paths();
        let sample_rate_filter = WaveSampleRate::F48000;
        
        Self {
            file_manager,
            config_manager,
            selected_index: None,
            relative_paths,
            sample_rate_filter,
            config_installed,
            status_message: String::new(),
        }
    }

    fn on_write_config_click(&mut self) {
        if let Some(index) = self.selected_index {
            let absolute_path = self.file_manager.absolute_path(index);
            match self.config_manager.write_config(absolute_path) {
                Ok(()) => {
                    self.message(MessageLevel::Normal, &format!("Config written using {}", absolute_path.display()));
                    self.config_installed = true;
                }
                Err(e) => {
                    self.message(MessageLevel::Error, &format!("Failed to write config: {}", e));
                }
            }
        } else {
            self.message(MessageLevel::Error, "No file selected");
        }
    }

    fn on_delete_config_click(&mut self) {
        match self.config_manager.delete_config() {
            Ok(()) => {
                self.message(MessageLevel::Normal, "Config deleted");
                self.config_installed = false;
            }
            Err(e) => {
                self.message(MessageLevel::Error, &format!("Failed to delete config: {}", e));
            }
        }
    }

    fn on_apply_click(&mut self) {
        if let Some(index) = self.selected_index {
            let absolute_path = self.file_manager.absolute_path(index);
            match self.config_manager.write_config(absolute_path) {
                Ok(()) => {
                    self.message(MessageLevel::Normal, &format!("Config applied using {}", absolute_path.display()));
                    self.config_installed = true;
                }
                Err(e) => {
                    self.message(MessageLevel::Error, &format!("Failed to apply config: {}", e));
                }
            }
        }
    }

    /// Displays message to status bar and log. 
    fn message(&mut self, message_level: MessageLevel, message: &str) {
        match message_level {
            MessageLevel::Normal => {
                info!("{}", message);
                self.status_message = message.to_string();
            }
            MessageLevel::Error => {
                error!("{}", message);
                self.status_message = format!("ERROR! {}", message);
            }
        }
    }
}

impl<'a> eframe::App for AppGUI<'a> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
            // Add status bar at the bottom
            ui.label(&self.status_message);                
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Select Surround Sound");
            
            // Add Write Config and Delete Config buttons
            ui.horizontal(|ui| {
                let write_button = ui.button("Write config");
                if write_button.clicked() {
                    self.on_write_config_click();
                }
                
                // Delete config button should be disabled if config is not installed
                let delete_button = ui.add_enabled(self.config_installed, egui::Button::new("Delete config"));
                if delete_button.clicked() {
                    self.on_delete_config_click();
                }
            });
            
            // Determine if a file is selected
            let is_file_selected = self.selected_index.is_some();
            
            // Create the Apply button, disabled if no file is selected
            let apply_button = ui.add_enabled(is_file_selected, egui::Button::new("Apply"));
            
            if apply_button.clicked() {
                self.on_apply_click();
            }
            
            ui.separator();
            ui.heading("Registered Wave Files");

            // Radio buttons for sample rate filter
            ui.horizontal(|ui| {
                ui.label("Sample rate:");
                ui.radio_value(&mut self.sample_rate_filter, WaveSampleRate::F48000, "48000");
                ui.radio_value(&mut self.sample_rate_filter, WaveSampleRate::F44100, "44100");
                ui.radio_value(&mut self.sample_rate_filter, WaveSampleRate::Unknown, "All");
            });

            // Create a scrollable area for the file list
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    let mut any_displayed = false;
                    // Display each wave file as a selectable item, filtered by sample rate
                    for (index, rel_path) in self.relative_paths.iter().enumerate() {
                        let wave = &self.file_manager.wave_data[index];
                        let matches = match self.sample_rate_filter {
                            WaveSampleRate::F48000 => wave.sample_rate == WaveSampleRate::F48000,
                            WaveSampleRate::F44100 => wave.sample_rate == WaveSampleRate::F44100,
                            WaveSampleRate::Unknown => true,
                        };
                        if !matches {
                            continue;
                        }
                        any_displayed = true;
                        let is_selected = self.selected_index == Some(index);
                        if ui.selectable_label(is_selected, rel_path.to_string_lossy()).clicked() {
                            self.selected_index = Some(index);
                        }
                    }
                    
                    // If no files are found, show a message
                    if !any_displayed {
                        ui.label("No .wav files found in the directory.");
                    }
                });
        });
    }
}
