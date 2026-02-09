use eframe::egui;
use egui_extras::{Column, TableBuilder};
use rfd::FileDialog;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::config_manager::ConfigManager;
use crate::file_manager::{FileManager, WavFileData, WaveSampleRate};
use crate::settings::{AppSettings, DEFAULT_VIRTUAL_DEVICE_NAME};
use crate::wav_file_index::WavFileIndex;
use log::{error, info, warn};
use std::sync::{Arc, Mutex};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(PartialEq, Eq, Clone, Copy)]
/// Represents the selected tab in the main window.
enum Tab {
    Files,
    Options,
    Log,
    Help,
}

pub struct AppGUI<'a> {
    // === App data ===
    // Application settings
    settings: Rc<RefCell<AppSettings>>,
    // Manages a collection of WAV files
    file_manager: &'a mut FileManager,
    // Manages writing Pipewire configuration
    config_manager: &'a ConfigManager,
    // Contains data about WAV files
    all_wav_index: WavFileIndex,
    // Cached filtered items (None when dirty)
    filtered_wav_index: Option<WavFileIndex>,
    // Shared log buffer
    log_buffer: Arc<Mutex<Vec<String>>>,

    // === UI state ===
    // Checksum of selected file (None if none selected)
    selected_checksum: Option<u64>,
    // Currently selected sample rate filter
    sample_rate_filter: WaveSampleRate,
    // Checksum of the WAV file set in installed Pipewire config file if any
    // None = no config, Some(0) = config exists but file is damaged, Some(nonzero) = valid checksum
    config_installed: Option<u64>,
    // Search filter text
    search_text: String,
    // Currently selected tab (Files/Options)
    selected_tab: Tab,
    // Directory path displayed in edit field in options tab
    directory_text: String,
    // Virtual device name displayed in edit field in options tab
    device_name_text: String,
    // UI theme preference (local copy for radio buttons)
    theme_preference: eframe::egui::ThemePreference,
    // Row index to scroll to (None if no scroll requested)
    scroll_to_row: Option<usize>,

    // === Modal state ===
    // Whether modal dialog is open
    modal_open: bool,
    // Modal dialog header text
    modal_header: String,
    // Modal dialog message text
    modal_message: String,
}

impl<'a> AppGUI<'a> {
    const METADATA_FRAME_HEIGHT: f32 = 120.0;

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Rc<RefCell<AppSettings>>,
        file_manager: &'a mut FileManager,
        config_manager: &'a ConfigManager,
        log_buffer: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.

        let config_installed = Self::check_config_exists(config_manager);
        let sample_rate_filter = WaveSampleRate::F48000;

        // Initialize directory_text from settings
        let current_dir = settings.borrow().get_wav_directory();
        let directory_text = current_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Initialize device_name_text from settings
        let device_name_text = settings.borrow().virtual_device_name.clone();

        // Initialize theme preference from settings and apply it
        let theme_preference = settings.borrow().theme_preference;
        cc.egui_ctx.set_theme(theme_preference);

        let mut result = Self {
            settings,
            file_manager,
            config_manager,
            all_wav_index: WavFileIndex::new(),
            log_buffer,
            selected_checksum: None,
            sample_rate_filter,
            config_installed,
            search_text: String::new(),
            selected_tab: Tab::Files,
            modal_open: false,
            modal_header: String::new(),
            modal_message: String::new(),
            directory_text,
            device_name_text,
            theme_preference,
            filtered_wav_index: None,
            scroll_to_row: None,
        };

        if let Err(e) = result.safe_rescan() {
            error!("Could not rescan wav directory on startup!. Reason: {}", e);
        }
        result
    }

    /// Checks if Pipewire config exists and returns the checksum if found.
    /// Returns None if config doesn't exist or there's an error.
    fn check_config_exists(config_manager: &ConfigManager) -> Option<u64> {
        match config_manager.config_exists() {
            Ok(Some(checksum)) => Some(checksum),
            Ok(None) => None,
            Err(e) => {
                error!("Error checking config: {}", e);
                None
            }
        }
    }

    fn on_write_config_click(&mut self) {
        if let Some(checksum) = self.selected_checksum {
            let selected_wav = match self.find_wav_by_checksum(checksum) {
                Some(wave) => wave,
                None => {
                    error!("Selected file not found");
                    return;
                }
            };
            let absolute_path = selected_wav.path.as_path();
            let display_path = absolute_path.display().to_string();
            match self.config_manager.write_config(absolute_path) {
                Ok(()) => {
                    // Double-check that config was written correctly and extract the checksum from config
                    match self.config_manager.config_exists() {
                        Ok(Some(checksum)) => {
                            info!("Config written using {}", display_path);
                            self.config_installed = Some(checksum);
                        }
                        Ok(None) => {
                            // Config file doesn't exist after writing - something went wrong
                            error!("Config written but not found afterwards");
                            self.config_installed = None;
                        }
                        Err(e) => {
                            // Error reading config after write
                            error!("Config written but error verifying: {}", e);
                            self.config_installed = None;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to write config: {}", e);
                }
            }
        } else {
            warn!("No file selected");
        }
    }

    fn on_delete_config_click(&mut self) {
        match self.config_manager.delete_config() {
            Ok(()) => {
                info!("Config deleted");
                self.config_installed = None;
            }
            Err(e) => {
                error!("Failed to delete config: {}", e);
            }
        }
    }


    /// Shows a modal dialog with a header, message body, and a "Continue" button.
    /// The modal will be displayed until the user clicks "Continue" or closes it.
    fn show_modal(&mut self, header: &str, message: &str) {
        self.modal_open = true;
        self.modal_header = header.to_string();
        self.modal_message = message.to_string();
    }

    /// Find wav data by checksum.
    fn find_wav_by_checksum(&self, checksum: u64) -> Option<&WavFileData> {
        self.all_wav_index.get_by_checksum(checksum)
    }

    /// Get HRTF metadata for the currently selected file, if any.
    fn selected_metadata(&self) -> Option<&crate::descriptions::HRTFMetadata> {
        let checksum = self.selected_checksum?;
        let wave = self.find_wav_by_checksum(checksum)?;
        wave.metadata.as_deref()
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

    /// Auto‑select the file that matches the installed config (if any).
    fn apply_auto_selection(&mut self) {
        let old_checksum = self.selected_checksum;
        match self.config_installed {
            Some(checksum) if checksum != 0 => {
                if self.find_wav_by_checksum(checksum).is_some() {
                    self.selected_checksum = Some(checksum);
                } else {
                    self.selected_checksum = None;
                }
            }
            _ => {
                self.selected_checksum = None;
            }
        }
        // If selection changed (or newly selected) and we have a filtered index,
        // scroll to the selected row if it's present in the filtered list.
        if self.selected_checksum != old_checksum && self.selected_checksum.is_some() {
            if let Some(filtered) = self.filtered_wav_index.as_ref() {
                if let Some(row) = filtered.index_of_checksum(self.selected_checksum.unwrap()) {
                    self.scroll_to_row = Some(row);
                }
            }
        }
    }

    /// Gives access to filtered items index, recreating it if it is None.
    fn get_filtered_items(&mut self) -> &WavFileIndex {
        if self.filtered_wav_index.is_some() {
            return self.filtered_wav_index.as_ref().unwrap();
        }
        let filter_predicate = |wave: &&WavFileData| {
            let sample_rate_ok = match self.sample_rate_filter {
                WaveSampleRate::Unknown => true,
                WaveSampleRate::Damaged => false,
                _ => wave.sample_rate == self.sample_rate_filter,
            };
            let search_ok = if self.search_text.is_empty() {
                true
            } else {
                let search_lower = self.search_text.to_lowercase();
                let path_lower = wave.relative_path.to_string_lossy().to_lowercase();
                path_lower.contains(&search_lower)
            };
            sample_rate_ok && search_ok
        };
        self.filtered_wav_index = Some(self.all_wav_index.filtered_clone(filter_predicate));
        // After recreating the filtered index, scroll to the selected row if present
        if let Some(checksum) = self.selected_checksum {
            if let Some(row) = self.filtered_wav_index.as_ref().unwrap().index_of_checksum(checksum) {
                self.scroll_to_row = Some(row);
            }
        }
        self.filtered_wav_index.as_ref().unwrap()
    }

    /// Renders the file table with two columns: "Files" and "Description".
    fn render_file_table(&mut self, ui: &mut egui::Ui) {
        // Wrap the table in its own frame
        let table_frame = egui::Frame::group(ui.style());
        table_frame.show(ui, |ui| {
            // Create a two-column table using rows() for better performance
            let row_height = 20.0;
            let num_rows = self.get_filtered_items().len();
            let available_width = ui.available_width();
            let available_height: f32 = ui.available_height() - Self::METADATA_FRAME_HEIGHT;

            let mut table_builder = TableBuilder::new(ui)
                .column(Column::initial(available_width * 0.6)) // "Files" column - auto width
                .column(Column::remainder().clip(true)) // "Description" column - takes remaining width
                .max_scroll_height(available_height)
                .auto_shrink([false, false]) // Vertical auto_shrink false to always use available height
                .resizable(true)
                .striped(true)
                .sense(egui::Sense::click()) // Make rows clickable
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center)); // Center content vertically

            // Take the scroll request (if any) so we don't scroll again next frame
            let scroll_row = self.scroll_to_row.take();
            // Apply scroll if requested
            if let Some(row) = scroll_row {
                table_builder = table_builder.scroll_to_row(row, None);
            }

            table_builder
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
                        let selected_checksum: Option<u64> = self.selected_checksum;
                        let wave: &WavFileData = self
                            .get_filtered_items()
                            .get_by_index(row.index())
                            .expect("Index out of bounds in table.rows()");
                        let rel_path: &PathBuf = &wave.relative_path;
                        let is_selected: bool = selected_checksum == Some(wave.checksum);
                        let mut label_text: String = rel_path.to_string_lossy().to_string();

                        // Get HRTF metadata for this file (cheap lookup)
                        let description_text: String = wave
                            .metadata
                            .as_ref()
                            .map(|rc| {
                                // Take first line of description, fallback to empty string
                                rc.description
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
                                ui.add(egui::Label::new(label_text).truncate().selectable(false));
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
                            self.selected_checksum = Some(wave.checksum);
                        }
                    });
                });
        });
    }

    /// Renders the file list table with two columns: "Files" and "Description".
    fn render_file_list_and_metadata(&mut self, ui: &mut egui::Ui) {
        ui.heading("Located IR Files");

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
                // Invalidate cached filtered items
                self.filtered_wav_index = None;
            }
        });

        // Search field
        ui.horizontal(|ui| {
            let old_search = self.search_text.clone();
            ui.add(
                egui::TextEdit::singleline(&mut self.search_text).hint_text("Search IR files..."),
            );
            if ui.button("Clear").clicked() {
                self.search_text.clear();
            }
            // If search text changed, invalidate cached filtered items
            if old_search != self.search_text {
                self.filtered_wav_index = None;
            }
        });

        if self.get_filtered_items().len() == 0 {
            ui.label("No .wav files matching this filter were found in the directory.");
        } else {
            self.render_file_table(ui);
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

    /// Renders the options tab content.
    fn render_options(&mut self, ui: &mut egui::Ui) {
        ui.heading("IR files Directory");
        ui.label("Set the directory containing IR files for surround sound:");

        ui.horizontal(|ui| {
            ui.label("Directory:");
            ui.add(
                egui::TextEdit::singleline(&mut self.directory_text).hint_text("Path to IR files"),
            );
            if ui.button("Select").clicked() {
                // Create file dialog for directory selection
                let mut dialog = FileDialog::new().set_title("Select IR Files Directory");

                // Try to set starting directory from current directory_text if it's a valid path
                let current_dir = self.directory_text.trim();
                if !current_dir.is_empty() {
                    let path = PathBuf::from(current_dir);
                    if path.exists() && path.is_dir() {
                        dialog = dialog.set_directory(path);
                    }
                }

                // Show directory picker dialog
                if let Some(selected_folder) = dialog.pick_folder() {
                    // Update directory text field with selected path
                    self.directory_text = selected_folder.to_string_lossy().to_string();
                    // Automatically trigger rescan for the newly selected directory
                    self.on_rescan_click();
                }
            }
            let rescan_enabled = !self.directory_text.trim().is_empty();
            let rescan_button = ui.add_enabled(rescan_enabled, egui::Button::new("Rescan"));
            if rescan_button.clicked() {
                self.on_rescan_click();
            }
        });

        ui.separator();

        ui.heading("Virtual Device Name");
        ui.label("Set the name of the virtual audio device that will appear in your system audio settings:");

        // Display currently configured device name
        let current_device_name = self.settings.borrow().virtual_device_name.clone();
        ui.label(format!("Currently configured: {}", current_device_name));

        ui.horizontal(|ui| {
            ui.label("Device name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.device_name_text)
                    .hint_text("Virtual device name"),
            );

            // Get current device name inside the closure to avoid borrowing issues
            let current_device_name = self.settings.borrow().virtual_device_name.clone();
            let default_value = DEFAULT_VIRTUAL_DEVICE_NAME;

            // Apply button should be disabled when text matches current settings
            let trimmed_text = self.device_name_text.trim().to_string();
            let apply_enabled = !trimmed_text.is_empty() && trimmed_text != current_device_name;
            let apply_button = ui.add_enabled(apply_enabled, egui::Button::new("Apply"));
            if apply_button.clicked() {
                self.on_apply_device_name_click(&trimmed_text);
            }

            // Default button must be disabled when text matches default value
            // AND default value matches the value in settings
            let default_button_enabled =
                !(trimmed_text == default_value && current_device_name == default_value);
            let default_button =
                ui.add_enabled(default_button_enabled, egui::Button::new("Default"));
            if default_button.clicked() {
                self.on_default_device_name_click();
            }
        });

        ui.separator();

        ui.heading("UI Theme");
        ui.label("Select the application visual theme:");
        let old_preference = self.theme_preference;
        self.theme_preference.radio_buttons(ui);
        if self.theme_preference != old_preference {
            // Update settings
            self.settings.borrow_mut().theme_preference = self.theme_preference;
            self.write_settings();
            // Apply immediately
            ui.ctx().set_theme(self.theme_preference);
        }

        if self.settings.borrow().dev_mode {
            // Developer-only buttons
            ui.separator();
            if ui.button("Show modal test message").clicked() {
                self.show_modal("Test Modal", "This is a test message to demonstrate the modal dialog functionality. Click 'Continue' to close this dialog.");
            }
        }
    }

    /// Renders the log tab content.
    fn render_log(&mut self, ui: &mut egui::Ui) {
        // Update cached log text from buffer
        let logs = match self.log_buffer.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        };

        let row_height = ui.text_style_height(&egui::TextStyle::Body);
        let num_rows = logs.len();
        let available_height = ui.available_height();

        TableBuilder::new(ui)
            .column(Column::remainder())
            .max_scroll_height(available_height)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .striped(true)
            .body(|body| {
                body.rows(row_height, num_rows, |mut row| {
                    let logline = &logs[row.index()];
                    row.col(|ui| {
                        ui.label(logline);
                    });
                });
            });
    }

    /// Renders the help tab content.
    fn render_help(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                // About section
                ui.heading("About");
                ui.label(format!("IrateGoose v{}", VERSION));
                ui.hyperlink_to("Home page", REPOSITORY);
                
                ui.separator();
                
                // Placeholder for future help content
                ui.heading("Help");
                ui.label("Help content will be added here in a future version.");
                ui.label("This tab will contain usage instructions and troubleshooting information.");
            });
    }

    /// Handles the "Rescan" button click for WAV directory.
    fn on_rescan_click(&mut self) {
        let dir_text = self.directory_text.trim().to_string();
        if dir_text.is_empty() {
            return;
        }

        let path = PathBuf::from(&dir_text);

        // Check if directory exists and is a directory
        if !path.exists() {
            self.show_modal(
                "Directory Not Found",
                "The specified directory does not exist.",
            );
            return;
        }

        if !path.is_dir() {
            self.show_modal("Not a Directory", "The specified path is not a directory.");
            return;
        }

        // Invalidate filtered items cache
        self.filtered_wav_index = None;

        // Perform safe rescan with the new directory (safe_rescan will handle persistence)
        // We need to set the directory in settings, but safe_rescan will temporarily set to None.
        // However, safe_rescan expects wav_directory to already be set.
        self.settings.borrow_mut().set_wav_directory(Some(path));

        match self.safe_rescan() {
            Ok(_) => {
                info!(
                    "Scanned IR directory: {} ({} files found)",
                    dir_text,
                    self.all_wav_index.len()
                );
            }
            Err(e) => {
                self.show_modal(
                    "Rescan Error",
                    &format!("Failed to rescan directory: {}", e),
                );
            }
        }
    }

    /// Performs a safe rescan. The purpose is to make sure that if
    /// application crashes during rescan, then the faulty directory
    /// is not saved into settings and will not be scanned on restart.
    fn safe_rescan(&mut self) -> anyhow::Result<()> {
        // Clean filtered_items, just to be sure
        self.filtered_wav_index = None;
        // If get_wav_directory is None, skip scanning
        let original_path = self.settings.borrow().get_wav_directory();
        if original_path.is_none() {
            self.all_wav_index.clear();
            return Ok(());
        }

        // If settings.active_wav_directory is used, simply scan
        if !self.settings.borrow().is_wav_directory_set() {
            self.all_wav_index = self.file_manager.rescan_configured_directory()?;
        } else {
            // Temporarily set wav_directory to None and persist
            self.settings.borrow_mut().set_wav_directory(None);
            self.write_settings();

            // Restore original path in memory (but not persisted yet)
            self.settings.borrow_mut().set_wav_directory(original_path);

            // Perform the actual scan
            self.all_wav_index = self.file_manager.rescan_configured_directory()?;

            // Persist the directory after successful scan
            self.write_settings();
        }

        // Update UI state
        // Keep selected_checksum, but verify it still exists after rescan
        if let Some(checksum) = self.selected_checksum
            && self.find_wav_by_checksum(checksum).is_none()
        {
            self.selected_checksum = None;
        }
        // Auto‑select the file that matches the installed config (if any)
        self.apply_auto_selection();
        Ok(())
    }

    /// Handles the "Apply" button click for virtual device name.
    fn on_apply_device_name_click(&mut self, trimmed_text: &str) {
        debug_assert!(!trimmed_text.is_empty());

        // Update settings
        {
            let mut settings = self.settings.borrow_mut();
            settings.virtual_device_name = trimmed_text.to_string();
        }

        // Save settings
        self.write_settings();

        // Show success message
        info!("Device name updated to '{}'", trimmed_text);
    }

    /// Handles the "Default" button click for virtual device name.
    fn on_default_device_name_click(&mut self) {
        // Set text to default
        self.device_name_text = DEFAULT_VIRTUAL_DEVICE_NAME.to_string();
        self.on_apply_device_name_click(DEFAULT_VIRTUAL_DEVICE_NAME);
    }

    /// Write current settings to disk.
    fn write_settings(&mut self) {
        let save_result = self.settings.borrow().save();
        if let Err(e) = save_result {
            self.show_modal("Settings Error", &format!("Failed to save settings: {}", e));
        }
    }
}

impl<'a> eframe::App for AppGUI<'a> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
            // Add status bar at the bottom
            ui.horizontal(|ui| {
                if self.settings.borrow().dev_mode {
                    ui.label(
                        egui::RichText::new("DEV")
                            .color(egui::Color32::YELLOW)
                            .strong(),
                    );
                }
                // Get the last line from the log buffer
                let last_log = self.log_buffer.lock().ok()
                    .and_then(|guard| guard.last().cloned())
                    .unwrap_or_default();
                ui.label(last_log);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Create Virtual Device");

            // Determine if a file is selected
            let is_file_selected = self.selected_checksum.is_some();

            // Add the "Write Config" and the "Delete Config" buttons
            ui.horizontal(|ui| {
                ui.style_mut().spacing.button_padding = (8.0, 6.0).into();
                // The "Write config" button should be disabled if no file is selected
                let write_button = ui.add_enabled(
                    is_file_selected,
                    egui::Button::new(
                        egui::RichText::new("💾 Create device").heading()
                    )
                );
                if write_button.clicked() {
                    self.on_write_config_click();
                }
                if !write_button.enabled() && write_button.hovered() {
                    write_button.on_hover_text("Select a IR file to proceed.");
                }

                ui.style_mut().spacing.button_padding = (6.0, 4.0).into();
                // The "Delete config" button should be disabled if config is not installed
                let delete_button = ui.add_enabled(
                    self.config_installed.is_some(),
                    egui::Button::new("❌ Remove device"),
                );
                if delete_button.clicked() {
                    self.on_delete_config_click();
                }
            });

            // Display current config status
            match self.config_installed {
                Some(0) => {
                    ui.label(egui::RichText::new("Warning: The configured IR file is damaged.")
                        .color(egui::Color32::RED));
                }
                Some(checksum) => {
                    if let Some(wave) = self.find_wav_by_checksum(checksum) {
                        ui.label(format!("Current IR file: {}", wave.relative_path.display()));
                    } else {
                        ui.label(egui::RichText::new("Warning: The configured IR file is not found in the current IR directory.")
                            .color(egui::Color32::RED))
                            .on_hover_text("If you create a new virtual device, the content of the IR file currently used will be lost.");
                    }
                }
                None => {
                    ui.label("No config installed");
                }
            }

            ui.separator();

            // Tab selection - all buttons have the same width
            ui.horizontal(|ui| {
                // Use a minimum width that ensures all buttons are the same size
                // The actual width will be determined by the button's content
                let min_button_width = 80.0; // Minimum width, buttons will expand if needed
                
                // Files tab
                if ui.add(
                    egui::Button::selectable(
                        self.selected_tab == Tab::Files,
                        egui::RichText::new("♪ Files").heading(),
                    )
                    .min_size(egui::vec2(min_button_width, ui.spacing().interact_size.y))
                ).clicked() {
                    self.selected_tab = Tab::Files;
                }
                
                // Options tab
                if ui.add(
                    egui::Button::selectable(
                        self.selected_tab == Tab::Options,
                        egui::RichText::new("⚙ Options").heading(),
                    )
                    .min_size(egui::vec2(min_button_width, ui.spacing().interact_size.y))
                ).clicked() {
                    self.selected_tab = Tab::Options;
                }
                
                // Log tab
                if ui.add(
                    egui::Button::selectable(
                        self.selected_tab == Tab::Log,
                        egui::RichText::new("🖹 Log").heading(),
                    )
                    .min_size(egui::vec2(min_button_width, ui.spacing().interact_size.y))
                ).clicked() {
                    self.selected_tab = Tab::Log;
                }
                
                // Help tab
                if ui.add(
                    egui::Button::selectable(
                        self.selected_tab == Tab::Help,
                        egui::RichText::new("❓ Help").heading(),
                    )
                    .min_size(egui::vec2(min_button_width, ui.spacing().interact_size.y))
                ).clicked() {
                    self.selected_tab = Tab::Help;
                }
            });

            ui.separator();

            // Tab content
            match self.selected_tab {
                Tab::Files => {
                    self.render_file_list_and_metadata(ui);
                }
                Tab::Options => {
                    self.render_options(ui);
                }
                Tab::Log => {
                    self.render_log(ui);
                }
                Tab::Help => {
                    self.render_help(ui);
                }
            }

            // Render modal if open
            if self.modal_open {
                let modal = egui::Modal::new(egui::Id::new("message_modal")).show(ctx, |ui| {
                    ui.set_width(300.0);

                    // Header
                    ui.heading(&self.modal_header);

                    // Message body
                    ui.label(&self.modal_message);

                    ui.separator();

                    // Continue button
                    if ui.button("Continue").clicked() {
                        ui.close();
                    }
                });

                if modal.should_close() {
                    self.modal_open = false;
                }
            }
        });
    }
}
