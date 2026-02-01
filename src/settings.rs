#![allow(dead_code)]

use anyhow::{Context, Result};
use eframe::egui::ThemePreference;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default virtual device name used when no custom name is provided.
pub const DEFAULT_VIRTUAL_DEVICE_NAME: &str = "Virtual Surround Sink";

/// Application settings for IrateGoose (NOT PipeWire settings).
/// These settings control the application behavior, such as WAV directory
/// preferences and virtual device naming, and are stored separately from
/// the PipeWire configuration managed by ConfigManager.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Path to the WAV files directory
    wav_directory: Option<PathBuf>,

    /// Virtual device name for PipeWire
    pub virtual_device_name: String,

    /// UI theme preference (Light, Dark, or follow system)
    pub theme_preference: ThemePreference,

    /// Active WAV directory (runtime only, not persisted)
    #[serde(skip)]
    active_wav_directory: Option<PathBuf>,

    /// Development mode flag (runtime only, not persisted)
    #[serde(skip)]
    pub dev_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            wav_directory: None,
            virtual_device_name: DEFAULT_VIRTUAL_DEVICE_NAME.to_string(),
            theme_preference: ThemePreference::System,
            active_wav_directory: None,
            dev_mode: false,
        }
    }
}

impl AppSettings {
    /// Loads settings from a TOML string
    fn load_from_str(toml_str: &str) -> Result<Self> {
        let settings: AppSettings =
            toml::from_str(toml_str).context("Failed to parse settings TOML")?;

        Ok(settings)
    }

    /// Saves settings to a TOML string
    fn save_to_str(&self) -> Result<String> {
        let toml_string =
            toml::to_string_pretty(self).context("Failed to serialize settings to TOML")?;

        Ok(toml_string)
    }

    /// Gets the default settings file path
    fn default_settings_path(&self) -> Result<PathBuf> {
        if self.dev_mode {
            // In dev mode, use a file in the current directory
            Ok(std::env::current_dir()?.join("irate_goose_dev_settings.toml"))
        } else {
            // In normal mode, use the standard config directory
            let config_dir = dirs::config_dir().context("Could not determine config directory")?;

            Ok(config_dir.join("irate_goose").join("settings.toml"))
        }
    }

    /// Write settings to a file
    fn write_settings_to_file(&self, path: &std::path::Path) -> Result<()> {
        let toml_string = self.save_to_str()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(path, toml_string)
            .with_context(|| format!("Failed to write settings to: {}", path.display()))?;

        Ok(())
    }

    /// Read settings from a file
    fn read_settings_from_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read settings file: {}", path.display()))?;

        Self::load_from_str(&content)
            .with_context(|| format!("Failed to parse settings TOML: {}", path.display()))
    }

    /// Load settings from the default settings file
    /// Uses self.dev_mode to determine which file to load from
    /// The loaded settings will have the same dev_mode as self
    pub fn load(&self) -> Result<Self> {
        let path = self.default_settings_path()?;
        let mut settings = Self::read_settings_from_file(&path)?;
        settings.dev_mode = self.dev_mode;
        Ok(settings)
    }

    /// Save settings to the default settings file
    pub fn save(&self) -> Result<()> {
        let path = self.default_settings_path()?;
        self.write_settings_to_file(&path)
    }

    /// Get the WAV directory to use
    pub fn get_wav_directory(&self) -> Option<PathBuf> {
        self.active_wav_directory
            .clone()
            .or_else(|| self.wav_directory.clone())
    }

    /// Set the WAV directory
    pub fn set_wav_directory(&mut self, path: PathBuf) {
        self.wav_directory = Some(path);
        self.active_wav_directory = None;
    }
    /// Set temporary WAV directory
    pub fn set_temp_wav_directory(&mut self, path: PathBuf) {
        self.active_wav_directory = Some(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_str_and_save_to_str() {
        // Create a settings instance with some values
        let mut settings = AppSettings::default();
        settings.wav_directory = Some(std::path::PathBuf::from("/test/path/to/wav"));
        settings.virtual_device_name = "Test Virtual Device".to_string();

        // Save to string
        let saved_str = settings
            .save_to_str()
            .expect("Failed to save settings to string");

        // Verify the string contains expected TOML structure
        assert!(saved_str.contains("wav_directory"));
        assert!(saved_str.contains("virtual_device_name"));
        assert!(saved_str.contains("Test Virtual Device"));

        // Load from the string
        let loaded_settings =
            AppSettings::load_from_str(&saved_str).expect("Failed to load settings from string");

        // Verify the loaded settings match the original
        assert_eq!(loaded_settings.wav_directory, settings.wav_directory);
        assert_eq!(
            loaded_settings.virtual_device_name,
            settings.virtual_device_name
        );

        // Test with default settings
        let default_settings = AppSettings::default();
        let default_str = default_settings
            .save_to_str()
            .expect("Failed to save default settings");
        let loaded_default =
            AppSettings::load_from_str(&default_str).expect("Failed to load default settings");

        assert_eq!(loaded_default.wav_directory, default_settings.wav_directory);
        assert_eq!(
            loaded_default.virtual_device_name,
            default_settings.virtual_device_name
        );
    }
}
