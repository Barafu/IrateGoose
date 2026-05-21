use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::fs;
use std::path::{Path, PathBuf};

use super::AppGUI;
use crate::file_manager::{WavFileData, WaveSampleRate};
use crate::wav_file_index::WavFileIndex;
use log::info;
use walkdir::WalkDir;

impl<'a> AppGUI<'a> {
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
        if self.selected_checksum != old_checksum
            && self.selected_checksum.is_some()
            && let Some(filtered) = self.filtered_wav_index.as_ref()
            && let Some(row) = filtered.index_of_checksum(self.selected_checksum.unwrap())
        {
            self.scroll_to_row = Some(row);
        }
    }

    /// Gives access to filtered items index, recreating it if it is None.
    fn get_filtered_wav_files(&mut self) -> &WavFileIndex {
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
        if let Some(checksum) = self.selected_checksum
            && let Some(row) = self
                .filtered_wav_index
                .as_ref()
                .unwrap()
                .index_of_checksum(checksum)
        {
            self.scroll_to_row = Some(row);
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
            let num_rows = self.get_filtered_wav_files().len();
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
                        let selected_checksum: Option<u128> = self.selected_checksum;
                        let wave: &WavFileData = self
                            .get_filtered_wav_files()
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
    pub(crate) fn render_file_list_and_metadata(&mut self, ui: &mut egui::Ui) {
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

        if self.all_wav_index.len() == 0 {
            ui.label("");
            ui.label("Irate Goose needs IR (Impulse Response) files to create a virtual surround sound effect.");
            ui.label("No IR files were found in the selected directory.");
            ui.label("Some ways to obtain IR files are described on the project page:");
            ui.hyperlink_to("Irate Goose GitHub", "https://github.com/Barafu/IrateGoose");
        } else if self.get_filtered_wav_files().len() == 0 {
            ui.label("");
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

    /// Handles the "Rescan" button click for WAV directory.
    pub(crate) fn on_rescan_click(&mut self) {
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
    pub(crate) fn safe_rescan(&mut self) -> anyhow::Result<()> {
        // Clean filtered_items, just to be sure
        self.filtered_wav_index = None;

        // Auto-descend: if the selected directory has no WAV files at root level
        // and exactly one subfolder, descend into that subfolder
        {
            let dir = self.settings.borrow().get_wav_directory();
            if let Some(ref dir_path) = dir {
                if dir_path.is_dir()
                    && !Self::dir_has_wav_files(dir_path)
                    && Self::subdir_count(dir_path) == 1
                {
                    let new_dir = Self::find_single_subdir(dir_path);
                    self.directory_text = new_dir.to_string_lossy().to_string();
                    self.settings.borrow_mut().set_wav_directory(Some(new_dir));
                    self.write_settings();
                    return self.safe_rescan();
                }
            }
        }

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

        // If no WAV files found, check if the directory contains .tar.zstd archives
        if self.all_wav_index.len() == 0 && self.contains_tar_zstd() {
            self.show_modal(
                "Archives Found",
                "No IR files were found in the directory, but .tar.zstd archives were detected.\n\n\
                You need to unpack the archive files before IrateGoose can use them.\n\
                Navigate to the project page for instructions on how to obtain and install IR files.",
            );
        }

        Ok(())
    }

    /// Checks the configured WAV directory for `.tar.zstd` archives.
    fn contains_tar_zstd(&self) -> bool {
        let dir = match self.settings.borrow().get_wav_directory() {
            Some(d) => d,
            None => return false,
        };
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|entry| {
                entry.file_type().is_file()
                    && entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("zstd"))
                    && entry
                        .path()
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.ends_with(".tar"))
            })
    }

    /// Checks if a directory has any `.wav` files at the root level (non-recursive).
    fn dir_has_wav_files(dir: &Path) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        entries.flatten().any(|entry| {
            entry.path().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
        })
    }

    /// Counts immediate subdirectories in a directory.
    fn subdir_count(dir: &Path) -> usize {
        let Ok(entries) = fs::read_dir(dir) else {
            return 0;
        };
        entries.flatten().filter(|e| e.path().is_dir()).count()
    }

    /// Returns the only subdirectory path, panicking if not exactly one exists.
    /// Call only after verifying `subdir_count(dir) == 1`.
    fn find_single_subdir(dir: &Path) -> PathBuf {
        fs::read_dir(dir)
            .unwrap()
            .flatten()
            .find(|e| e.path().is_dir())
            .unwrap()
            .path()
    }
}
