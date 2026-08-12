#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod unpack;
mod packer;
mod app;

fn main() -> Result<(), eframe::Error> {
    app::run()
}
