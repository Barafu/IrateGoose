use anyhow::{Context, Result, anyhow, bail};
use log::{info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
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
    const CONFIG_TEMPLATE: &'static str = include_str!("../templates/virtual_device.conf.template");

    /// Suffix for virtual surround node names (appended after "effect_input." / "effect_output.")
    const VIRTUAL_NODE_SUFFIX: &str = "virtual-surround-7.1-irategoose";

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
            "pipewire/pipewire.conf.d/sink-virtual-surround-7.1-irategoose.conf"
        };

        // Append the config suffix to get the full absolute path
        let config_path = config_dir.join(config_suffix);

        // Migrate config file from old name to new name
        let old_suffix = "pipewire/pipewire.conf.d/sink-virtual-surround-7.1-hesuvi.conf";
        let old_path = config_dir.join(old_suffix);
        let new_path = &config_path; // already uses new suffix

        if old_path.exists() && !new_path.exists() {
            match std::fs::rename(&old_path, new_path) {
                Ok(_) => {
                    info!(
                        "Renamed config file from {} to {}",
                        old_path.display(),
                        new_path.display()
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to rename config file from {} to {}: {}",
                        old_path.display(),
                        new_path.display(),
                        e
                    );
                }
            }
        } else if old_path.exists() && new_path.exists() {
            info!(
                "Probably old config file detected ({}).",
                old_path.display()
            );
        }

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
            .replace("{IRFILETEMPLATE}", target_path.to_string_lossy().as_ref())
            .replace(
                "{DEVICENAMETEMPLATE}",
                &self.settings.borrow().virtual_device_name,
            )
            .replace("{VIRTUALNODENAME}", Self::VIRTUAL_NODE_SUFFIX);

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
    /// Looks for pattern: filename = "..." (with optional spaces)
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

    /// Runs `pw-cli list-objects` and parses its output into a vector of property maps.
    ///
    /// Each object is represented as a `HashMap<String, String>` where keys are property names
    /// (e.g., "id", "type", "media.class", "node.name") and values are the corresponding values
    /// (quotes stripped). The "id" and "type" fields are extracted from the object header line.
    ///
    /// Returns an error if `pw-cli` is not found, fails to execute, or the output cannot be parsed.
    pub fn list_audio_devices(&self) -> Result<Vec<HashMap<String, String>>> {
        let output = Command::new("pw-cli")
            .arg("list-objects")
            .output()
            .with_context(
                || "Failed to execute pw-cli command. Ensure pw-cli is installed and in PATH.",
            )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("pw-cli failed with status {}: {}", output.status, stderr);
        }

        let stdout =
            String::from_utf8(output.stdout).with_context(|| "pw-cli output is not valid UTF-8")?;

        Self::parse_pwcli_output(&stdout)
    }

    /// Parses the stdout of `pw-cli list-objects` into a vector of property maps.
    ///
    /// The expected format is:
    /// ```ignore
    /// id X, type Y
    ///     key1 = "value1"
    ///     key2 = value2
    /// ```
    /// Lines are trimmed; empty lines are ignored.
    fn parse_pwcli_output(output: &str) -> Result<Vec<HashMap<String, String>>> {
        let mut objects = Vec::new();
        let mut current_obj: Option<HashMap<String, String>> = None;

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Check if line starts with "id" and contains "type"
            if line.starts_with("id ") && line.contains("type ") {
                // If we have a previous object, push it
                if let Some(obj) = current_obj.take() {
                    objects.push(obj);
                }
                // Start a new object
                let mut obj = HashMap::new();

                // Parse id and type
                // Example: "id 0, type PipeWire:Interface:Core/4"
                let parts: Vec<&str> = line.splitn(2, ',').collect();
                if parts.len() >= 1 {
                    let id_part = parts[0].trim();
                    if let Some(id) = id_part.strip_prefix("id ") {
                        obj.insert("id".to_string(), id.trim().to_string());
                    }
                }
                if parts.len() >= 2 {
                    let type_part = parts[1].trim();
                    if let Some(type_val) = type_part.strip_prefix("type ") {
                        obj.insert("type".to_string(), type_val.trim().to_string());
                    }
                }
                current_obj = Some(obj);
            } else if let Some(ref mut obj) = current_obj {
                // Parse key = value line
                // Lines are indented with spaces/tabs; we already trimmed.
                // Split at first '=' (there may be spaces around it)
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().to_string();
                    let mut value = parts[1].trim().to_string();
                    // Strip surrounding double quotes if present
                    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                        value = value[1..value.len() - 1].to_string();
                    }
                    obj.insert(key, value);
                } else {
                    // If line doesn't contain '=', ignore (should not happen)
                    warn!("Could not parse line in pw-cli output: {line}");
                }
            }
        }

        // Push the last object
        if let Some(obj) = current_obj.take() {
            objects.push(obj);
        }

        Ok(objects)
    }

    /// Filters a list of audio device objects, returning only those that are audio sinks.
    ///
    /// An audio sink is defined as having a property `media.class` equal to AUDIO_DEVICE_TYPE. Skips
    /// IrateGoose virtual device.
    /// The returned vector contains clones of the matching entries.

    const AUDIO_DEVICE_CLASS: &str = "Audio/Sink";
    pub fn filter_audio_sinks(
        devices: &Vec<HashMap<String, String>>,
    ) -> Vec<HashMap<String, String>> {
        let irategoose_node = format!("effect_input.{}", Self::VIRTUAL_NODE_SUFFIX);
        devices
            .iter()
            .filter(|obj| match obj.get("media.class") {
                Some(v) => v == ConfigManager::AUDIO_DEVICE_CLASS,
                None => false,
            })
            .filter(|obj| obj.get("node.name").as_deref() != Some(&irategoose_node))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pwcli_output() {
        let input = r#"id 0, type PipeWire:Interface:Core/4
                object.serial = "0"
                core.name = "pipewire-0"
            id 36, type PipeWire:Interface:Node/3
                object.serial = "36"
                factory.id = "19"
                media.class = "Audio/Sink"
                node.name = "effect_input.virtual-surround-7.1-buttface"
            id 37, type PipeWire:Interface:Node/3
                object.serial = "37"
                media.class = "Stream/Output/Audio""#;

        let result = ConfigManager::parse_pwcli_output(input).unwrap();
        assert_eq!(result.len(), 3);

        let first = &result[0];
        assert_eq!(first.get("id"), Some(&"0".to_string()));
        assert_eq!(
            first.get("type"),
            Some(&"PipeWire:Interface:Core/4".to_string())
        );
        assert_eq!(first.get("object.serial"), Some(&"0".to_string()));
        assert_eq!(first.get("core.name"), Some(&"pipewire-0".to_string()));

        let second = &result[1];
        assert_eq!(second.get("media.class"), Some(&"Audio/Sink".to_string()));
        assert_eq!(
            second.get("node.name"),
            Some(&"effect_input.virtual-surround-7.1-buttface".to_string())
        );

        let third = &result[2];
        assert_eq!(
            third.get("media.class"),
            Some(&"Stream/Output/Audio".to_string())
        );
    }

    #[test]
    fn test_parse_without_quotes() {
        let input = r#"id 99, type Test
                key = value
                quoted = "value with spaces""#;
        let result = ConfigManager::parse_pwcli_output(input).unwrap();
        let obj = &result[0];
        assert_eq!(obj.get("key"), Some(&"value".to_string()));
        assert_eq!(obj.get("quoted"), Some(&"value with spaces".to_string()));
    }

    #[test]
    fn test_filter_audio_sinks() {
        let mut dev1 = HashMap::new();
        dev1.insert("id".to_string(), "36".to_string());
        dev1.insert("media.class".to_string(), "Audio/Sink".to_string());
        let mut dev2 = HashMap::new();
        dev2.insert("id".to_string(), "37".to_string());
        dev2.insert("media.class".to_string(), "Stream/Output/Audio".to_string());
        let mut dev3 = HashMap::new();
        dev3.insert("id".to_string(), "38".to_string());
        // no media.class

        let devices = vec![dev1, dev2, dev3];
        let filtered = ConfigManager::filter_audio_sinks(&devices);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].get("id"), Some(&"36".to_string()));
    }
}
