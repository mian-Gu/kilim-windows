//! OLE image-file drop receiver for an application-owned tray drop panel.
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};
use windows::{
    Win32::{
        Foundation::{HWND, POINTL},
        System::{
            Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL},
            Ole::*,
            SystemServices::MODIFIERKEYS_FLAGS,
        },
        UI::Shell::{DragQueryFileW, HDROP},
    },
    core::{Ref, Result, implement},
};
pub enum DropEvent {
    Image(PathBuf),
    Leave,
}
#[implement(IDropTarget)]
struct Target {
    sender: mpsc::Sender<DropEvent>,
    accepts: AtomicBool,
}
unsafe fn effect_for(effect: *mut DROPEFFECT, accepts: bool) -> bool {
    // OLE supplies the source's allowed-effect mask. Importing never moves the source file.
    unsafe {
        if effect.is_null() {
            return false;
        }
        let copy = accepts && (*effect).0 & DROPEFFECT_COPY.0 != 0;
        *effect = if copy {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_NONE
        };
        copy
    }
}
impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        let accepts = data
            .as_ref()
            .is_some_and(|data| first_image(data).is_some());
        self.accepts.store(accepts, Ordering::Relaxed);
        unsafe {
            effect_for(effect, accepts);
        }
        Ok(())
    }
    fn DragOver(
        &self,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        unsafe {
            effect_for(effect, self.accepts.load(Ordering::Relaxed));
        }
        Ok(())
    }
    fn DragLeave(&self) -> Result<()> {
        self.accepts.store(false, Ordering::Relaxed);
        let _ = self.sender.send(DropEvent::Leave);
        Ok(())
    }
    fn Drop(
        &self,
        data: Ref<IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        let path = data.as_ref().and_then(first_image);
        let accepted = unsafe { effect_for(effect, path.is_some()) };
        if accepted {
            if let Some(path) = path {
                let _ = self.sender.send(DropEvent::Image(path));
            }
        } else {
            let _ = self.sender.send(DropEvent::Leave);
        }
        self.accepts.store(false, Ordering::Relaxed);
        Ok(())
    }
}
fn image_file(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "jpe"
            | "jfif"
            | "bmp"
            | "dib"
            | "rle"
            | "gif"
            | "tif"
            | "tiff"
            | "ico"
            | "wdp"
            | "hdp"
            | "jxr"
            | "webp"
            | "heic"
            | "heif"
            | "avif"
            | "svg"
            | "dng"
            | "cr2"
            | "nef"
            | "arw"
            | "orf"
            | "raf"
    ) && path.is_file()
}
fn first_image(data: &IDataObject) -> Option<PathBuf> {
    // SAFETY: CF_HDROP is an owned HGLOBAL medium. Copy a path before releasing the medium;
    // the first image in a multi-file drag is imported, matching the upstream URL filtering.
    unsafe {
        if data.QueryGetData(&format()).is_err() {
            return None;
        }
        let mut medium = data.GetData(&format()).ok()?;
        let mut result = None;
        if medium.tymed == TYMED_HGLOBAL.0 as u32 {
            let handle = HDROP(medium.u.hGlobal.0);
            for index in 0..DragQueryFileW(handle, u32::MAX, None) {
                let length = DragQueryFileW(handle, index, None);
                let mut text = vec![0u16; length as usize + 1];
                if DragQueryFileW(handle, index, Some(&mut text)) > 0 {
                    text.truncate(length as usize);
                    let path = PathBuf::from(String::from_utf16_lossy(&text));
                    if image_file(&path) {
                        result = Some(path);
                        break;
                    }
                }
            }
        }
        ReleaseStgMedium(&mut medium);
        result
    }
}
fn format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    }
}
pub struct Receiver {
    window: HWND,
    _target: IDropTarget,
    pub events: mpsc::Receiver<DropEvent>,
}
impl Receiver {
    pub fn register(window: HWND) -> Result<Self> {
        let (sender, events) = mpsc::channel();
        let target: IDropTarget = Target {
            sender,
            accepts: AtomicBool::new(false),
        }
        .into();
        unsafe {
            RegisterDragDrop(window, &target)?;
        }
        Ok(Self {
            window,
            _target: target,
            events,
        })
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        unsafe {
            let _ = RevokeDragDrop(self.window);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        Win32::{
            System::Com::CoTaskMemFree,
            UI::Shell::{
                BHID_DataObject, Common::ITEMIDLIST, SHCreateShellItemArrayFromIDLists,
                SHParseDisplayName,
            },
        },
        core::HSTRING,
    };
    struct Pidl(*mut ITEMIDLIST);
    impl Drop for Pidl {
        fn drop(&mut self) {
            unsafe {
                CoTaskMemFree(Some(self.0.cast()));
            }
        }
    }
    fn data_object(paths: &[PathBuf]) -> IDataObject {
        let owned: Vec<_> = paths
            .iter()
            .map(|path| {
                let mut pointer = std::ptr::null_mut();
                let normalized: PathBuf = path.components().collect();
                unsafe {
                    SHParseDisplayName(
                        &HSTRING::from(normalized.as_os_str()),
                        None,
                        &mut pointer,
                        0,
                        None,
                    )
                    .unwrap_or_else(|error| {
                        panic!("Shell could not parse {}: {error}", path.display())
                    });
                }
                Pidl(pointer)
            })
            .collect();
        let pointers: Vec<_> = owned
            .iter()
            .map(|pidl| pidl.0 as *const ITEMIDLIST)
            .collect();
        unsafe {
            SHCreateShellItemArrayFromIDLists(&pointers)
                .unwrap()
                .BindToHandler(None, &BHID_DataObject)
                .unwrap()
        }
    }
    #[test]
    fn shell_drop_filters_nonimages_and_honors_source_effects() {
        let _com = crate::image::ComScope::new().unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let bad = data_object(&[root.join("Cargo.toml"), root.join("src")]);
        assert!(first_image(&bad).is_none());
        let data = data_object(&[
            root.join("assets/test/README.txt"),
            root.join("assets/test/srgb-profile.jpg"),
        ]);
        assert_eq!(
            first_image(&data).unwrap().file_name().unwrap(),
            "srgb-profile.jpg"
        );
        let (sender, events) = mpsc::channel();
        let target: IDropTarget = Target {
            sender,
            accepts: AtomicBool::new(false),
        }
        .into();
        let mut effect = DROPEFFECT_MOVE;
        unsafe {
            target
                .DragEnter(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)
                .unwrap();
        }
        assert_eq!(effect, DROPEFFECT_NONE);
        effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
        unsafe {
            target
                .DragEnter(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)
                .unwrap();
            target
                .Drop(&data, MODIFIERKEYS_FLAGS(0), POINTL::default(), &mut effect)
                .unwrap();
        }
        assert_eq!(effect, DROPEFFECT_COPY);
        assert!(matches!(events.try_recv(), Ok(DropEvent::Image(_))));
    }
}
