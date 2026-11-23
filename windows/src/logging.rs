//! Opt-in local diagnostic logging, including slow dragging frames.
use std::{io::Write, path::PathBuf, sync::OnceLock};
static PATH: OnceLock<PathBuf> = OnceLock::new();
pub fn enabled() -> bool {
    std::env::var_os("KILIM_LOG").is_some() || std::env::temp_dir().join("kilim.debug").is_file()
}
fn path() -> &'static PathBuf {
    PATH.get_or_init(|| std::env::temp_dir().join("kilim.log"))
}
pub fn write(message: impl AsRef<str>) {
    if enabled()
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path())
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let _ = writeln!(file, "{now:.3} {}", message.as_ref());
        }
}
