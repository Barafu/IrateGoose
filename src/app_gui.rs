use eframe::egui;
use std::path::PathBuf;

use crate::file_manager::FileManager;
use crate::config_manager::ConfigManager;

pub struct AppGUI<'a> {
    file_manager: &'a mut FileManager,
    config_manager: &'a ConfigManager,
    selected_index: Option<usize>,
    relative_paths: Vec<PathBuf>,
    config_installed: bool,
    status_message: String,
}

impl<'a> AppGUI<'a> {
    pub fn new(_cc: &eframe::CreationContext<'_>, file_manager: &'a mut FileManager, config_manager: &'a ConfigManager) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        
        let config_installed = config_manager.config_exists();
        let relative_paths = file_manager.relative_paths();
        
        Self {
            file_manager,
            config_manager,
            selected_index: None,
            relative_paths,
            config_installed,
            status_message: String::new(),
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
                    if let Some(index) = self.selected_index {
                        let absolute_path = self.file_manager.absolute_path(index);
                        match self.config_manager.write_config(absolute_path) {
                            Ok(()) => {
                                self.status_message = format!("Config written using {}", absolute_path.display());
                                self.config_installed = true;
                            }
                            Err(e) => {
                                self.status_message = format!("Failed to write config: {}", e);
                            }
                        }
                    } else {
                        self.status_message = "No file selected".to_string();
                    }
                }
                
                // Delete config button should be disabled if config is not installed
                let delete_button = ui.add_enabled(self.config_installed, egui::Button::new("Delete config"));
                if delete_button.clicked() {
                    match self.config_manager.delete_config() {
                        Ok(()) => {
                            self.status_message = "Config deleted".to_string();
                            self.config_installed = false;
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to delete config: {}", e);
                        }
                    }
                }
            });
            
            // Determine if a file is selected
            let is_file_selected = self.selected_index.is_some();
            
            // Create the Apply button, disabled if no file is selected
            let apply_button = ui.add_enabled(is_file_selected, egui::Button::new("Apply"));
            
            if apply_button.clicked() {
                if let Some(index) = self.selected_index {
                    let absolute_path = self.file_manager.absolute_path(index);
                    match self.config_manager.write_config(absolute_path) {
                        Ok(()) => {
                            self.status_message = format!("Config applied using {}", absolute_path.display());
                            self.config_installed = true;
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to apply config: {}", e);
                        }
                    }
                }
            }
            
            ui.separator();
            ui.heading("Registered Wave Files");
            
            // Create a scrollable area for the file list
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    // Display each wave file as a selectable item
                    for (index, rel_path) in self.relative_paths.iter().enumerate() {
                        let is_selected = self.selected_index == Some(index);
                        if ui.selectable_label(is_selected, rel_path.to_string_lossy()).clicked() {
                            self.selected_index = Some(index);
                        }
                    }
                    
                    // If no files are found, show a message
                    if self.relative_paths.is_empty() {
                        ui.label("No .wav files found in the directory.");
                    }
                });
        });
    }
}
