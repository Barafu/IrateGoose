# GUI for Surround Sound - Agent Documentation

## Project Overview

This is a Rust GUI application for configuring surround sound capabilities with Pipewire. The application provides a graphical interface to select and apply surround sound configurations using WAV files.

## Basic Information

- **Project Name**: GUI for Surround Sound
- **Language**: Rust
- **Framework**: eframe/egui for the GUI
- **Purpose**: Configure surround sound capabilities of Pipewire
- **Repository**: Located at `/home/barafu/Coding/Rust/GUI_for_surround_sound`

## Application Architecture

### Core Components

1. **Main Application** (`src/main.rs`)
   - Handles CLI argument parsing
   - Initializes the file manager
   - Launches the GUI application

2. **GUI Module** (`src/app_gui.rs`)
   - Implements the eframe/egui interface
   - Provides file selection UI
   - Handles user interactions

3. **File Manager** (`src/file_manager.rs`)
   - Scans directories recursively for WAV files
   - Stores relative file paths (stripping base directory prefix)
   - Sorts entries with HeSuVi/ entries prioritized, then alphabetically
   - Provides rescan functionality to refresh the file list
   - Uses WaveFileData struct to store file path information

4. **Config Manager** (`src/config_manager.rs`)
   - Manages Pipewire configuration files
   - Uses template from `sink_template.conf`
   - Creates configuration files in user's config directory
   - Handles writing, deleting, and checking configuration files
   - Replaces template placeholders with actual WAV file paths

### Key Features

- Graphical file browser for WAV files
- Selectable file list with scrollable interface
- Apply button for selected configurations
- Error handling for missing files/directories

## Platform Requirements

### Target Platform
**This application is intended to be used only on a modern Linux desktop.**

The application has the following platform-specific requirements:

1. **Operating System**: Linux (specifically modern desktop distributions)
2. **Audio System**: Pipewire audio server
3. **Dependencies**:
   - Rust toolchain (cargo, rustc)
   - eframe/egui dependencies
   - Standard Linux desktop libraries

### Compatibility Notes
- The application is designed for Linux Pipewire environments
- No support for Windows or macOS is planned
- Requires standard Linux desktop environment for GUI rendering
- May have dependencies on specific Linux audio APIs

## Template Texts and Compilation

### Text Compilation Strategy
**All template texts should be compiled into the binary itself.**

This approach provides several benefits:

1. **Performance**: No runtime file I/O for text resources
2. **Portability**: Single binary deployment
3. **Security**: Reduced attack surface from external files
4. **Reliability**: No missing resource file errors

### Implementation Details

Texts are compiled using Rust's `const` declarations:

```rust
// Example from main.rs
const NO_WAVEFILE_PATH: &str = "Could not determine path to wave files";
```

### Text Resources to Include

The following texts should be compiled into the binary:

1. **UI Labels and Headings**
   - Window titles
   - Button labels
   - Section headers
   - Status messages

2. **Error Messages**
   - File system errors
   - Audio configuration errors
   - User input validation errors

3. **Help and Documentation**
   - Tooltips
   - Status bar messages
   - Dialog box content

### Compilation Method

Use one of these approaches:

1. **String Constants**: `const TEXT: &str = "message";`
2. **Static Strings**: `static TEXT: &'static str = "message";`
3. **Include Macros**: `include_str!()` for larger text blocks if needed

## Development Guidelines

### Code Organization
- Keep UI texts as constants in relevant modules
- Group related texts together for maintainability
- Use descriptive constant names that indicate purpose

### Distribution
The resulting binary is self-contained with all necessary texts compiled in. No additional resource files are required for deployment.

### Testing
- Do not write tests unless explicitly instructed to do so
- To verify for code errors, run `cargo check`. Do not run `cargo build` afterwards only to check for errors. 

