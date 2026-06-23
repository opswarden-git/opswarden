// Prevents an extra console window on Windows in release; no-op elsewhere.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opswarden_desktop_lib::run()
}
