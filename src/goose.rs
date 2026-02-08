//! Contains functions that provide integration of the app into the system
use anyhow::{Context, Result, anyhow};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Icon bytes embedded at compile time
const ICON_BYTES: &[u8] = include_bytes!("../data/IrateGoose256.png");
/// Desktop file template embedded at compile time
const DESKTOP_TEMPLATE: &str = include_str!("../data/barafu-irategoose.desktop.template");

/// Determine the executable path to use in the .desktop file.
/// Returns either the binary name if the binary is in PATH and matches current_exe,
/// otherwise the absolute path of current_exe.
fn determine_executable() -> Result<String> {
    let current_exe = env::current_exe()
        .context("Failed to get current executable path")?
        .canonicalize()
        .context("Failed to canonicalize current executable path")?;

    // Extract binary name from current_exe
    let binary_name = current_exe
        .file_name()
        .ok_or_else(|| anyhow!("Failed to get filename from current executable path"))?
        .to_string_lossy()
        .into_owned();

    // Check if binary_name is in PATH and points to the same file
    let type_command = format!("type -p {}", &binary_name);
    let which_output = Command::new("sh")
        .args(["-c", &type_command])
        .output()
        .with_context(|| format!("Failed to run 'type -p {}'", binary_name))?;

    if which_output.status.success() {
        let path_str = String::from_utf8_lossy(&which_output.stdout).trim().to_string();
        if let Ok(which_path) = PathBuf::from(&path_str).canonicalize()
            && which_path == current_exe {
                return Ok(binary_name);
            }
    }

    // Fallback to absolute path
    Ok(current_exe.to_string_lossy().into_owned())
}

/// Create a temporary file with the given content and filename.
/// Returns the path to the created file.
fn create_temp_file(content: &[u8], filename: &str) -> Result<PathBuf> {
    let temp_dir = env::temp_dir();
    let file_path = temp_dir.join(filename);
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write temporary file {}", file_path.display()))?;
    Ok(file_path)
}

/// Install the application to the system menu according to XDG desktop specifications.
pub fn install_goose() -> Result<()> {
    log::info!("Installing application to system menu");

    // Determine executable path
    let exec = determine_executable()?;
    log::debug!("Using executable: {}", exec);

    // Fill desktop template
    let desktop_content = DESKTOP_TEMPLATE.replace("{EXEC}", &exec);
    log::debug!("Desktop content:\n{}", desktop_content);

    // Create temporary .desktop file with exact name "barafu-irategoose.desktop"
    let desktop_temp = create_temp_file(desktop_content.as_bytes(), "barafu-irategoose.desktop")?;
    log::debug!("Created temporary desktop file: {}", desktop_temp.display());

    // Install desktop entry via xdg-desktop-menu
    let status = Command::new("xdg-desktop-menu")
        .arg("install")
        .arg(&desktop_temp)
        .status()
        .context("Failed to execute xdg-desktop-menu")?;

    if !status.success() {
        return Err(anyhow!("xdg-desktop-menu failed with exit code {:?}", status.code()));
    }

    // Create temporary icon file with exact name "barafu-irategoose.png"
    let icon_temp = create_temp_file(ICON_BYTES, "barafu-irategoose.png")?;
    log::debug!("Created temporary icon file: {}", icon_temp.display());

    // Install icon via xdg-icon-resource
    let status = Command::new("xdg-icon-resource")
        .args(["install", "--size", "256", "--context", "apps"])
        .arg(&icon_temp)
        .arg("barafu-irategoose")
        .status()
        .context("Failed to execute xdg-icon-resource")?;

    if !status.success() {
        return Err(anyhow!("xdg-icon-resource failed with exit code {:?}", status.code()));
    }

    log::info!("Installation completed successfully");
    println!("Installation completed successfully");
    Ok(())
}

/// Remove the application from the system menu.
pub fn uninstall_goose() -> Result<()> {
    log::info!("Removing application from system menu");

    // Determine executable path (same as install) to generate identical desktop content
    let exec = determine_executable()?;
    let desktop_content = DESKTOP_TEMPLATE.replace("{EXEC}", &exec);
    let desktop_temp = create_temp_file(desktop_content.as_bytes(), "barafu-irategoose.desktop")?;

    // Uninstall desktop entry
    let status = Command::new("xdg-desktop-menu")
        .arg("uninstall")
        .arg(&desktop_temp)
        .status()
        .context("Failed to execute xdg-desktop-menu uninstall")?;

    if !status.success() {
        return Err(anyhow!("xdg-desktop-menu uninstall failed with exit code {:?}", status.code()));
    }

    // Uninstall icon
    let status = Command::new("xdg-icon-resource")
        .args(["uninstall", "--size", "256", "--context", "apps"])
        .arg("barafu-irategoose")
        .status()
        .context("Failed to execute xdg-icon-resource uninstall")?;

    if !status.success() {
        return Err(anyhow!("xdg-icon-resource uninstall failed with exit code {:?}", status.code()));
    }

    log::info!("Uninstallation completed successfully");
    println!("Uninstallation completed successfully");
    Ok(())
}