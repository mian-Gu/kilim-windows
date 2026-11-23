//! A session-local kernel object provides normal launch reuse, like macOS LaunchServices.
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
        System::Threading::CreateMutexW,
    },
    core::{HSTRING, Result},
};
pub struct Instance(HANDLE);
impl Instance {
    pub fn acquire(name: &str) -> Result<Option<Self>> {
        // SAFETY: a session-local mutex is held by this RAII handle only. No system configuration is written.
        unsafe {
            let name = HSTRING::from(name);
            let handle = CreateMutexW(None, false, &name)?;
            let existing = GetLastError() == ERROR_ALREADY_EXISTS;
            if existing {
                CloseHandle(handle)?;
                Ok(None)
            } else {
                Ok(Some(Self(handle)))
            }
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_second_launch_reuses_the_session_instance() {
        let name = format!("Local\\Kilim.InstanceTest.{}", std::process::id());
        let first = Instance::acquire(&name).unwrap().unwrap();
        assert!(Instance::acquire(&name).unwrap().is_none());
        drop(first);
        assert!(Instance::acquire(&name).unwrap().is_some());
    }
}
