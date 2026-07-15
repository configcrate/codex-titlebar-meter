#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("Codex Titlebar Meter currently supports Windows only.");

mod codex;
mod locale;
mod model;
mod native;
mod settings;

use anyhow::Result;

fn main() -> Result<()> {
    native::run()
}
