# IrateGoose 

## Project Overview

- Application written in Rust.
- Uses eframe/egui for building GUI. For versions of crates check Cargo.lock
- Platform: Linux only. 

## Terminology

- **Settings**: the application settings managed by `src/settings.rs` (`AppSettings`), persisted in the application directory (e.g. `~/.config/irate_goose/settings.toml`). Never use "settings" to refer to PipeWire files.
- **Config**: the PipeWire configuration required to create the virtual sound card, managed by `ConfigManager`. Never use "config" to refer to the application settings.

These are two separate entities; keep the distinction in code, comments, logs, and conversation.

## Tools to use

- `cargo check` without extra arguments to verify the new code for errors
- `cargo test` to run tests
- `cargo add` to add new dependency
- Never run `cargo build --release`

## Testing

 - Do not create new tests unless the user explicitly instructs you to do it. 
 - Run 'cargo check' after making changes in code. 
 - Trust the output of `cargo check` unless it returns an explicit error.  

## Rules
 
 - When you need to search docs, use `context7` tools.
 - Always write doc comments (`///`) for every struct and function. Use regular comments (`//`) to explain large or non-obvious blocks of code.
