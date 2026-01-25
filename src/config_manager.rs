use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use anyhow::{Context, Result, anyhow};

pub struct ConfigManager {
    /// Full absolute path to the config file
    config_path: PathBuf,
}

impl ConfigManager {
    /// The config file template
    const CONFIG_TEMPLATE: &'static str = include_str!("../sink_template.conf");

    /// Creates a new ConfigManager instance
    pub fn new() -> Result<ConfigManager> {
        // Determine the full path to the current user's ~/.config folder
        let config_dir = dirs::config_dir().ok_or(anyhow!("Could not determine home directory"))?;
        
        // Determine config suffix based on DEV_MODE
        // Uses /tmp/surround.conf in dev mode for testing
        // Uses the real PipeWire config path in production mode
        let config_suffix = if crate::DEV_MODE.load(std::sync::atomic::Ordering::Relaxed) {
            "/tmp/surround.conf"
        } else {
            "pipewire/pipewire.conf.d/sink-virtual-surround-7.1-hesuvi.conf"
        };
        
        // Append the config suffix to get the full absolute path
        let config_path = config_dir.join(config_suffix);
        
        Ok(Self {
            config_path,
        })
    }
        
    /// Writes the updated configuration to the config path
    pub fn write_config(&self, wavefile_path: &Path) -> Result<()> {
        //Create text for config file
        let config_text = Self::CONFIG_TEMPLATE.replace("TEMPLATE", wavefile_path.to_string_lossy().as_ref());
        // Ensure the parent directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
        
        fs::write(&self.config_path, config_text)
            .with_context(|| format!("Failed to write config to {}", self.config_path.display()))?;
        
        // Restart services to apply the new config
        self.apply_config()?;
        
        Ok(())
    }
    
    /// Deletes the config file completely
    pub fn delete_config(&self) -> Result<()> {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path)
                .with_context(|| format!("Failed to delete config file {}", self.config_path.display()))?;
        }
        // Restart services to apply the removal
        self.apply_config()?;
        Ok(())
    }
    
    /// Checks if the config file exists and returns the wavefile path if found
    /// Returns Ok(Some(PathBuf)) if config exists and contains a valid filename
    /// Returns Ok(None) if config file does not exist
    /// Returns Err(String) if config exists but cannot be read or parsed
    pub fn config_exists(&self) -> Result<Option<PathBuf>, String> {
        if !self.config_path.exists() {
            return Ok(None);
        }
        
        // Read the config file
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        // Search for filename pattern in the config
        Self::extract_filename_from_config(&content)
            .map(Some)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }
    
    /// Extracts the filename from config content
    /// Looks for pattern: filename = "PATH" (with optional spaces)
    fn extract_filename_from_config(content: &str) -> Result<PathBuf, String> {
        // Search for filename = "..." pattern
        // The pattern could be: filename = "/path/to/file.wav"
        // or: filename = "/home/barafu/Scripts/Surround_WAV/HeSuVi/Common/cmss_ent-/cmss_ent-.wav"
        let re = regex::Regex::new(r#"filename\s*=\s*"([^"]+)"#)
            .map_err(|e| format!("Failed to compile regex: {}", e))?;
            
        if let Some(captures) = re.captures(content) {
            if let Some(filename_match) = captures.get(1) {
                let filename = filename_match.as_str();
                return Ok(PathBuf::from(filename));
            }
        }
        
        Err("No filename found in config".to_string())
    }

    /// Restarts the PipeWire services to apply configuration changes.
    /// Does nothing when in dev mode.
    fn apply_config(&self) -> Result<()> {
        // In dev mode, skip restarting services
        if crate::DEV_MODE.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        
        let output = Command::new("systemctl")
            .args(["--user", "restart", "wireplumber", "pipewire", "pipewire-pulse"])
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