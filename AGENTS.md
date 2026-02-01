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
- Never run `cargo run`

## Testing

 - Do not create new tests unless the user explicitly instructs you to do it. 
 - Use just `cargo test` to run tests, do not try to run specific tests only. 

## Rules
 
 - Ignore ./data directory.
 - If a file from data directory is needed, assume that it exists and has proper format.
 - Ignore warnings about unused variables,functions and structs unless explicitly instructed to fix them. 

 ## Notes
 - Function trim() of a String object returns a referense to the string, not a separate value. Use .trim().to_string() to get a separate value().


