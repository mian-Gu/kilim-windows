//! User-level login shortcut. Called only by the user's explicit menu action.
use std::path::{Path, PathBuf};
use windows::{
    Win32::{
        Foundation::{E_FAIL, MAX_PATH},
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile},
        UI::Shell::*,
    },
    core::{Error, HSTRING, Interface, Result},
};
const DESCRIPTION: &str = "Kilim desktop rug";
pub fn shortcut_path() -> Result<PathBuf> {
    // SAFETY: known-folder path is a COM task allocation and freed after conversion.
    unsafe {
        let value = SHGetKnownFolderPath(&FOLDERID_Startup, KF_FLAG_DEFAULT, None)?;
        let text = value.to_string();
        CoTaskMemFree(Some(value.0.cast()));
        Ok(PathBuf::from(text?).join("Kilim.lnk"))
    }
}
pub fn enabled() -> Result<bool> {
    let path = shortcut_path()?;
    if !path.is_file() {
        return Ok(false);
    }
    let link = load(&path)?;
    is_owned(&link)
}
pub fn set_enabled(enabled: bool) -> Result<()> {
    let path = shortcut_path()?;
    if path.exists() {
        let link = load(&path)?;
        if !is_owned(&link)? {
            return Err(Error::new(
                E_FAIL,
                "An unrelated Kilim.lnk exists in Startup; it was not modified",
            ));
        }
    }
    if enabled {
        let executable = std::env::current_exe().map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        write_shortcut(&path, &executable)
    } else {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        }
        Ok(())
    }
}
fn load(path: &Path) -> Result<IShellLinkW> {
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let persist: IPersistFile = link.cast()?;
        persist.Load(
            &HSTRING::from(path.as_os_str()),
            windows::Win32::System::Com::STGM_READ,
        )?;
        Ok(link)
    }
}
fn is_owned(link: &IShellLinkW) -> Result<bool> {
    let mut description = [0u16; MAX_PATH as usize];
    unsafe {
        link.GetDescription(&mut description)?;
    }
    let length = description
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(description.len());
    Ok(String::from_utf16_lossy(&description[..length]) == DESCRIPTION)
}
fn write_shortcut(path: &Path, executable: &Path) -> Result<()> {
    // SAFETY: ShellLink is owned on the initialized UI STA. Save writes only the explicitly requested shortcut.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(executable.as_os_str()))?;
        link.SetDescription(&HSTRING::from(DESCRIPTION))?;
        if let Some(parent) = executable.parent() {
            link.SetWorkingDirectory(&HSTRING::from(parent.as_os_str()))?;
        }
        link.SetIconLocation(&HSTRING::from(executable.as_os_str()), 0)?;
        let persist: IPersistFile = link.cast()?;
        persist.Save(&HSTRING::from(path.as_os_str()), true)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_shortcut_can_be_encoded_without_touching_startup() {
        let _com = crate::image::ComScope::new().unwrap();
        let path = std::env::temp_dir().join(format!("kilim-startup-{}.lnk", std::process::id()));
        let executable = std::env::current_exe().unwrap();
        write_shortcut(&path, &executable).unwrap();
        assert!(is_owned(&load(&path).unwrap()).unwrap());
    }
}
