use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::path::PathBuf;

use crate::config_manager::ConfigManager;
use crate::descriptions::Descriptions;
use crate::file_manager::{FileManager, WaveSampleRate};
use log::{error, info};

pub struct AppGUI<'a> {
    file_manager: &'a mut FileManager,
    config_manager: &'a ConfigManager,
    descriptions: &'a Descriptions,
    selected_index: Option<usize>,
    relative_paths: Vec<PathBuf>,
    sample_rate_filter: WaveSampleRate,
    config_installed: Option<PathBuf>,
    status_message: String,
    search_text: String,
}

enum MessageLevel {
    Normal,
    Error,
}

impl<'a> AppGUI<'a> {
    const METADATA_FRAME_HEIGHT: f32 = 120.0;

    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        file_manager: &'a mut FileManager,
        config_manager: &'a ConfigManager,
        descriptions: &'a Descriptions,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.

        let config_installed = Self::check_config_exists(config_manager);
        let relative_paths = file_manager.list_relative_paths();
        let sample_rate_filter = WaveSampleRate::F48000;

        Self {
            file_manager,
            config_manager,
            descriptions,
            selected_index: None,
            relative_paths,
            sample_rate_filter,
            config_installed,
            status_message: String::new(),
            search_text: String::new(),
        }
    }

    /// Checks if config exists and returns the path if found.
    /// Returns None if config doesn't exist or there's an error.
    fn check_config_exists(config_manager: &ConfigManager) -> Option<PathBuf> {
        match config_manager.config_exists() {
            Ok(Some(path)) => Some(path),
            Ok(None) => None,
            Err(e) => {
                error!("Error checking config: {}", e);
                None
            }
        }
    }

    fn on_write_config_click(&mut self) {
        if let Some(index) = self.selected_index {
            let absolute_path = self.file_manager.absolute_path(index).to_path_buf();
            let display_path = absolute_path.display().to_string();
            match self.config_manager.write_config(&absolute_path) {
                Ok(()) => {
                    // Double-check that config was written correctly and extract the path from config
                    match self.config_manager.config_exists() {
                        Ok(Some(config_path)) => {
                            self.message(
                                MessageLevel::Normal,
                                &format!("Config written using {}", display_path),
                            );
                            self.config_installed = Some(config_path);
                        }
                        Ok(None) => {
                            // Config file doesn't exist after writing - something went wrong
                            self.message(
                                MessageLevel::Error,
                                "Config written but not found afterwards",
                            );
                            self.config_installed = None;
                        }
                        Err(e) => {
                            // Error reading config after write
                            self.message(
                                MessageLevel::Error,
                                &format!("Config written but error verifying: {}", e),
                            );
                            self.config_installed = None;
                        }
                    }
                }
                Err(e) => {
                    self.message(
                        MessageLevel::Error,
                        &format!("Failed to write config: {}", e),
                    );
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
                self.config_installed = None;
            }
            Err(e) => {
                self.message(
                    MessageLevel::Error,
                    &format!("Failed to delete config: {}", e),
                );
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

    /// Get HRTF metadata for the currently selected file, if any.
    fn selected_metadata(&self) -> Option<&crate::descriptions::HRTFMetadata> {
        let index = self.selected_index?;
        let path = self.file_manager.absolute_path(index);
        let stem = path.file_stem()?.to_str()?;
        self.descriptions.get(stem)
    }

    /// Truncate a description to approximately three lines.
    fn truncate_description(description: &str) -> String {
        const MAX_LEN: usize = 240;
        if description.len() <= MAX_LEN {
            return description.to_string();
        }
        let truncated = &description[..MAX_LEN - 3];
        format!("{}...", truncated.trim_end())
    }

    /// Renders the file table with two columns: "Files" and "Description".
    fn render_file_table(&mut self, ui: &mut egui::Ui, filtered_items: Vec<usize>) {
        // Wrap the table in its own frame
        let table_frame = egui::Frame::group(ui.style());
        table_frame.show(ui, |ui| {
            // Create a two-column table using rows() for better performance
            let row_height = 20.0;
            let num_rows = filtered_items.len();
            let available_width = ui.available_width();
            let available_height: f32 = ui.available_height() - Self::METADATA_FRAME_HEIGHT;
            TableBuilder::new(ui)
                .column(Column::initial(available_width * 0.6)) // "Files" column - auto width
                .column(Column::remainder().clip(true)) // "Description" column - takes remaining width
                .max_scroll_height(available_height)
                .auto_shrink([false, false]) // Vertical auto_shrink false to always use available height
                .resizable(true)
                .striped(true)
                .sense(egui::Sense::click()) // Make rows clickable
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center)) // Center content vertically
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Files");
                    });
                    header.col(|ui| {
                        ui.heading("Description");
                    });
                })
                .body(|body| {
                    // Table rows are generated here
                    body.rows(row_height, num_rows, |mut row| {
                        let row_index = row.index();
                        let index = filtered_items[row_index];
                        let rel_path = &self.relative_paths[index];
                        let wave = &self.file_manager.wave_data[index];
                        let is_selected = self.selected_index == Some(index);
                        let mut label_text = rel_path.to_string_lossy().to_string();

                        // Get HRTF metadata for this file (cheap lookup)
                        let description_text = self
                            .file_manager
                            .absolute_path(index)
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .and_then(|stem| self.descriptions.get(stem))
                            .map(|metadata| {
                                // Take first line of description, fallback to empty string
                                metadata
                                    .description
                                    .lines()
                                    .next()
                                    .unwrap_or("")
                                    .trim()
                                    .to_string()
                            })
                            .unwrap_or_default();

                        // Set selection state for the row
                        row.set_selected(is_selected);

                        if wave.sample_rate == WaveSampleRate::Damaged {
                            label_text.insert_str(0, "(Damaged)");
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(label_text).color(egui::Color32::GRAY),
                                    )
                                    .truncate(),
                                );
                            });
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(description_text)
                                            .color(egui::Color32::GRAY),
                                    )
                                    .truncate(),
                                );
                            });
                        } else {
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(label_text)
                                        .truncate()
                                        .selectable(false),
                                );
                            });
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(description_text)
                                        .truncate()
                                        .selectable(false),
                                );
                            });
                        }

                        // Handle row click
                        if row.response().clicked() {
                            self.selected_index = Some(index);
                        }
                    });
                });
        });
    }

    /// Renders the file list table with two columns: "Files" and "Description".
    fn render_file_list_and_metadata(&mut self, ui: &mut egui::Ui) {
        ui.heading("Registered Wave Files");

        // Radio buttons for sample rate filter
        ui.horizontal(|ui| {
            ui.label("Sample rate:");
            let old_filter = self.sample_rate_filter;
            ui.radio_value(
                &mut self.sample_rate_filter,
                WaveSampleRate::F48000,
                "48000",
            );
            ui.radio_value(
                &mut self.sample_rate_filter,
                WaveSampleRate::F44100,
                "44100",
            );
            ui.radio_value(
                &mut self.sample_rate_filter,
                WaveSampleRate::F96000,
                "96000",
            );
            ui.radio_value(&mut self.sample_rate_filter, WaveSampleRate::Unknown, "All");

            // Check if filter changed
            if old_filter != self.sample_rate_filter {
                // If a file is selected, check if it's still visible with the new filter
                if let Some(index) = self.selected_index {
                    let wave = &self.file_manager.wave_data[index];
                    let matches = match self.sample_rate_filter {
                        WaveSampleRate::F48000 => wave.sample_rate == WaveSampleRate::F48000,
                        WaveSampleRate::F44100 => wave.sample_rate == WaveSampleRate::F44100,
                        WaveSampleRate::F96000 => wave.sample_rate == WaveSampleRate::F96000,
                        _ => true,
                    };
                    if !matches {
                        self.selected_index = None;
                    }
                }
            }
        });

        // Search field
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_text).hint_text("Search wav files..."),
            );
            if ui.button("Clear").clicked() {
                self.search_text.clear();
            }
        });

        // Collect filtered items (indices only)
        let filtered_items: Vec<usize> = self
            .relative_paths
            .iter()
            .enumerate()
            .filter(|(index, rel_path)| {
                let wave = &self.file_manager.wave_data[*index];
                let sample_rate_ok = match self.sample_rate_filter {
                    WaveSampleRate::F48000 => wave.sample_rate == WaveSampleRate::F48000,
                    WaveSampleRate::F44100 => wave.sample_rate == WaveSampleRate::F44100,
                    WaveSampleRate::F96000 => wave.sample_rate == WaveSampleRate::F96000,
                    _ => true,
                };
                let search_ok = if self.search_text.is_empty() {
                    true
                } else {
                    let search_lower = self.search_text.to_lowercase();
                    let path_lower = rel_path.to_string_lossy().to_lowercase();
                    path_lower.contains(&search_lower)
                };
                sample_rate_ok && search_ok
            })
            .map(|(index, _)| index)
            .collect();

        // If selected file is no longer visible, deselect it
        if let Some(selected_idx) = self.selected_index
            && !filtered_items.contains(&selected_idx) {
                self.selected_index = None;
            }

        if filtered_items.is_empty() {
            ui.label("No .wav files matching this filter were found in the directory.");
        } else {
            self.render_file_table(ui, filtered_items);
            // HRTF metadata frame (detailed view for selected file)
            let frame = egui::Frame::group(ui.style());
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Fixed height scroll area for metadata
                egui::ScrollArea::vertical()
                    .max_height(Self::METADATA_FRAME_HEIGHT)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        if let Some(metadata) = self.selected_metadata() {
                            ui.heading(&metadata.hrtf);
                            ui.label(Self::truncate_description(&metadata.description));
                            if !metadata.source.is_empty() {
                                ui.label(format!("Source: {}", metadata.source));
                            }
                            if !metadata.credits.is_empty() {
                                ui.label(format!("By: {}", metadata.credits));
                            }
                        } else {
                            ui.label("No description for the selected files.");
                        }
                    });
            });
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

            // Determine if a file is selected
            let is_file_selected = self.selected_index.is_some();

            // Add the "Write Config" and the "Delete Config" buttons
            ui.horizontal(|ui| {
                // The "Write config" button should be disabled if no file is selected
                let write_button =
                    ui.add_enabled(is_file_selected, egui::Button::new("Write config"));
                if write_button.clicked() {
                    self.on_write_config_click();
                }

                // The "Delete config" button should be disabled if config is not installed
                let delete_button = ui.add_enabled(
                    self.config_installed.is_some(),
                    egui::Button::new("Delete config"),
                );
                if delete_button.clicked() {
                    self.on_delete_config_click();
                }
            });

            // Display current config status
            match &self.config_installed {
                Some(path) => {
                    ui.label(format!("Current config: {}", path.display()));
                }
                None => {
                    ui.label("No config installed");
                }
            }

            ui.separator();
            self.render_file_list_and_metadata(ui);
        });
    }
}
