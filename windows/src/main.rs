#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod cloth;
mod collision;
mod desktop;
mod diagnostics;
mod drop_target;
mod image;
mod instance;
mod lighting;
mod localization;
mod logging;
mod random;
mod renderer;
mod settings;
mod startup;
mod texture;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("Kilim: {error}");
        // SAFETY: both strings remain alive throughout the synchronous dialog call.
        if !std::env::args().any(|arg| arg.starts_with("--") && arg != "--preview") {
            unsafe {
                let message = windows::core::HSTRING::from(error.to_string());
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    None,
                    &message,
                    windows::core::w!("Kilim Windows"),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                );
            }
        }
        std::process::exit(1);
    }
}
