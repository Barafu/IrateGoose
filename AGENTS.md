# IrateGoose - Agent Documentation

## Project Overview

- Application written in Rust.
- Uses eframe/egui v0.33.3 for building GUI. 
- Platform: Linux only. 

## Tools to use

- `cargo check` to verify new code for errors
- `cargo test` to run tests
- `cargo add` to add new dependency
- Never run `cargo build --release`
- Never run `cargo run --release`

## Testing

 - Do not create new tests unless the user explicitly instructs you to do it. 

## Rules
 
 - Ignore ./data folder.
 - If a file from data folder is needed, assume that it exists and has proper format.
 - Ignore warnings about unused variables,functions and structs unless explicitly instructed to fix them. 


