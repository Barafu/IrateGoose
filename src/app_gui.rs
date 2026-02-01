use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::config_manager::ConfigManager;
use crate::file_manager::{FileManager, WaveSampleRate};
use crate::settings::{AppSettings, DEFAULT_VIRTUAL_DEVICE_NAME};
use log::{error, info};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(PartialEq, Eq, Clone, Copy)]
/// Represents the selected tab in the main window.
enum Tab {
    Files,
    Options,
}

pub struct AppGUI<'a> {
    // === App data ===
    // Application settings
    settings: Rc<RefCell<AppSettings>>,
    // Manages a collection of WAV files
    file_manager: &'a mut FileManager,
    // Manages writing Pipewire configuration
    config_manager: &'a ConfigManager,

    // === UI state ===
    // Index of selected file in list
    selected_index: Option<usize>,
    // Currently selected sample rate filter
    sample_rate_filter: WaveSampleRate,
    // Path to WAV file set in installed Pipewire config file if any
    config_installed: Option<PathBuf>,
    // Status bar message
    status_message: String,
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

    // === Modal state ===
    // Whether modal dialog is open
    modal_open: bool,
    // Modal dialog header text
    modal_header: String,
    // Modal dialog message text
    modal_message: String,
}

enum MessageLevel {
    Normal,
    Error,
}

impl<'a> AppGUI<'a> {
    const METADATA_FRAME_HEIGHT: f32 = 120.0;

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Rc<RefCell<AppSettings>>,
        file_manager: &'a mut FileManager,
        config_manager: &'a ConfigManager,
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

        // If scan_success is true, perform a safe rescan on startup
        let scan_success = settings.borrow().scan_success;
        if scan_success {
            // Attempt to rescan, but if it fails we keep empty lists
            let _ = Self::safe_rescan_internal(settings.clone(), file_manager);
        }

        Self {
            settings,
            file_manager,
            config_manager,
            selected_index: None,
            sample_rate_filter,
            config_installed,
            status_message: String::new(),
            search_text: String::new(),
            selected_tab: Tab::Files,
            modal_open: false,
            modal_header: String::new(),
            modal_message: String::new(),
            directory_text,
            device_name_text,
            theme_preference,
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
            let selected_wav = &self.file_manager.wave_data[index];
            let absolute_path = selected_wav.path.as_path();
            let display_path = absolute_path.display().to_string();
            match self.config_manager.write_config(absolute_path) {
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

    /// Shows a modal dialog with a header, message body, and a "Continue" button.
    /// The modal will be displayed until the user clicks "Continue" or closes it.
    fn show_modal(&mut self, header: &str, message: &str) {
        self.modal_open = true;
        self.modal_header = header.to_string();
        self.modal_message = message.to_string();
    }

    /// Get HRTF metadata for the currently selected file, if any.
    fn selected_metadata(&self) -> Option<&crate::descriptions::HRTFMetadata> {
        let index = self.selected_index?;
        self.file_manager.wave_data[index].metadata.as_deref()
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
                        let wave = &self.file_manager.wave_data[index];
                        let rel_path = &wave.relative_path;
                        let is_selected = self.selected_index == Some(index);
                        let mut label_text = rel_path.to_string_lossy().to_string();

                        // Get HRTF metadata for this file (cheap lookup)
                        let description_text = wave.metadata.as_ref()
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
                            self.selected_index = Some(index);
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
                // If a file is selected, check if it's still visible with the new filter
                if let Some(index) = self.selected_index {
                    let wave = &self.file_manager.wave_data[index];
                    let matches = match self.sample_rate_filter {
                        WaveSampleRate::Unknown => true,
                        WaveSampleRate::Damaged => false,
                        _ => wave.sample_rate == self.sample_rate_filter
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
                egui::TextEdit::singleline(&mut self.search_text).hint_text("Search IR files..."),
            );
            if ui.button("Clear").clicked() {
                self.search_text.clear();
            }
        });

        // Collect filtered items (indices only)
        let filtered_items: Vec<usize> = self
            .file_manager
            .wave_data
            .iter()
            .enumerate()
            .filter(|(_index, wave)| {
                let sample_rate_ok = match self.sample_rate_filter { 
                        WaveSampleRate::Unknown => true,
                        WaveSampleRate::Damaged => false,
                        _ => wave.sample_rate == self.sample_rate_filter
                    };
                let search_ok = if self.search_text.is_empty() {
                    true
                } else {
                    let search_lower = self.search_text.to_lowercase();
                    let path_lower = wave.relative_path.to_string_lossy().to_lowercase();
                    path_lower.contains(&search_lower)
                };
                sample_rate_ok && search_ok
            })
            .map(|(index, _)| index)
            .collect();

        // If selected file is no longer visible, deselect it
        if let Some(selected_idx) = self.selected_index
            && !filtered_items.contains(&selected_idx)
        {
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
                // TODO: Implement directory picker
                self.show_modal(
                    "Not Implemented",
                    "Directory selection is not yet implemented.",
                );
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

        ui.separator();
        ui.heading("About");
        ui.label(format!("IrateGoose v{}", VERSION));
        ui.hyperlink_to("Home page", REPOSITORY);

        if self.settings.borrow().dev_mode {
            // Developer-only buttons
            ui.separator();
            if ui.button("Show modal test message").clicked() {
                self.show_modal("Test Modal", "This is a test message to demonstrate the modal dialog functionality. Click 'Continue' to close this dialog.");
            }
        }
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

        // Update settings with new directory
        {
            let mut settings = self.settings.borrow_mut();
            settings.set_wav_directory(path.clone());
        }

        // Save settings
        self.write_settings();

        // Perform safe rescan
        match self.safe_rescan() {
            Ok(file_count) => {
                self.message(
                    MessageLevel::Normal,
                    &format!(
                        "Scanned IR directory: {} ({} files found)",
                        dir_text, file_count
                    ),
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

    /// Performs a safe rescan with scan_success flag management.
    /// Returns the number of files scanned on success, or an error.
    fn safe_rescan(&mut self) -> anyhow::Result<usize> {
        // Set scan_success to false before scanning
        {
            let mut settings = self.settings.borrow_mut();
            settings.scan_success = false;
        }
        self.write_settings();

        // Perform the actual scan
        self.file_manager.rescan_configured_directory()?;

        // Update UI state
        self.selected_index = None;
        let file_count = self.file_manager.wave_data.len();

        // Set scan_success to true after successful scan
        {
            let mut settings = self.settings.borrow_mut();
            settings.scan_success = true;
        }
        self.write_settings();

        Ok(file_count)
    }

    /// Internal helper for safe rescan that doesn't update UI.
    /// Used by constructor.
    fn safe_rescan_internal(
        settings: Rc<RefCell<AppSettings>>,
        file_manager: &mut FileManager,
    ) -> anyhow::Result<()> {
        // Set scan_success to false before scanning
        {
            let mut settings = settings.borrow_mut();
            settings.scan_success = false;
        }
        // Save settings (ignore errors for now)
        let _ = settings.borrow().save();

        // Perform the actual scan
        file_manager.rescan_configured_directory()?;

        // Set scan_success to true after successful scan
        {
            let mut settings = settings.borrow_mut();
            settings.scan_success = true;
        }
        let _ = settings.borrow().save();

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
        self.message(
            MessageLevel::Normal,
            &format!("Device name updated to '{}'", trimmed_text),
        );
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
                ui.label(&self.status_message);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Create Virtual Device");

            // Determine if a file is selected
            let is_file_selected = self.selected_index.is_some();

            // Add the "Write Config" and the "Delete Config" buttons
            ui.horizontal(|ui| {
                // The "Write config" button should be disabled if no file is selected
                let write_button =
                    ui.add_enabled(is_file_selected, egui::Button::new("Create device"));
                if write_button.clicked() {
                    self.on_write_config_click();
                }
                if !write_button.enabled() && write_button.hovered() {
                    write_button.on_hover_text("Select a IR file to proceed.");
                }

                // The "Delete config" button should be disabled if config is not installed
                let delete_button = ui.add_enabled(
                    self.config_installed.is_some(),
                    egui::Button::new("Remove device"),
                );
                if delete_button.clicked() {
                    self.on_delete_config_click();
                }
            });

            // Display current config status
            match &self.config_installed {
                Some(path) => {
                    ui.label(format!("Current IR file: {}", path.display()));
                }
                None => {
                    ui.label("No config installed");
                }
            }

            ui.separator();

            // Tab selection
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.selected_tab,
                    Tab::Files,
                    egui::RichText::new("Files").heading(),
                );
                ui.selectable_value(
                    &mut self.selected_tab,
                    Tab::Options,
                    egui::RichText::new("Options").heading(),
                );
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
