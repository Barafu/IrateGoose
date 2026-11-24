use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::cell::RefCell;
use std::rc::Rc;

use crate::app_gui::theme::resolve_theme;
use crate::config_manager::ConfigManager;
use crate::file_manager::{FileManager, WavFileData, WaveSampleRate};
use crate::settings::AppSettings;
use crate::wav_file_index::WavFileIndex;
use egui_commonmark::{CommonMarkCache, commonmark_str};
use log::{error, warn};
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
    pub(crate) settings: Rc<RefCell<AppSettings>>,
    // Manages a collection of WAV files
    pub(crate) file_manager: &'a mut FileManager,
    // Manages writing Pipewire configuration
    config_manager: &'a ConfigManager,
    // Contains data about WAV files
    pub(crate) all_wav_index: WavFileIndex,
    // Cached filtered items (None when dirty)
    pub(crate) filtered_wav_index: Option<WavFileIndex>,
    // Shared log buffer
    log_buffer: Arc<Mutex<Vec<String>>>,

    // === UI state ===
    // Checksum of selected file (None if none selected)
    pub(crate) selected_checksum: Option<u128>,
    // Currently selected sample rate filter
    pub(crate) sample_rate_filter: WaveSampleRate,
    // Checksum of the WAV file set in installed Pipewire config file if any
    // None = no config, Some(0) = config exists but file is damaged, Some(nonzero) = valid checksum
    pub(crate) config_installed: Option<u128>,
    // Search filter text
    pub(crate) search_text: String,
    // Currently selected tab (Files/Options)
    selected_tab: Tab,
    // Directory path displayed in edit field in options tab
    pub(crate) directory_text: String,
    // Virtual device name displayed in edit field in options tab
    pub(crate) device_name_text: String,
    // Service restart command displayed in edit field in options tab
    pub(crate) restart_command_text: String,
    // UI theme preference (local copy for radio buttons)
    pub(crate) theme_preference: eframe::egui::ThemePreference,
    // Row index to scroll to (None if no scroll requested)
    pub(crate) scroll_to_row: Option<usize>,

    // === Output device selection ===
    // List of audio sinks (each a HashMap of properties)
    pub(crate) sinks: Vec<std::collections::HashMap<String, String>>,
    // Selected index in combobox (0 = Auto, 1..len = sink index)
    pub(crate) selected_sink_index: usize,

    // Cache for rendering markdown help content
    help_cache: CommonMarkCache,
    // Whether to scroll help tab to top on next render
    help_scroll_to_top: bool,

    // === Modal state ===
    // Whether modal dialog is open
    modal_open: bool,
    // Modal dialog header text
    modal_header: String,
    // Modal dialog message text
    modal_message: String,
}

impl<'a> AppGUI<'a> {
    pub(crate) const METADATA_FRAME_HEIGHT: f32 = 120.0;

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

        // Initialize restart_command_text from settings
        let restart_command_text = settings.borrow().service_restart_command.clone();

        // Initialize theme preference from settings, resolving System via D-Bus
        let theme_preference = settings.borrow().theme_preference;
        let resolved = resolve_theme(theme_preference);
        cc.egui_ctx.set_theme(resolved);

        AppGUI::adjust_font_size(&cc.egui_ctx, 1.1);

        // Load sinks and compute selected index
        let sinks = match config_manager.list_audio_devices() {
            Ok(devices) => ConfigManager::filter_audio_sinks(&devices),
            Err(e) => {
                error!("Failed to list audio devices: {}", e);
                Vec::new()
            }
        };
        let saved_output_device = settings.borrow().output_device.clone();
        let selected_sink_index = Self::find_sink_index_by_name(&sinks, &saved_output_device)
            .map(|idx| idx + 1) // +1 because index 0 is Auto
            .unwrap_or(0);

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
            restart_command_text,
            theme_preference,
            filtered_wav_index: None,
            scroll_to_row: None,
            sinks,
            selected_sink_index,
            help_cache: CommonMarkCache::default(),
            help_scroll_to_top: true,
        };

        if let Err(e) = result.safe_rescan() {
            error!("Could not rescan wav directory on startup!. Reason: {}", e);
        }
        result
    }

    /// Find sink index by node.name in the sinks list.
    /// Returns None if not found.
    fn find_sink_index_by_name(
        sinks: &[std::collections::HashMap<String, String>],
        node_name: &Option<String>,
    ) -> Option<usize> {
        let node_name = node_name.as_ref()?;
        sinks
            .iter()
            .position(|s| s.get("node.name") == Some(node_name))
    }

    /// Refresh the list of sinks from ConfigManager.
    /// If the currently selected sink (by node.name) is still present in the new list,
    /// keep it selected; otherwise reset to Auto.
    pub(crate) fn refresh_sinks(&mut self) {
        let old_selection = self.settings.borrow().output_device.clone();
        let new_sinks = match self.config_manager.list_audio_devices() {
            Ok(devices) => ConfigManager::filter_audio_sinks(&devices),
            Err(e) => {
                error!("Failed to refresh audio devices: {}", e);
                Vec::new()
            }
        };
        self.sinks = new_sinks;
        // Update selected index
        self.selected_sink_index = Self::find_sink_index_by_name(&self.sinks, &old_selection)
            .map(|idx| idx + 1)
            .unwrap_or(0);
        // If selection changed, update settings
        let new_selection = if self.selected_sink_index == 0 {
            None
        } else {
            self.sinks
                .get(self.selected_sink_index - 1)
                .and_then(|s| s.get("node.name").cloned())
        };
        if old_selection != new_selection {
            self.settings.borrow_mut().output_device = new_selection.clone();
            self.write_settings();
        }
    }

    /// Checks if Pipewire config exists and returns the checksum if found.
    /// Returns None if config doesn't exist or there's an error.
    fn check_config_exists(config_manager: &ConfigManager) -> Option<u128> {
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
            match self.config_manager.write_config(absolute_path) {
                Ok(()) => {
                    // Double-check that config was written correctly and extract the checksum from config
                    match self.config_manager.config_exists() {
                        Ok(Some(checksum)) => {
                            // "Config written" is already logged by ConfigManager,
                            // before any restart command messages.
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
                // "Config deleted" is already logged by ConfigManager, before any
                // restart command messages.
                self.config_installed = None;
            }
            Err(e) => {
                error!("Failed to delete config: {}", e);
            }
        }
    }

    /// Shows a modal dialog with a header, message body, and a "Continue" button.
    /// The modal will be displayed until the user clicks "Continue" or closes it.
    pub(crate) fn show_modal(&mut self, header: &str, message: &str) {
        self.modal_open = true;
        self.modal_header = header.to_string();
        self.modal_message = message.to_string();
    }

    /// Find wav data by checksum.
    pub(crate) fn find_wav_by_checksum(&self, checksum: u128) -> Option<&WavFileData> {
        self.all_wav_index.get_by_checksum(checksum)
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
                if self.help_scroll_to_top {
                    ui.scroll_to_cursor(Some(egui::Align::TOP));
                    self.help_scroll_to_top = false;
                }

                // About section
                ui.heading("About");
                ui.label(format!("Irate Goose v{}", VERSION));
                ui.hyperlink_to("Home page", REPOSITORY);

                ui.separator();

                // Render the help markdown file
                commonmark_str!(ui, &mut self.help_cache, "docs/goose_help.md");
            });
    }

    /// Write current settings to disk.
    pub(crate) fn write_settings(&mut self) {
        let save_result = self.settings.borrow().save();
        if let Err(e) = save_result {
            self.show_modal("Settings Error", &format!("Failed to save settings: {}", e));
        }
    }

    /// Fixes font sizes
    fn adjust_font_size(ctx: &egui::Context, factor: f32) {
        ctx.all_styles_mut(|style| {
            for font_id in style.text_styles.values_mut() {
                font_id.size = (font_id.size * factor).max(1.0);
            }
        });
    }
}

impl<'a> eframe::App for AppGUI<'a> {
    /// Build the application UI: status bar and the central panel.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("status_panel").show(ui, |ui| {
            // Add status bar at the bottom
            ui.horizontal(|ui| {
                if self.settings.borrow().dev_mode {
                    // Blink the DEV marker between yellow and black at 1 Hz.
                    let time = ui.input(|i| i.time);
                    let blink_on = (time * 2.0).fract() < 0.5;
                    let color = if blink_on {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::BLACK
                    };

                    ui.label(egui::RichText::new("DEV").color(color).strong());

                    // egui idles between events; request a repaint so the blink keeps animating.
                    ui.request_repaint_after(std::time::Duration::from_millis(100));
                }
                // Get the last line from the log buffer
                let last_log = self
                    .log_buffer
                    .lock()
                    .ok()
                    .and_then(|guard| guard.last().cloned())
                    .unwrap_or_default();
                ui.label(last_log);
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Create Virtual Device");

            // Determine if a file is selected
            let is_file_selected = self.selected_checksum.is_some();

            // Add the "Write Config" and the "Delete Config" buttons
            ui.horizontal(|ui| {
                ui.style_mut().spacing.button_padding = (8.0, 6.0).into();
                // The "Write config" button should be disabled if no file is selected
                let button_text = match self.config_installed {
                    Some(_) => "💾 Update device",
                    None => "💾 Create device",
                };
                let write_button = ui.add_enabled(
                    is_file_selected,
                    egui::Button::new(
                        egui::RichText::new(button_text).heading()
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
                let modal = egui::Modal::new(egui::Id::unique("message_modal")).show(ui.ctx(), |ui| {
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
