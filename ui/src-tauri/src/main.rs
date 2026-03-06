// ============================================================
// FILE:        main.rs
// MODULE:      Layer 3 — Desktop UI Shell (Tauri backend)
// TASK:        T-028
// PLATFORM:    cross (Windows, macOS, Linux)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// DEPENDENCIES: tauri 2.0, interprocess 2.0
// ============================================================

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    focusme_tauri::run();
}
