//! UserDefaults-equivalent preferences. No registry writes or global settings.
use crate::lighting::LightMode;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeekKey {
    Alt,
    Shift,
    Control,
    F8,
}
impl PeekKey {
    pub const ALL: [Self; 4] = [Self::Alt, Self::Shift, Self::Control, Self::F8];
    pub fn key(self) -> &'static str {
        match self {
            Self::Alt => "alt",
            Self::Shift => "shift",
            Self::Control => "control",
            Self::F8 => "f8",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Alt => "Alt",
            Self::Shift => "Shift",
            Self::Control => "Ctrl",
            Self::F8 => "F8",
        }
    }
    pub fn virtual_key(self) -> i32 {
        match self {
            Self::Alt => 0x12,
            Self::Shift => 0x10,
            Self::Control => 0x11,
            Self::F8 => 0x77,
        }
    }
    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.key() == value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RugSize {
    Small,
    Medium,
    Large,
    Huge,
}
impl RugSize {
    pub const ALL: [Self; 4] = [Self::Small, Self::Medium, Self::Large, Self::Huge];
    pub fn key(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Huge => "huge",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "S",
            Self::Medium => "M",
            Self::Large => "L",
            Self::Huge => "XL",
        }
    }
    pub fn factor(self) -> f32 {
        match self {
            Self::Small => 0.7,
            Self::Medium => 1.,
            Self::Large => 1.45,
            Self::Huge => 2.,
        }
    }
    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.key() == value)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub theme: String,
    pub size: RugSize,
    pub light: LightMode,
    pub sound: bool,
    pub language: String,
    pub peek: PeekKey,
    /// Whole desktop covered by one sheet, printed with the currently selected pattern.
    pub fullscreen: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "kashan".into(),
            size: RugSize::Medium,
            light: LightMode::Auto,
            sound: true,
            language: String::new(),
            peek: PeekKey::F8,
            fullscreen: false,
        }
    }
}
impl Settings {
    pub fn read(path: &Path) -> io::Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let values: BTreeMap<_, _> = text
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect();
        let mut result = Self::default();
        if let Some(theme) = values.get("theme")
            && theme.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !theme.is_empty() {
                result.theme = (*theme).into();
            }
        if let Some(value) = values.get("size").and_then(|v| RugSize::parse(v)) {
            result.size = value;
        }
        if let Some(value) = values.get("light").and_then(|v| LightMode::parse(v)) {
            result.light = value;
        }
        if let Some(value) = values.get("peek").and_then(|v| PeekKey::parse(v)) {
            result.peek = value;
        }
        if let Some(value) = values.get("sound") {
            result.sound = *value != "false";
        }
        if let Some(value) = values.get("fullscreen") {
            result.fullscreen = *value == "true";
        }
        if let Some(value) = values.get("language")
            && value.chars().all(|c| c.is_ascii_lowercase()) && value.len() <= 3 {
                result.language = (*value).into();
            }
        Ok(result)
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = format!(
            "theme={}\nsize={}\nlight={}\nsound={}\nlanguage={}\npeek={}\nfullscreen={}\n",
            self.theme,
            self.size.key(),
            self.light.key(),
            self.sound,
            self.language,
            self.peek.key(),
            self.fullscreen
        );
        let staging = path.with_extension("pending");
        std::fs::write(&staging, text)?;
        // MoveFileExW replaces only this application's preference file atomically.
        let from = windows::core::HSTRING::from(staging.as_os_str());
        let to = windows::core::HSTRING::from(path.as_os_str());
        unsafe {
            windows::Win32::Storage::FileSystem::MoveFileExW(
                &from,
                &to,
                windows::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(io::Error::other)
    }
}
pub fn data_directory() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Kilim")
}
pub fn directory_argument(args: &[String]) -> io::Result<Option<PathBuf>> {
    if let Some(index) = args.iter().position(|arg| arg == "--config-dir") {
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--config-dir requires a directory",
                )
            })?;
        Ok(Some(PathBuf::from(value)))
    } else {
        Ok(None)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_directory_cannot_silently_fall_back_to_user_data() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>()
        };
        assert!(directory_argument(&args(&["Kilim", "--config-dir"])).is_err());
        assert!(directory_argument(&args(&["Kilim", "--config-dir", "--preview"])).is_err());
        assert_eq!(
            directory_argument(&args(&["Kilim", "--config-dir", "isolated"])).unwrap(),
            Some(PathBuf::from("isolated"))
        );
    }
    #[test]
    fn preferences_round_trip_and_ignore_invalid_values() {
        let directory =
            std::env::temp_dir().join(format!("kilim-settings-test-{}", std::process::id()));
        let path = directory.join("settings.ini");
        let settings = Settings {
            theme: "selcuklu".into(),
            size: RugSize::Huge,
            light: LightMode::Evening,
            sound: false,
            language: "zh".into(),
            peek: PeekKey::Control,
            fullscreen: true,
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::read(&path).unwrap(), settings);
        std::fs::write(
            &path,
            "size=oops\nlight=bad\npeek=unknown\nlanguage=../../secret\ntheme=../../secret\n",
        )
        .unwrap();
        assert_eq!(Settings::read(&path).unwrap(), Settings::default());
    }
}
