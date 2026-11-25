use eframe::egui;
use log::{error, info};
use rfd::FileDialog;
use std::path::PathBuf;

use super::AppGUI;
use crate::app_gui::theme::resolve_theme;
use crate::settings::{DEFAULT_SERVICE_RESTART_COMMAND, DEFAULT_VIRTUAL_DEVICE_NAME};

/// Common minimum width (in points) of the single-line text edits in the
/// Options tab, so all fields look consistent when empty or short.
const MIN_TEXT_EDIT_WIDTH: f32 = 240.0;

/// Extra width added on top of the displayed content when stretching a text
/// edit, so the caret at the end of the text never sits on the very edge.
const TEXT_EDIT_EXTRA_WIDTH: f32 = 24.0;

impl<'a> AppGUI<'a> {
    /// Generate display text for a sink (two lines).
    /// First line: node.nick if present, else node.name.
    /// Second line: node.name.
    fn sink_display_text(sink: &std::collections::HashMap<String, String>) -> String {
        let node_name = sink
            .get("node.name")
            .filter(|s| !s.is_empty())
            .map(String::as_str)
            .unwrap_or("UNKNOWN DEVICE");
        let first_line = sink
            .get("node.nick")
            .map(String::as_str)
            .unwrap_or(node_name);
        if first_line == node_name {
            return format!("{}", node_name);
        } else {
            return format!("{}\n{}", first_line, node_name);
        }
    }

    /// Generate display text for the currently selected sink (including Auto).
    fn selected_sink_display_text(&self) -> String {
        if self.selected_sink_index == 0 {
            "Auto\nLet Pipewire decide".to_string()
        } else if let Some(sink) = self.sinks.get(self.selected_sink_index - 1) {
            Self::sink_display_text(sink)
        } else {
            "Invalid".to_string()
        }
    }

    /// Width for a single-line text edit in the Options tab: it stretches to
    /// fit its displayed content (the text, or the hint while the field is
    /// empty) plus [`TEXT_EDIT_EXTRA_WIDTH`], but never shrinks below
    /// [`MIN_TEXT_EDIT_WIDTH`], which gives all fields the same minimum width.
    fn text_edit_width(ui: &mut egui::Ui, text: &str, hint: &str) -> f32 {
        let shown = if text.is_empty() { hint } else { text };
        // Measure the text with the same font the text edit renders with
        // (body style), allowing an unbounded wrap width so it stays on one
        // line.
        let style = ui.style();
        let job = egui::text::LayoutJob::simple(
            shown.to_string(),
            style.text_styles[&egui::TextStyle::Body].clone(),
            style.visuals.widgets.noninteractive.fg_stroke.color,
            f32::INFINITY,
        );
        let content_width = ui.fonts_mut(|fonts| fonts.layout_job(job)).rect.width();
        (content_width + TEXT_EDIT_EXTRA_WIDTH).max(MIN_TEXT_EDIT_WIDTH)
    }

    /// Renders the options tab content inside a scrollable area, together with a
    /// fixed footer panel at the bottom holding the Apply button.
    ///
    /// The footer panel is shown first so it claims its space; the scroll area then always fills the
    /// remaining available space so the footer stays at a constant position and the
    /// scrollbar only appears when the content overflows.
    pub(crate) fn render_options(&mut self, ui: &mut egui::Ui) {
        // Compute the validity of all Options values before opening the
        // panel closure; Apply is enabled only by validation and only when
        // the Apply-managed fields actually differ from the current settings.
        let options_valid = self.validate_options();
        let has_changes = self.has_unapplied_changes();

        // Fixed footer panel with the Apply button. Shown inside the central
        // area, before the scroll area, so it always keeps its space at the
        // bottom while the scroll area takes the rest.
        egui::Panel::bottom("options_footer")
            .show_separator_line(false)
            .show(ui, |ui| {
                // Center the Apply button horizontally with a centered top-down
                // layout. `horizontal_centered` cannot be used here: it allocates
                // its child against all remaining space, which would stretch this
                // bottom panel to fill the whole window.
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    // Some vertical margin above and below the button.
                    ui.add_space(8.0);
                    // Match the size and font of the "Create device" button.
                    // The button is added directly to this centered layout;
                    // wrapping it in `ui.horizontal` would stretch to the full
                    // panel width and defeat the centering.
                    ui.style_mut().spacing.button_padding = (16.0, 12.0).into();
                    if ui
                        .add_enabled(
                            options_valid && has_changes,
                            egui::Button::new(egui::RichText::new("✔️Apply").heading()),
                        )
                        .clicked()
                    {
                        self.on_apply_options_click();
                    }
                    ui.add_space(4.0);
                });
            });

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
        ui.heading("IR files Directory");
        ui.label("Set the directory containing IR files for surround sound:");

        ui.horizontal(|ui| {
            ui.label("Directory:");
            // Compute the width before the mutable borrow of the text starts.
            let hint = "Path to IR files";
            let width = Self::text_edit_width(ui, &self.directory_text, hint);
            ui.add(
                egui::TextEdit::singleline(&mut self.directory_text)
                    .hint_text(hint)
                    .desired_width(width),
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
                    // Only update the text field; the directory is copied to
                    // the settings and rescanned when Apply is pressed.
                    self.directory_text = selected_folder.to_string_lossy().to_string();
                }
            }
            // Rescan re-reads the directory already applied in the settings;
            // a pending path in the text field is only applied via Apply.
            let rescan_enabled = self.settings.borrow().get_wav_directory().is_some();
            let rescan_button = ui.add_enabled(rescan_enabled, egui::Button::new("Rescan"));
            if rescan_button.clicked() {
                self.on_rescan_applied_click();
            }
        });

        ui.separator();

        ui.heading("Virtual Device Name");
        ui.label("Set the name of the virtual audio device that will appear in your system audio settings:");

        ui.horizontal(|ui| {
            ui.label("Device name:");
            let hint = "Virtual device name";
            let width = Self::text_edit_width(ui, &self.device_name_text, hint);
            ui.add(
                egui::TextEdit::singleline(&mut self.device_name_text)
                    .hint_text(hint)
                    .desired_width(width),
            );

            let default_value = DEFAULT_VIRTUAL_DEVICE_NAME;
            let trimmed_text = self.device_name_text.trim().to_string();

            // Default only fills the text field, so it is disabled when the
            // field already contains the default value.
            let default_button_enabled = trimmed_text != default_value;
            let default_button =
                ui.add_enabled(default_button_enabled, egui::Button::new("Default"));
            if default_button.clicked() {
                self.on_default_device_name_click();
            }
        });

        ui.separator();

        ui.heading("Output Device");
        ui.label("Select the audio sink where the virtual surround device will output sound:");
        ui.horizontal_top(|ui| {
            ui.label("Output device:");
            // Combobox for sink selection
            let total_items = self.sinks.len() + 1; // +1 for Auto
            egui::ComboBox::from_label("")
                .selected_text(self.selected_sink_display_text())
                .show_index(ui, &mut self.selected_sink_index, total_items, |i| {
                    if i == 0 {
                        "Auto\nLet Pipewire decide".to_string()
                    } else {
                        let sink = &self.sinks[i - 1];
                        Self::sink_display_text(sink)
                    }
                });
            // The selection is kept only in the UI state (selected_sink_index);
            // it is copied to the settings when Apply is pressed.
            // Reload button
            if ui.button("Reload").clicked() {
                self.refresh_sinks();
            }
        });

        ui.separator();

        ui.heading("Service Restart Command");
        ui.label(
            "Command used to restart the audio services when the device is created or removed:",
        );

        ui.horizontal(|ui| {
            ui.label("Command:");
            let hint = "No command set";
            let width = Self::text_edit_width(ui, &self.restart_command_text, hint);
            ui.add(
                egui::TextEdit::singleline(&mut self.restart_command_text)
                    .hint_text(hint)
                    .desired_width(width),
            );


            // Restore the default systemd command into the text field without applying it.
            // Disabled when the field already contains exactly the default command.
            let at_default = self.restart_command_text == DEFAULT_SERVICE_RESTART_COMMAND;
            let use_systemd_button = ui.add_enabled(!at_default, egui::Button::new("Use systemd"));
            if use_systemd_button.clicked() {
                self.restart_command_text = DEFAULT_SERVICE_RESTART_COMMAND.to_string();
            }
        });

        ui.separator();

        ui.heading("UI Theme");
        ui.label("Select the application visual theme:");
        let old_preference = self.theme_preference;
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.theme_preference,
                egui::ThemePreference::Light,
                "🌞 Light",
            );
            ui.selectable_value(
                &mut self.theme_preference,
                egui::ThemePreference::Dark,
                "🌙 Dark",
            );
            ui.selectable_value(
                &mut self.theme_preference,
                egui::ThemePreference::System,
                "🌟 System",
            );
        });
        if self.theme_preference != old_preference {
            // Update settings
            self.settings.borrow_mut().theme_preference = self.theme_preference;
            self.write_settings();
            // Resolve and apply immediately
            let resolved = resolve_theme(self.theme_preference);
            ui.ctx().set_theme(resolved);
        }

        if self.settings.borrow().dev_mode {
            // Developer-only controls
            ui.separator();

            // Allow running the real restart command even in dev mode.
            // Runtime-only flag, not persisted to settings.
            let mut apply_restart = self.settings.borrow().dev_apply_restart_command;
            if ui
                .checkbox(&mut apply_restart, "Apply restart command")
                .changed()
            {
                self.settings.borrow_mut().dev_apply_restart_command = apply_restart;
            }

            if ui.button("Show modal test message").clicked() {
                self.show_modal("Test Modal", "This is a test message to demonstrate the modal dialog functionality. Click 'Continue' to close this dialog.");
            }
        }
            });
    }

    /// Validates all editable Options values before they are applied.
    /// Returns false when any value is invalid: the IR files directory and
    /// the device name must be non-empty; the restart command may be empty,
    /// which means "never restart the services". The directory is only
    /// checked for existence and readability when Apply is pressed.
    fn validate_options(&self) -> bool {
        !self.directory_text.trim().is_empty() && !self.device_name_text.trim().is_empty()
    }

    /// Whether the fields applied by the "Apply" button differ from the
    /// current application settings. Apply persists the IR files directory,
    /// the virtual device name, the service restart command and the output
    /// device (only the theme is applied immediately by its own control).
    ///
    /// The field values are trimmed before comparison because
    /// [`Self::on_apply_options_click`] trims them when writing, so whitespace
    ///-only differences are not real changes.
    fn has_unapplied_changes(&self) -> bool {
        let settings = self.settings.borrow();

        let dir_text = self.directory_text.trim();
        let dir_changed = match settings.get_wav_directory() {
            Some(dir) => dir.to_string_lossy().as_ref() != dir_text,
            None => !dir_text.is_empty(),
        };

        dir_changed
            || self.selected_output_device_name() != settings.output_device
            || self.device_name_text.trim() != settings.virtual_device_name
            || self.restart_command_text.trim() != settings.service_restart_command
    }

    /// Handles the single "Apply" button click in the Options footer.
    /// Collects all option fields, verifies that the IR files directory is
    /// usable, then writes the directory, device name, restart command and
    /// output device to the settings and saves once. When the directory
    /// changed, the file index is rescanned.
    ///
    /// Does nothing when the fields are invalid ([`Self::validate_options`]),
    /// and cancels without saving when the directory does not exist or is
    /// not readable.
    fn on_apply_options_click(&mut self) {
        if !self.validate_options() {
            error!("Options tab is in invalid state!");
            return;
        }

        let device_name = self.device_name_text.trim().to_string();
        let restart_command = self.restart_command_text.trim().to_string();
        let dir_text = self.directory_text.trim().to_string();
        let dir_path = PathBuf::from(&dir_text);

        // Verify the directory exists and is readable before applying any
        // change; on failure the whole apply is cancelled.
        if !dir_path.exists() {
            self.show_modal(
                "Directory Not Found",
                "The specified directory does not exist.",
            );
            return;
        }
        if !dir_path.is_dir() {
            self.show_modal("Not a Directory", "The specified path is not a directory.");
            return;
        }
        if let Err(e) = std::fs::read_dir(&dir_path) {
            self.show_modal(
                "Directory Not Readable",
                &format!("The specified directory cannot be read: {}", e),
            );
            return;
        }

        // Whether the directory differs from the applied one; computed before
        // the settings are overwritten below.
        let dir_changed = match self.settings.borrow().get_wav_directory() {
            Some(dir) => dir.to_string_lossy().as_ref() != dir_text,
            None => true,
        };

        // Write every collected field, then save once.
        {
            let mut settings = self.settings.borrow_mut();
            settings.virtual_device_name = device_name;
            settings.service_restart_command = restart_command;
            settings.output_device = self.selected_output_device_name();
        }

        self.write_settings();

        info!("Settings changed");

        // When the directory changed, apply it and rescan the file index.
        // safe_rescan persists the new directory crash-safely itself.
        if dir_changed {
            self.settings.borrow_mut().set_wav_directory(Some(dir_path));
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
    }

    /// Handles the "Default" button click for virtual device name.
    /// Only fills the text field with the default value; the change is
    /// saved when Apply is pressed.
    fn on_default_device_name_click(&mut self) {
        self.device_name_text = DEFAULT_VIRTUAL_DEVICE_NAME.to_string();
    }
}
