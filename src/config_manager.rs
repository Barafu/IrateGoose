use anyhow::{Context, Result, anyhow};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use crate::settings::AppSettings;
use xxhash_rust::xxh3::xxh3_64;

/// Manages PipeWire configuration files, NOT application configuration.
/// This class handles creation, deletion, and application of PipeWire config files
/// that define virtual audio sinks for surround sound processing.
pub struct ConfigManager {
    /// Full absolute path to the config file
    config_path: PathBuf,
    settings: Rc<RefCell<AppSettings>>,
}

impl ConfigManager {
    /// The config file template
    const CONFIG_TEMPLATE: &'static str = include_str!("../sink_template.conf");

    /// Creates a new ConfigManager instance
    pub fn new(settings: Rc<RefCell<AppSettings>>) -> Result<ConfigManager> {
        // Determine the full path to the current user's ~/.config directory
        let config_dir = dirs::config_dir().ok_or(anyhow!("Could not determine home directory"))?;

        // Determine config suffix based on dev_mode from settings
        // Uses /tmp/surround.conf in dev mode for testing
        // Uses the real PipeWire config path in production mode
        let dev_mode = settings.borrow().dev_mode;
        let config_suffix = if dev_mode {
            "/tmp/surround.conf"
        } else {
            "pipewire/pipewire.conf.d/sink-virtual-surround-7.1-hesuvi.conf"
        };

        // Append the config suffix to get the full absolute path
        let config_path = config_dir.join(config_suffix);

        Ok(Self {
            config_path,
            settings,
        })
    }

    /// Writes the updated configuration to the config path
    pub fn write_config(&self, wavefile_path: &Path) -> Result<()> {
        // Determine the hrir directory (sibling of config file)
        let hrir_dir = self
            .config_path
            .parent()
            .ok_or_else(|| anyhow!("Config path has no parent directory"))?
            .join("hrir");

        // Remove all existing files in the hrir directory
        let _ = fs::remove_dir_all(&hrir_dir);

        // Ensure the hrir directory exists
        fs::create_dir_all(&hrir_dir)
            .with_context(|| format!("Failed to create hrir directory {}", hrir_dir.display()))?;

        // Copy the selected WAV file into the hrir directory, preserving its filename
        let target_path = self.copy_wav_to_hrir(wavefile_path, &hrir_dir)?;

        // Create text for config file using the copied file's absolute path
        let config_text = Self::CONFIG_TEMPLATE
            .replace("IRFILETEMPLATE", target_path.to_string_lossy().as_ref())
            .replace(
                "DEVICENAMETEMPLATE",
                &self.settings.borrow().virtual_device_name,
            );

        // Ensure the parent directory of the config file exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // Write the config file
        if let Err(e) = fs::write(&self.config_path, config_text) {
            // If writing fails, delete any partially written config file.
            let _ = fs::remove_file(&self.config_path);
            return Err(e).with_context(|| {
                format!("Failed to write config to {}", self.config_path.display())
            });
        }

        // Restart services to apply the new config
        if let Err(e) = self.apply_config() {
            // If service restart fails, the config may be unreliable; delete it.
            let _ = fs::remove_file(&self.config_path);
            return Err(e);
        }

        Ok(())
    }

    /// Deletes the config file completely
    pub fn delete_config(&self) -> Result<()> {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path).with_context(|| {
                format!(
                    "Failed to delete config file {}",
                    self.config_path.display()
                )
            })?;
        }
        // Restart services to apply the removal
        self.apply_config()?;
        Ok(())
    }

    /// Checks if the config file exists and returns the checksum of the configured WAV file.
    /// Returns Ok(Some(u64)) if config exists and contains a valid filename; checksum is 0 if file is damaged.
    /// Returns Ok(None) if config file does not exist.
    /// Returns Err(String) if config exists but cannot be read or parsed.
    pub fn config_exists(&self) -> Result<Option<u64>, String> {
        if !self.config_path.exists() {
            return Ok(None);
        }

        // Read the config file
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        // Extract filename from config
        let file_path = Self::extract_filename_from_config(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        // Compute checksum of the referenced WAV file
        let checksum = match fs::read(&file_path) {
            Ok(data) => {
                // Basic WAV header check (optional)
                if data.len() >= 28 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
                    xxh3_64(&data)
                } else {
                    0 // Damaged or not a WAV
                }
            }
            Err(_) => 0, // File missing or unreadable
        };

        Ok(Some(checksum))
    }

    /// Copies a WAV file into the hrir directory, preserving the filename.
    /// Returns the absolute path of the copied file.
    fn copy_wav_to_hrir(&self, source: &Path, hrir_dir: &Path) -> Result<PathBuf> {
        let filename = source
            .file_name()
            .ok_or_else(|| anyhow!("Source path has no filename"))?;
        let target = hrir_dir.join(filename);
        fs::copy(source, &target).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                source.display(),
                target.display()
            )
        })?;
        Ok(target)
    }

    /// Extracts the filename from config content
    /// Looks for pattern: filename = "PATH" (with optional spaces)
    fn extract_filename_from_config(content: &str) -> Result<PathBuf, String> {
        // Search for filename = "..." pattern
        // The pattern could be: filename = "/path/to/file.wav"
        // or: filename = "/home/barafu/Scripts/Surround_WAV/HeSuVi/Common/cmss_ent-/cmss_ent-.wav"
        let re = regex::Regex::new(r#"filename\s*=\s*"([^"]+)"#)
            .map_err(|e| format!("Failed to compile regex: {}", e))?;

        if let Some(captures) = re.captures(content)
            && let Some(filename_match) = captures.get(1)
        {
            let filename = filename_match.as_str();
            return Ok(PathBuf::from(filename));
        }

        Err("No filename found in config".to_string())
    }

    /// Restarts the PipeWire services to apply configuration changes.
    /// Does nothing when in dev mode.
    fn apply_config(&self) -> Result<()> {
        // In dev mode, skip restarting services
        if self.settings.borrow().dev_mode {
            return Ok(());
        }

        let output = Command::new("systemctl")
            .args([
                "--user",
                "restart",
                "wireplumber",
                "pipewire",
                "pipewire-pulse",
            ])
            .output()
            .with_context(|| "Failed to execute systemctl command")?;

        if output.status.success() {
            Ok(())
        } else {
            match output.status.code() {
                Some(5) => Ok(()), // unit not loaded is fine
                Some(code) => Err(anyhow!("systemctl failed with exit code {}", code)),
                None => Err(anyhow!("systemctl terminated by signal")),
            }
        }
    }
}
