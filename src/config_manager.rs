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

    /// The last part of the config path
    const CONFIG_SUFFIX: &'static str = "/tmp/surround.conf";
    // const CONFIG_SUFFIX: &'static str = "pipewire/pipewire.conf.d/sink-virtual-surround-7.1-hesuvi.conf";

    /// Creates a new ConfigManager instance
    pub fn new() -> Result<ConfigManager> {
        // Determine the full path to the current user's ~/.config folder
        let config_dir = dirs::config_dir().ok_or(anyhow!("Could not determine home directory"))?;
        
        // Append the config suffix to get the full absolute path
        let config_path = config_dir.join(Self::CONFIG_SUFFIX);
        
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
    
    /// Checks if the config file exists
    pub fn config_exists(&self) -> bool {
        self.config_path.exists()
    }

    /// Restarts the PipeWire services to apply configuration changes.
    fn apply_config(&self) -> Result<()> {
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