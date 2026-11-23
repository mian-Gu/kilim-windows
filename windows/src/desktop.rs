//! Explorer discovery and read-only icon scans. COM scans run on a separate MTA thread.
use crate::{image::ComScope, renderer::Rect};
use std::{sync::mpsc, thread};
use windows::{
    Win32::{
        Foundation::{E_FAIL, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::*,
        System::{
            Com::{CLSCTX_LOCAL_SERVER, CoCreateInstance, CoTaskMemFree, IServiceProvider},
            Variant::VARIANT,
        },
        UI::{
            Shell::{
                FOLDERVIEWMODE, IFolderView, IFolderView2, IShellBrowser, IShellWindows,
                SID_STopLevelBrowser, SVGIO_ALLVIEW, SWC_DESKTOP, SWFO_NEEDDISPATCH, ShellWindows,
            },
            WindowsAndMessaging::*,
        },
    },
    core::{BOOL, Error, Interface, Result, w},
};
#[derive(Clone, Copy, Debug)]
pub struct Screen {
    pub rect: Rect,
    pub primary: bool,
    pub scale: f32,
}
pub struct Desktop {
    pub host: HWND,
    pub list: HWND,
    pub origin: POINT,
    pub size: [f32; 2],
    pub screens: Vec<Screen>,
}
impl Desktop {
    pub fn connect() -> Result<Self> {
        // SAFETY: both enumerations synchronously write to live Vecs; all coordinates are physical
        // pixels under the application's per-monitor-V2 manifest. No Shell windows are changed.
        unsafe {
            let mut windows: Vec<HWND> = Vec::new();
            EnumWindows(
                Some(collect_window),
                LPARAM(&mut windows as *mut _ as isize),
            )?;
            for host in windows {
                let mut class_name = [0u16; 64];
                let length = GetClassNameW(host, &mut class_name) as usize;
                let class_name = String::from_utf16_lossy(&class_name[..length]);
                if !matches!(class_name.as_str(), "Progman" | "WorkerW") {
                    continue;
                }
                if let Ok(view) = FindWindowExW(Some(host), None, w!("SHELLDLL_DefView"), None) {
                    if !IsWindowVisible(host).as_bool() {
                        continue;
                    }
                    let list = FindWindowExW(Some(view), None, w!("SysListView32"), None)?;
                    let mut rect = RECT::default();
                    GetClientRect(host, &mut rect)?;
                    let mut origin = POINT::default();
                    if !ClientToScreen(host, &mut origin).as_bool() {
                        return Err(Error::from_thread());
                    }
                    let mut monitors: Vec<(RECT, bool, u32)> = Vec::new();
                    if !EnumDisplayMonitors(
                        None,
                        None,
                        Some(collect_monitor),
                        LPARAM(&mut monitors as *mut _ as isize),
                    )
                    .as_bool()
                    {
                        return Err(Error::from_thread());
                    }
                    let screens = monitors
                        .into_iter()
                        .map(|(r, primary, dpi)| Screen {
                            rect: Rect {
                                x: r.left - origin.x,
                                y: r.top - origin.y,
                                width: (r.right - r.left) as u32,
                                height: (r.bottom - r.top) as u32,
                            },
                            primary,
                            scale: dpi as f32 / 96.,
                        })
                        .collect();
                    return Ok(Self {
                        host,
                        list,
                        origin,
                        size: [
                            (rect.right - rect.left) as f32,
                            (rect.bottom - rect.top) as f32,
                        ],
                        screens,
                    });
                }
            }
        }
        Err(Error::new(E_FAIL, "Explorer desktop is unavailable"))
    }
    pub fn primary(&self) -> Screen {
        self.screens
            .iter()
            .copied()
            .find(|screen| screen.primary)
            .unwrap_or(Screen {
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: self.size[0] as u32,
                    height: self.size[1] as u32,
                },
                primary: true,
                scale: 1.,
            })
    }
    pub fn absolute_icon_positions(&self) -> Result<Vec<[f32; 2]>> {
        // SAFETY: caller owns a COM apartment. Item PIDLs are freed once, including failed position reads.
        unsafe {
            let folder = folder_view()?;
            let count = folder.ItemCount(SVGIO_ALLVIEW)?;
            let folder2: IFolderView2 = folder.cast()?;
            let mut mode = FOLDERVIEWMODE::default();
            let mut icon_size = 0;
            folder2.GetViewModeAndIconSize(&mut mode, &mut icon_size)?;
            let mut list_origin = POINT::default();
            if !ClientToScreen(self.list, &mut list_origin).as_bool() {
                return Err(Error::from_thread());
            }
            let mut result = Vec::with_capacity(count.max(0) as usize);
            for index in 0..count {
                let pidl = folder.Item(index)?;
                let position = folder.GetItemPosition(pidl);
                CoTaskMemFree(Some(pidl.cast()));
                let p = position?;
                result.push([
                    (p.x + list_origin.x + icon_size / 2) as f32,
                    (p.y + list_origin.y + icon_size / 2) as f32,
                ]);
            }
            Ok(result)
        }
    }
    pub fn icon_positions(&self) -> Result<Vec<[f32; 2]>> {
        Ok(self
            .absolute_icon_positions()?
            .into_iter()
            .map(|p| [p[0] - self.origin.x as f32, p[1] - self.origin.y as f32])
            .collect())
    }
}
unsafe extern "system" fn collect_window(hwnd: HWND, parameter: LPARAM) -> BOOL {
    unsafe {
        (&mut *(parameter.0 as *mut Vec<HWND>)).push(hwnd);
    }
    BOOL(1)
}
unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    parameter: LPARAM,
) -> BOOL {
    unsafe {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let (mut x, mut y) = (96, 96);
            let _ = windows::Win32::UI::HiDpi::GetDpiForMonitor(
                monitor,
                windows::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
                &mut x,
                &mut y,
            );
            (&mut *(parameter.0 as *mut Vec<(RECT, bool, u32)>)).push((
                info.rcMonitor,
                info.dwFlags & 1 != 0,
                x.max(96),
            ));
        }
    }
    BOOL(1)
}
fn folder_view() -> Result<IFolderView> {
    unsafe {
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
        let location = VARIANT::default();
        let mut handle = 0;
        let dispatch = windows.FindWindowSW(
            &location,
            &location,
            SWC_DESKTOP,
            &mut handle,
            SWFO_NEEDDISPATCH,
        )?;
        let service: IServiceProvider = dispatch.cast()?;
        let browser: IShellBrowser = service.QueryService(&SID_STopLevelBrowser)?;
        browser.QueryActiveShellView()?.cast()
    }
}

pub struct Scan {
    pub result: std::result::Result<Vec<[f32; 2]>, String>,
}
pub struct Scanner {
    sender: mpsc::SyncSender<bool>,
    receiver: mpsc::Receiver<Scan>,
    worker: Option<thread::JoinHandle<()>>,
    busy: bool,
}
impl Scanner {
    pub fn new() -> std::io::Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<bool>(1);
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kilim-desktop-scan".into())
            .spawn(move || {
                let apartment = ComScope::new();
                if let Err(error) = &apartment {
                    let _ = done_tx.send(Scan {
                        result: Err(error.to_string()),
                    });
                    return;
                }
                while let Ok(scan) = rx.recv() {
                    if !scan {
                        break;
                    }
                    let result = Desktop::connect()
                        .and_then(|desktop| desktop.absolute_icon_positions())
                        .map_err(|error| error.to_string());
                    if done_tx.send(Scan { result }).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: tx,
            receiver: done_rx,
            worker: Some(worker),
            busy: false,
        })
    }
    pub fn request(&mut self) {
        if !self.busy && self.sender.try_send(true).is_ok() {
            self.busy = true;
        }
    }
    pub fn poll(&mut self) -> Option<Scan> {
        match self.receiver.try_recv() {
            Ok(result) => {
                self.busy = false;
                Some(result)
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.busy = false;
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }
}
impl Drop for Scanner {
    fn drop(&mut self) {
        let _ = self.sender.try_send(false);
        self.worker.take(); /* Never wait for a hung Shell RPC on the UI thread. */
    }
}
