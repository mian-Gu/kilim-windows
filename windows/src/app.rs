use crate::{
    audio::{RugSound, ThumpDetector},
    cloth::Cloth,
    desktop::{Desktop, Scanner, Screen},
    drop_target::{DropEvent, Receiver as DropReceiver},
    lighting::{LightEnv, LightMode},
    localization::{L10n, Text},
    logging,
    renderer::{Rect, Renderer},
    settings::{self, PeekKey, RugSize, Settings},
    startup, texture,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    mem::size_of,
    path::PathBuf,
    time::{Duration, Instant},
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::{
    Win32::{
        Foundation::{E_FAIL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::*,
        System::{
            Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree},
            LibraryLoader::GetModuleHandleW,
            Ole::{OleInitialize, OleUninitialize},
        },
        UI::{
            Input::{KeyboardAndMouse::*, *},
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
    core::{Error, HSTRING, Result, w},
};
const TRAY_MESSAGE: u32 = WM_APP + 1;
const TIMER_ID: usize = 1;
const SHOW: usize = 1;
const RESET: usize = 2;
const RESCAN: usize = 3;
const SOUND: usize = 4;
const CUSTOM: usize = 5;
const LOGIN: usize = 6;
const SPONSOR: usize = 7;
const FULLSCREEN: usize = 8;
const EXIT: usize = 9;
const RELEASE_FOLD_FADE: f32 = 0.35;
const FOLD_LIGHT_RECOVERY: f32 = 0.12;
const RELEASE_FOLD_CONTRAST: f32 = 0.30;
/// Submenu entries use this id to request a separator instead of a command item.
const SEPARATOR: usize = 0;
#[derive(Clone, Copy)]
enum Event {
    Tick,
    Down(bool, HWND, [i32; 2]),
    Up,
    Menu,
    Command(usize),
    Display,
    Mouse(u32, [i32; 2]),
    Hover([i32; 2]),
}
thread_local! {
    static EVENTS:RefCell<VecDeque<Event>>=const{RefCell::new(VecDeque::new())};
    static TICK_QUEUED:Cell<bool>=const{Cell::new(false)};
}
fn enqueue(event: Event) {
    if matches!(event, Event::Tick) && TICK_QUEUED.with(|pending| pending.replace(true)) {
        return;
    }
    EVENTS.with(|events| {
        let mut events = events.borrow_mut();
        if let Event::Mouse(WM_MOUSEMOVE, _) = event
            && matches!(events.back(), Some(Event::Mouse(WM_MOUSEMOVE, _))) {
                events.pop_back();
            }
        if matches!(event, Event::Hover(_)) && matches!(events.back(), Some(Event::Hover(_))) {
            events.pop_back();
        }
        events.push_back(event);
    });
}
struct Apartment;
impl Apartment {
    fn new() -> Result<Self> {
        unsafe {
            OleInitialize(None)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            OleUninitialize();
        }
    }
}
struct Overlay {
    window: HWND,
    screen: Rect,
}
struct DragBump {
    index: usize,
    offset: [f32; 2],
    start: [f32; 2],
    moved: bool,
}
struct App {
    control: HWND,
    overlays: Vec<Overlay>,
    desktop: Desktop,
    renderer: Renderer,
    cloth: Cloth,
    preferences: Settings,
    language: L10n,
    custom_available: bool,
    settings_path: PathBuf,
    textures: texture::Worker,
    texture_sequence: u64,
    sound: RugSound,
    thumps: ThumpDetector,
    scanner: Scanner,
    hidden: bool,
    peek: bool,
    peek_alpha: f32,
    environment: LightEnv,
    last_light: Instant,
    last_tick: Instant,
    accumulated: Duration,
    last_scan: Instant,
    scan_after: Option<Instant>,
    drag_bump: Option<DragBump>,
    left_down: bool,
    last_pointer: [f32; 2],
    render_dirty: bool,
    grab_window: Option<HWND>,
    last_grab_end: Instant,
    returning_from_grab: bool,
    soften_released_fold: bool,
    fold_contrast: f32,
    smooth_slide: f32,
    active_before: bool,
    running: bool,
    taskbar_message: u32,
    timer_interval: u32,
    drop_window: HWND,
    drop_receiver: Option<DropReceiver>,
    drop_shown: Option<Instant>,
    icon_error: Option<String>,
    texture_error: Option<String>,
    action_error: Option<String>,
    input_unavailable: bool,
    reconnect_since: Option<Instant>,
    last_reconnect: Instant,
    preview: bool,
    last_draw: Instant,
    created: Instant,
    last_presented: u64,
}

pub fn run() -> Result<()> {
    let _apartment = Apartment::new()?;
    let args: Vec<String> = std::env::args().collect();
    if crate::diagnostics::run_if_requested(&args)? {
        return Ok(());
    }
    let preview = args.iter().any(|arg| arg == "--preview");
    let stop_after = if let Some(index) = args.iter().position(|arg| arg == "--run-for-seconds") {
        let seconds = args
            .get(index + 1)
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|seconds| (1..=3600).contains(seconds))
            .ok_or_else(|| Error::new(E_FAIL, "--run-for-seconds requires 1..3600"))?;
        Some(Duration::from_secs(seconds))
    } else {
        None
    };
    let isolated = settings::directory_argument(&args).map_err(io_error)?;
    let smoke = args.iter().any(|arg| arg == "--smoke-test");
    let _instance = if !preview && !smoke {
        let Some(instance) = crate::instance::Instance::acquire("Local\\Kilim.Windows")? else {
            return Ok(());
        };
        if unsafe { FindWindowW(w!("KilimRustWindow"), w!("Kilim Windows Controller")) }.is_ok() {
            unsafe {
                MessageBoxW(
                    None,
                    w!(
                        "已有 Kilim 版本正在运行。请先从旧版托盘菜单退出，再启动此版本。\nA Kilim version is already running. Quit it from its tray menu, then launch this version."
                    ),
                    w!("Kilim"),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            return Ok(());
        }
        Some(instance)
    } else {
        None
    };
    let directory = isolated.or_else(|| {
        (preview || smoke)
            .then(|| std::env::temp_dir().join(format!("kilim-diagnostic-{}", std::process::id())))
    });
    let mut app = App::new(preview, directory, smoke)?;
    if smoke {
        return app.smoke_test();
    }
    let started = Instant::now();
    // SAFETY: this thread owns the windows, OLE apartment and all graphics resources.
    unsafe {
        let mut message = MSG::default();
        while app.running {
            let result = GetMessageW(&mut message, None, 0, 0);
            if result.0 == -1 {
                return Err(Error::from_thread());
            }
            if result.0 == 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
            loop {
                let event = EVENTS.with(|events| events.borrow_mut().pop_front());
                let Some(event) = event else {
                    break;
                };
                if matches!(event, Event::Tick) {
                    TICK_QUEUED.with(|pending| pending.set(false));
                }
                app.handle(event)?;
            }
            if message.message == app.taskbar_message {
                app.schedule_reconnect();
            }
            // WM_TIMER has lower priority than raw input. A high-polling-rate mouse must not
            // starve a due frame, while each mouse move still avoids running the cloth solver.
            if message.message != WM_TIMER
                && app.last_draw.elapsed() >= Duration::from_millis(app.timer_interval.into())
            {
                app.tick()?;
            }
            if stop_after.is_some_and(|duration| started.elapsed() >= duration) {
                break;
            }
        }
    }
    Ok(())
}
impl App {
    fn new(preview: bool, directory: Option<PathBuf>, silent: bool) -> Result<Self> {
        let data_directory = directory.unwrap_or_else(settings::data_directory);
        let settings_path = data_directory.join("settings.ini");
        let (mut preferences, preference_error) = match Settings::read(&settings_path) {
            Ok(preferences) => (preferences, None),
            Err(error) => (Settings::default(), Some(error.to_string())),
        };
        if preview {
            preferences.sound = false;
        }
        if !texture::PRESETS
            .iter()
            .any(|preset| preset.key == preferences.theme)
            && preferences.theme != "custom"
        {
            preferences.theme = "kashan".into();
        }
        let language = L10n::new(&preferences.language);
        let photo = texture::saved_photo(&data_directory);
        let custom_available = photo.is_some();
        let image = if preferences.theme == "custom" {
            photo
                .as_ref()
                .map(texture::custom)
                .unwrap_or_else(texture::placeholder)
        } else {
            texture::generate(&preferences.theme, &data_directory)?
        };
        // SAFETY: callbacks enqueue events, rather than reentering or retaining raw App pointers.
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                style: CS_DBLCLKS,
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hIcon: LoadIconW(
                    Some(instance.into()),
                    resource_id(1),
                )
                .or_else(|_| LoadIconW(None, IDI_APPLICATION))?,
                lpszClassName: w!("KilimRustWindow"),
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(Error::from_thread());
            }
            let control = CreateWindowExW(
                if preview {
                    WINDOW_EX_STYLE::default()
                } else {
                    WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
                },
                class.lpszClassName,
                if preview {
                    w!("Kilim Windows Preview")
                } else {
                    w!("Kilim Windows Controller")
                },
                if preview {
                    WS_OVERLAPPEDWINDOW
                } else {
                    WS_POPUP
                },
                80,
                80,
                1100,
                760,
                None,
                None,
                Some(instance.into()),
                None,
            )?;
            let desktop = if preview {
                preview_desktop(control)?
            } else {
                Desktop::connect()?
            };
            let renderer = Renderer::with_texture(desktop.size, &image)?;
            let mut cloth = Cloth::new(desktop.size);
            cloth.set_point_scale(desktop.primary().scale);
            place(&mut cloth, &desktop, &preferences);
            if preview {
                cloth.bumps = vec![[500., 350.], [520., 370.], [540., 345.]];
            }
            let drop_title = HSTRING::from(language.text(Text::CustomRug));
            let drop_window = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                &drop_title,
                WS_POPUP | WS_BORDER | WINDOW_STYLE(0x201),
                0,
                0,
                240,
                90,
                Some(control),
                None,
                Some(instance.into()),
                None,
            )?;
            let drop_receiver = DropReceiver::register(drop_window)?;
            // Asynchronous observation only: keep legacy clicks/capture and never put a global
            // synchronous mouse hook on the thread doing physics and GPU readback.
            RegisterRawInputDevices(
                &[RAWINPUTDEVICE {
                    usUsagePage: 1,
                    usUsage: 2,
                    dwFlags: RIDEV_INPUTSINK,
                    hwndTarget: control,
                }],
                size_of::<RAWINPUTDEVICE>() as u32,
            )?;
            let environment = preferences.light.environment();
            let sound =
                RugSound::new(preferences.sound && !silent && !preview).map_err(io_error)?;
            let mut app = Self {
                control,
                overlays: Vec::new(),
                desktop,
                renderer,
                cloth,
                preferences,
                language,
                custom_available,
                settings_path,
                textures: texture::Worker::new(data_directory, photo).map_err(io_error)?,
                texture_sequence: 0,
                sound,
                thumps: ThumpDetector::default(),
                scanner: Scanner::new().map_err(io_error)?,
                hidden: false,
                peek: false,
                peek_alpha: 1.,
                environment,
                last_light: Instant::now(),
                last_tick: Instant::now(),
                accumulated: Duration::ZERO,
                last_scan: Instant::now(),
                scan_after: None,
                drag_bump: None,
                left_down: false,
                last_pointer: [f32::NAN; 2],
                render_dirty: false,
                grab_window: None,
                last_grab_end: Instant::now() - Duration::from_secs(10),
                returning_from_grab: false,
                soften_released_fold: false,
                fold_contrast: 1.,
                smooth_slide: 0.,
                active_before: true,
                running: true,
                taskbar_message: RegisterWindowMessageW(w!("TaskbarCreated")),
                timer_interval: 16,
                drop_window,
                drop_receiver: Some(drop_receiver),
                drop_shown: None,
                icon_error: None,
                texture_error: None,
                action_error: preference_error,
                input_unavailable: false,
                reconnect_since: None,
                last_reconnect: Instant::now() - Duration::from_secs(1),
                preview,
                last_draw: Instant::now(),
                created: Instant::now(),
                last_presented: 0,
            };
            app.rebuild_overlays()?;
            app.add_tray()?;
            if SetTimer(Some(control), TIMER_ID, 16, None) == 0 {
                return Err(Error::from_thread());
            }
            app.renderer.render_sync(&app.cloth, 1., app.shading())?;
            app.present()?;
            if !preview {
                app.scanner.request();
            } else {
                let _ = ShowWindow(control, SW_SHOWNORMAL);
            }
            app.sound.wake();
            logging::write(format!(
                "startup displays={} desktop={:?}",
                app.desktop.screens.len(),
                app.desktop.size
            ));
            Ok(app)
        }
    }
    fn add_tray(&self) -> Result<()> {
        let mut data = self.tray_data();
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        data.uCallbackMessage = TRAY_MESSAGE;
        unsafe {
            let instance = GetModuleHandleW(None)?;
            data.hIcon = LoadIconW(
                Some(instance.into()),
                resource_id(1),
            )
            .or_else(|_| LoadIconW(None, IDI_APPLICATION))?;
            let tip = format!(
                "Kilim · {}",
                self.language.hint(self.preferences.peek.label())
            );
            let tip: Vec<u16> = tip.encode_utf16().take(data.szTip.len() - 1).collect();
            data.szTip[..tip.len()].copy_from_slice(&tip);
            if !Shell_NotifyIconW(NIM_MODIFY, &data).as_bool()
                && !Shell_NotifyIconW(NIM_ADD, &data).as_bool()
            {
                return Err(Error::from_thread());
            }
        }
        Ok(())
    }
    fn tray_data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.control,
            uID: 1,
            ..Default::default()
        }
    }
    fn wake(&mut self) {
        let asleep = self.cloth.settled > 45;
        self.cloth.settled = 0;
        if asleep {
            self.last_tick = Instant::now();
            self.accumulated = Duration::ZERO;
        }
        if !self.hidden {
            self.sound.wake();
        } else {
            self.sound.stop_soon();
        }
    }
    fn present(&mut self) -> Result<()> {
        for overlay in &self.overlays {
            self.renderer
                .present_to(overlay.window, overlay.screen, !self.hidden)?;
        }
        self.last_presented = self.renderer.frame_sequence();
        Ok(())
    }
    fn point(&self) -> Result<[f32; 2]> {
        let mut point = POINT::default();
        unsafe {
            GetCursorPos(&mut point)?;
        }
        Ok([
            point.x as f32 - self.desktop.origin.x as f32,
            point.y as f32 - self.desktop.origin.y as f32,
        ])
    }
    fn local_point(&self, screen: [i32; 2]) -> [f32; 2] {
        [
            screen[0] as f32 - self.desktop.origin.x as f32,
            screen[1] as f32 - self.desktop.origin.y as f32,
        ]
    }
    fn update_hit(&self, point: [f32; 2]) {
        let over = !self.cloth.grabbing()
            && !self.hidden
            && !self.peek
            && !self.left_down
            && self.cloth.hit_node(point).is_some();
        for overlay in &self.overlays {
            let pass = if self.cloth.grabbing() {
                Some(overlay.window) != self.grab_window
            } else {
                self.hidden || self.peek || self.left_down || !over
            };
            unsafe {
                let style = GetWindowLongPtrW(overlay.window, GWL_EXSTYLE) as u32;
                let next = if pass {
                    style | WS_EX_TRANSPARENT.0
                } else {
                    style & !WS_EX_TRANSPARENT.0
                };
                if next != style {
                    SetWindowLongPtrW(overlay.window, GWL_EXSTYLE, next as isize);
                }
            }
        }
    }
    fn release_grab(&mut self) {
        if self.cloth.grabbing() {
            logging::write(format!(
                "grab released: max height {:.1}",
                self.cloth
                    .position
                    .iter()
                    .map(|p| p[2])
                    .fold(0_f32, f32::max)
            ));
            self.cloth.end_grab();
            self.last_grab_end = Instant::now();
            self.smooth_slide = 0.;
            self.sound.slide(0.);
            self.thumps.cancel_pending();
            self.wake();
            self.returning_from_grab = true;
            self.soften_released_fold = true;
        }
        self.grab_window = None;
        unsafe {
            if self
                .overlays
                .iter()
                .any(|overlay| GetCapture() == overlay.window)
            {
                let _ = ReleaseCapture();
            }
            if let Ok(cursor) = LoadCursorW(None, IDC_ARROW) {
                SetCursor(Some(cursor));
            }
        }
    }
    fn handle(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Tick
                if self.last_draw.elapsed()
                    >= Duration::from_millis(self.timer_interval.into()) =>
            {
                self.tick()?
            }
            Event::Down(double, window, screen)
                if !self.peek
                    && !self.hidden
                    && self.overlays.iter().any(|overlay| overlay.window == window) =>
            {
                let point = self.local_point(screen);
                if double {
                    self.release_grab();
                    self.returning_from_grab = false;
                    self.soften_released_fold = false;
                    let center = self.cloth.position.iter().map(|p| p[0]).sum::<f32>()
                        / self.cloth.position.len() as f32;
                    self.cloth.toggle_roll(point[0] > center);
                    self.wake();
                } else if self.cloth.begin_grab(point) {
                    self.returning_from_grab = false;
                    self.soften_released_fold = false;
                    logging::write(format!("grab started: {point:?}"));
                    self.grab_window = Some(window);
                    self.drag_bump = None;
                    unsafe {
                        SetCapture(window);
                        let instance = GetModuleHandleW(None)?;
                        SetCursor(Some(
                            LoadCursorW(
                                Some(instance.into()),
                                resource_id(2),
                            )
                            .or_else(|_| LoadCursorW(None, IDC_HAND))?,
                        ));
                    }
                    self.wake();
                }
            }
            Event::Up => self.release_grab(),
            Event::Menu => {
                if let Err(error) = self.menu() {
                    self.report_action_error(error);
                }
            }
            Event::Command(command) => {
                if let Err(error) = self.command(command) {
                    self.report_action_error(error);
                }
            }
            Event::Display => self.schedule_reconnect(),
            Event::Mouse(message, point) => self.mouse(message, self.local_point(point)),
            Event::Hover(point) if !self.cloth.grabbing() && !self.left_down => {
                self.update_hit(self.local_point(point));
            }
            _ => {}
        }
        Ok(())
    }
    fn mouse(&mut self, message: u32, point: [f32; 2]) {
        match message {
            WM_LBUTTONDOWN => {
                self.left_down = true;
                self.drag_bump = None;
                if !self.cloth.grabbing() && (self.peek || self.cloth.hit_node(point).is_none()) {
                    self.drag_bump = self
                        .cloth
                        .bumps
                        .iter()
                        .enumerate()
                        .filter_map(|(index, bump)| {
                            let distance = (point[0] - bump[0]).hypot(point[1] - bump[1]);
                            (distance < 42. * self.cloth.point_scale()).then_some((
                                index,
                                [bump[0] - point[0], bump[1] - point[1]],
                                distance,
                            ))
                        })
                        .min_by(|a, b| a.2.total_cmp(&b.2))
                        .map(|(index, offset, _)| DragBump {
                            index,
                            offset,
                            start: point,
                            moved: false,
                        });
                }
            }
            WM_MOUSEMOVE if self.cloth.grabbing() => {
                self.cloth.move_grab(point);
                self.wake();
            }
            WM_MOUSEMOVE if self.left_down => {
                let mut changed = false;
                if let Some(drag) = &mut self.drag_bump {
                    if !drag.moved
                        && (point[0] - drag.start[0]).hypot(point[1] - drag.start[1])
                            >= 6. * self.cloth.point_scale()
                    {
                        drag.moved = true;
                    }
                    if drag.moved && drag.index < self.cloth.bumps.len() {
                        self.cloth.bumps[drag.index] =
                            [point[0] + drag.offset[0], point[1] + drag.offset[1]];
                        changed = true;
                    }
                }
                if changed {
                    self.wake();
                }
            }
            WM_LBUTTONUP => {
                self.mouse(WM_MOUSEMOVE, point);
                self.left_down = false;
                self.drag_bump = None;
                self.scan_after = Some(Instant::now() + Duration::from_millis(700));
                self.release_grab();
            }
            _ => {}
        }
        if message != WM_MOUSEMOVE
            && (message != WM_LBUTTONDOWN
                || self.peek
                || self.hidden
                || self.cloth.hit_node(point).is_none())
        {
            self.update_hit(point);
        }
    }
    /// Light settings as handed to the renderer. The contact shadow exists to lift the rug off the
    /// desktop, but a sheet that covers the whole desktop can only cast it through the gaps you
    /// open by lifting, where it reads as a grey wash, so the full-screen mode drops it entirely.
    fn shading(&self) -> LightEnv {
        let mut environment = self.environment;
        if self.preferences.fullscreen {
            environment.tint[3] = 0.;
            environment.fold_contrast = self.fold_contrast;
        }
        environment
    }
    fn tick(&mut self) -> Result<()> {
        let tick_started = Instant::now();
        let previous_draw = self.last_draw;
        self.last_draw = tick_started;
        let invalid = unsafe {
            !IsWindow(Some(self.desktop.host)).as_bool()
                || self
                    .overlays
                    .iter()
                    .any(|overlay| !IsWindow(Some(overlay.window)).as_bool())
        };
        if invalid || self.reconnect_since.is_some() {
            if self.reconnect_since.is_none() {
                self.schedule_reconnect();
            }
            if self.last_reconnect.elapsed() >= Duration::from_millis(500) {
                self.last_reconnect = Instant::now();
                if let Err(error) = self.reconnect() {
                    self.icon_error = Some(error.to_string());
                    logging::write(format!("desktop reconnect pending: {error}"));
                } else {
                    self.reconnect_since = None;
                }
            }
            if self.reconnect_since.is_some() {
                return Ok(());
            }
        }
        let point = match self.point() {
            Ok(point) => {
                if self.input_unavailable {
                    self.input_unavailable = false;
                    self.last_tick = Instant::now();
                    self.accumulated = Duration::ZERO;
                    self.wake();
                }
                point
            }
            Err(error) => {
                if !self.input_unavailable {
                    self.release_grab();
                    self.sound.stop_soon();
                    self.active_before = false;
                    self.input_unavailable = true;
                    logging::write(format!("input temporarily unavailable: {error}"));
                }
                return Ok(());
            }
        };
        let held = unsafe { GetAsyncKeyState(self.preferences.peek.virtual_key()) < 0 };
        let down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 };
        let peek = held || (self.peek && down);
        if peek != self.peek && !self.cloth.grabbing() {
            self.peek = peek;
            self.wake();
        }
        // Sample the latest absolute cursor once per frame; do not replay thousands of stale
        // WM_MOUSEMOVE events or wake the audio worker once per hardware mouse report.
        if !down && (self.left_down || self.cloth.grabbing()) {
            self.mouse(WM_LBUTTONUP, point);
        }
        if point != self.last_pointer {
            self.mouse(WM_MOUSEMOVE, point);
            self.last_pointer = point;
        }
        self.update_hit(point);
        let mut changed = false;
        if let Some(scan) = self.scanner.poll() {
            match scan.result {
                Ok(points) => {
                    self.icon_error = None;
                    let origin = self.desktop.origin;
                    let view = self.desktop.size;
                    let points: Vec<_> = points
                        .into_iter()
                        .map(|p| [p[0] - origin.x as f32, p[1] - origin.y as f32])
                        .filter(|p| {
                            p[0] > -60.
                                && p[0] < view[0] + 60.
                                && p[1] > -60.
                                && p[1] < view[1] + 60.
                        })
                        .collect();
                    if points != self.cloth.bumps && !self.cloth.grabbing() && !down {
                        self.cloth.bumps = points;
                        self.wake();
                    }
                }
                Err(error) => {
                    logging::write(format!("desktop scan: {error}"));
                    self.icon_error = Some(error);
                }
            }
        }
        let scan_due = self.last_scan.elapsed() > Duration::from_secs(4)
            || self.scan_after.is_some_and(|time| Instant::now() >= time);
        if !self.preview && scan_due && !self.hidden && !self.cloth.grabbing() && !down {
            self.scanner.request();
            self.last_scan = Instant::now();
            self.scan_after = None;
        }
        while let Some(completed) = self.textures.poll() {
            self.custom_available = completed.has_custom;
            if completed.id == self.texture_sequence {
                match completed.result {
                    Ok(image) => {
                        self.renderer.set_texture(&image)?;
                        self.preferences.theme = completed.theme;
                        self.save_preferences();
                        self.texture_error = None;
                        self.wake();
                    }
                    Err(error) => {
                        self.texture_error = Some(error.to_string());
                        logging::write(format!("texture: {error}"));
                    }
                }
            }
        }
        let drops: Vec<_> = self
            .drop_receiver
            .as_ref()
            .map(|receiver| receiver.events.try_iter().collect())
            .unwrap_or_default();
        for event in drops {
            match event {
                DropEvent::Image(path) => {
                    self.texture_sequence = self.textures.request("custom", Some(path));
                    self.hide_drop_panel();
                }
                DropEvent::Leave => self.hide_drop_panel(),
            }
        }
        if down && !self.cloth.grabbing() {
            self.offer_tray_drop(POINT {
                x: point[0] as i32 + self.desktop.origin.x,
                y: point[1] as i32 + self.desktop.origin.y,
            })?;
        }
        if self
            .drop_shown
            .is_some_and(|shown| !down && shown.elapsed() > Duration::from_millis(250))
        {
            self.hide_drop_panel();
        }
        if self.preferences.light == LightMode::Auto
            && self.last_light.elapsed() >= Duration::from_secs(60)
        {
            let environment = self.preferences.light.environment();
            if environment != self.environment {
                self.environment = environment;
                self.wake();
            }
            self.last_light = Instant::now();
        }
        if let Some(error) = self.sound.error() {
            logging::write(format!("audio: {error}"));
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick);
        let fold_elapsed = now.duration_since(if self.soften_released_fold {
            self.last_tick.max(self.last_grab_end)
        } else {
            self.last_tick
        });
        self.last_tick = now;
        let target = if self.peek { 0.26 } else { 1. };
        if (self.peek_alpha - target).abs() > 0.012 {
            self.peek_alpha += (target - self.peek_alpha) * 0.28;
            self.cloth.settled = 0;
            changed = true;
        } else if self.peek_alpha != target {
            self.peek_alpha = target;
            changed = true;
        }
        let active = !self.hidden && (self.cloth.settled <= 45 || self.cloth.grabbing());
        let fold_target = if self.soften_released_fold {
            RELEASE_FOLD_CONTRAST
        } else {
            1.
        };
        if self.preferences.fullscreen && !self.hidden {
            let contrast =
                next_fold_contrast(self.fold_contrast, self.soften_released_fold, fold_elapsed);
            changed |= contrast != self.fold_contrast;
            self.fold_contrast = contrast;
        }
        if active {
            self.accumulated += elapsed.min(Duration::from_millis(50));
            let step = Duration::from_secs_f64(1. / 60.);
            let mut count = 0;
            while self.accumulated >= step && count < 2 {
                self.cloth.step();
                self.accumulated -= step;
                count += 1;
                changed = true;
            }
            if count == 2 {
                self.accumulated = Duration::ZERO;
            }
            if count > 0 {
                self.feed_sound();
            }
        } else {
            self.accumulated = Duration::ZERO;
            if !self.hidden {
                self.returning_from_grab = false;
            }
        }
        if self.active_before && !active {
            self.sound.stop_soon();
        }
        self.active_before = active;
        self.render_dirty |= changed;
        if self.render_dirty
            && !self.hidden
            && self
                .renderer
                .submit(&self.cloth, self.peek_alpha, self.shading())?
        {
            self.render_dirty = false;
        }
        let _ = self.renderer.poll()?;
        if self.renderer.frame_sequence() != self.last_presented {
            self.present()?;
        }
        let interval = if active
            || self.renderer.pending()
            || self.render_dirty && !self.hidden
            || self.preferences.fullscreen && !self.hidden && self.fold_contrast != fold_target
            || self.peek_alpha != target
        {
            16
        } else {
            50
        };
        if interval != self.timer_interval {
            unsafe {
                if SetTimer(Some(self.control), TIMER_ID, interval, None) == 0 {
                    return Err(Error::from_thread());
                }
            }
            self.timer_interval = interval;
        }
        if self.cloth.grabbing() && logging::enabled() {
            if tick_started.duration_since(previous_draw) > Duration::from_millis(34) {
                logging::write(format!(
                    "stall: {:.1} ms",
                    tick_started.duration_since(previous_draw).as_secs_f64() * 1000.
                ));
            }
            if tick_started.elapsed() > Duration::from_millis(12) {
                logging::write(format!(
                    "slow draw: {:.1} ms CPU",
                    tick_started.elapsed().as_secs_f64() * 1000.
                ));
            }
        }
        // Schedule from completion, not from the start of an expensive frame. A slow draw must
        // not immediately trigger another catch-up draw on the next raw mouse message.
        self.last_draw = Instant::now();
        Ok(())
    }
    fn feed_sound(&mut self) {
        if self.returning_from_grab {
            // Let the queued sound decay naturally while the falling cloth settles.
            return;
        }
        let slide = (self.cloth.slide_level / 3.).min(1.);
        self.smooth_slide += (slide * slide.sqrt() - self.smooth_slide) * 0.18;
        self.sound.slide(self.smooth_slide);
        let carrying =
            self.cloth.grabbing() || self.last_grab_end.elapsed() < Duration::from_millis(900);
        if let Some(amplitude) = self.thumps.feed(
            self.cloth.impact_sum,
            self.created.elapsed().as_secs_f64(),
            carrying || self.cloth.settled > 20,
        ) {
            self.sound.thump(amplitude);
        }
    }
    fn save_preferences(&mut self) {
        if let Err(error) = self.preferences.save(&self.settings_path) {
            self.report_action_error(error);
        }
    }
    fn report_action_error(&mut self, error: impl std::fmt::Display) {
        let message = error.to_string();
        logging::write(format!("action failed: {message}"));
        self.action_error = Some(message);
        unsafe {
            let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(MB_ICONWARNING);
        }
    }
    fn command(&mut self, command: usize) -> Result<()> {
        match command {
            SHOW => {
                self.hidden = !self.hidden;
                if self.hidden {
                    for overlay in &self.overlays {
                        unsafe {
                            let _ = ShowWindow(overlay.window, SW_HIDE);
                        }
                    }
                    self.sound.stop_soon();
                } else {
                    self.wake();
                    self.renderer
                        .render_sync(&self.cloth, self.peek_alpha, self.shading())?;
                    self.present()?;
                }
            }
            RESET => {
                self.release_grab();
                self.returning_from_grab = false;
                self.soften_released_fold = false;
                self.fold_contrast = 1.;
                self.hidden = false;
                place(&mut self.cloth, &self.desktop, &self.preferences);
                self.wake();
            }
            RESCAN => {
                if !self.preview {
                    self.scanner.request();
                }
                self.wake();
            }
            SOUND => {
                self.preferences.sound = !self.preferences.sound;
                self.sound.set_enabled(self.preferences.sound);
                self.save_preferences();
                self.wake();
            }
            CUSTOM => match self.choose_picture() {
                Ok(Some(path)) => {
                    self.texture_sequence = self.textures.request("custom", Some(path));
                }
                Ok(None) => {}
                Err(error) => self.report_action_error(error),
            },
            LOGIN => {
                if !self.preview
                    && let Err(error) =
                        startup::enabled().and_then(|enabled| startup::set_enabled(!enabled))
                    {
                        self.report_action_error(error);
                    }
            }
            SPONSOR => unsafe {
                let result = ShellExecuteW(
                    Some(self.control),
                    w!("open"),
                    w!("https://github.com/sponsors/yusufyilmazfr"),
                    None,
                    None,
                    SW_SHOWNORMAL,
                );
                if result.0 as isize <= 32 {
                    return Err(Error::new(E_FAIL, "Could not open sponsor page"));
                }
            },
            FULLSCREEN => {
                self.release_grab();
                self.returning_from_grab = false;
                self.soften_released_fold = false;
                self.fold_contrast = 1.;
                self.preferences.fullscreen = !self.preferences.fullscreen;
                self.save_preferences();
                place(&mut self.cloth, &self.desktop, &self.preferences);
                self.wake();
            }
            EXIT => self.running = false,
            1000..=1015 => {
                let index = command - 1000;
                let theme = if index < texture::PRESETS.len() {
                    texture::PRESETS[index].key
                } else {
                    "custom"
                };
                self.texture_sequence = self.textures.request(theme, None);
            }
            2000..=2003 => {
                self.preferences.light = LightMode::ALL[command - 2000];
                self.environment = self.preferences.light.environment();
                self.last_light = Instant::now();
                self.save_preferences();
                self.wake();
            }
            3000..=3003 => {
                let before = self.cloth.width;
                let size = RugSize::ALL[command - 3000];
                self.preferences.size = size;
                if self.preferences.fullscreen {
                    // The sheet covers the desktop, so the rug size only matters once it is off.
                } else if self.cloth.rolling || self.cloth.grabbing() {
                    self.release_grab();
                    self.returning_from_grab = false;
                    place(&mut self.cloth, &self.desktop, &self.preferences);
                } else {
                    self.cloth
                        .scale(rug_width(&self.desktop, size) / before.max(1.));
                }
                self.save_preferences();
                self.wake();
            }
            4000..=4003 => {
                self.preferences.peek = PeekKey::ALL[command - 4000];
                self.save_preferences();
                let _ = self.add_tray();
            }
            5000..=5020 => {
                let order = L10n::menu_order();
                self.language.set_language(order[command - 5000].0);
                self.preferences.language = self.language.language.clone();
                self.save_preferences();
                unsafe {
                    SetWindowTextW(
                        self.drop_window,
                        &HSTRING::from(self.language.text(Text::CustomRug)),
                    )?;
                }
                let _ = self.add_tray();
            }
            _ => {}
        }
        Ok(())
    }
    fn schedule_reconnect(&mut self) {
        if self.reconnect_since.is_none() {
            self.reconnect_since = Some(Instant::now());
            self.last_reconnect = Instant::now() - Duration::from_secs(1);
        }
        self.release_grab();
        self.drag_bump = None;
    }
    fn rebuild_overlays(&mut self) -> Result<()> {
        unsafe {
            for overlay in self.overlays.drain(..) {
                if IsWindow(Some(overlay.window)).as_bool() {
                    DestroyWindow(overlay.window)?;
                }
            }
            let instance = GetModuleHandleW(None)?;
            for screen in &self.desktop.screens {
                let window = CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    w!("KilimRustWindow"),
                    w!("Kilim Windows Rug"),
                    WS_CHILD,
                    screen.rect.x,
                    screen.rect.y,
                    screen.rect.width as i32,
                    screen.rect.height as i32,
                    Some(self.desktop.host),
                    None,
                    Some(instance.into()),
                    None,
                )?;
                SetWindowPos(
                    window,
                    Some(HWND_TOP),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                )?;
                self.overlays.push(Overlay {
                    window,
                    screen: screen.rect,
                });
            }
        }
        Ok(())
    }
    fn reconnect(&mut self) -> Result<()> {
        let desktop = if self.preview {
            preview_desktop(self.control)?
        } else {
            Desktop::connect()?
        };
        let scale = desktop.primary().scale;
        let factor = scale / self.cloth.point_scale();
        let delta = [
            self.desktop.origin.x as f32 * factor - desktop.origin.x as f32,
            self.desktop.origin.y as f32 * factor - desktop.origin.y as f32,
        ];
        self.cloth.set_point_scale(scale);
        self.cloth.view = desktop.size;
        self.cloth.translate(delta);
        self.renderer.resize_view(desktop.size);
        self.desktop = desktop;
        self.rebuild_overlays()?;
        self.renderer
            .render_sync(&self.cloth, self.peek_alpha, self.shading())?;
        self.present()?;
        if !self.preview {
            self.scanner.request();
        }
        let data = self.tray_data();
        unsafe {
            if !Shell_NotifyIconW(NIM_MODIFY, &data).as_bool() {
                self.add_tray()?;
            }
        }
        self.wake();
        Ok(())
    }
}
impl App {
    fn choose_picture(&self) -> Result<Option<PathBuf>> {
        // SAFETY: a native UI-STA file picker owns its modal dialog and returned COM strings.
        unsafe {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.SetOptions(
                FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM | FOS_DONTADDTORECENT,
            )?;
            dialog.SetTitle(&HSTRING::from(self.language.text(Text::CustomRug)))?;
            let filters = [
                COMDLG_FILTERSPEC {
                    pszName: w!("Images"),
                    pszSpec: w!("*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.tif;*.tiff;*.webp;*.heic"),
                },
                COMDLG_FILTERSPEC {
                    pszName: w!("All files"),
                    pszSpec: w!("*.*"),
                },
            ];
            dialog.SetFileTypes(&filters)?;
            if let Err(error) = dialog.Show(Some(self.control)) {
                if error.code().0 == 0x800704C7u32 as i32 {
                    return Ok(None);
                }
                return Err(error);
            }
            let item = dialog.GetResult()?;
            let value = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let text = value.to_string();
            CoTaskMemFree(Some(value.0.cast()));
            Ok(Some(PathBuf::from(text?)))
        }
    }
    fn menu(&mut self) -> Result<()> {
        // SAFETY: the root owns its submenus; Win32 copies labels and the root is destroyed exactly once.
        unsafe {
            let menu = CreatePopupMenu()?;
            let result = (|| -> Result<()> {
                if self.icon_error.is_some() {
                    append(menu, RESCAN, &self.language.text(Text::NoPermission), false)?;
                    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                }
                append(
                    menu,
                    SHOW,
                    &self.language.text(if self.hidden {
                        Text::ShowRug
                    } else {
                        Text::HideRug
                    }),
                    false,
                )?;
                append(
                    menu,
                    RESET,
                    &format!("{}\tCtrl+R", self.language.text(Text::TossRug)),
                    false,
                )?;
                append(menu, RESCAN, &self.language.text(Text::Rescan), false)?;
                AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                let patterns = pattern_entries(
                    &self.language,
                    &self.preferences.theme,
                    self.custom_available,
                );
                submenu(menu, &self.language.text(Text::Pattern), patterns)?;
                append(
                    menu,
                    FULLSCREEN,
                    &self.language.fullscreen_label(),
                    self.preferences.fullscreen,
                )?;
                let keys = [
                    Text::LightAuto,
                    Text::LightDay,
                    Text::LightEvening,
                    Text::LightNight,
                ];
                submenu(
                    menu,
                    &self.language.text(Text::Light),
                    LightMode::ALL
                        .into_iter()
                        .enumerate()
                        .map(|(index, mode)| {
                            (
                                2000 + index,
                                self.language.text(keys[index]),
                                self.preferences.light == mode,
                            )
                        })
                        .collect(),
                )?;
                submenu(
                    menu,
                    &self.language.text(Text::Size),
                    RugSize::ALL
                        .into_iter()
                        .enumerate()
                        .map(|(index, size)| {
                            (
                                3000 + index,
                                size.label().into(),
                                self.preferences.size == size,
                            )
                        })
                        .collect(),
                )?;
                append(
                    menu,
                    SOUND,
                    &self.language.text(Text::Sounds),
                    self.preferences.sound,
                )?;
                submenu(
                    menu,
                    &self.language.text(Text::Language),
                    L10n::menu_order()
                        .into_iter()
                        .enumerate()
                        .map(|(index, (code, name))| {
                            (5000 + index, name.into(), self.language.language == code)
                        })
                        .collect(),
                )?;
                AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                append(
                    menu,
                    LOGIN,
                    &self.language.text(Text::LaunchAtLogin),
                    !self.preview && startup::enabled().unwrap_or(false),
                )?;
                AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                append(menu, SPONSOR, &self.language.text(Text::Sponsor), false)?;
                AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                submenu(
                    menu,
                    &self.language.text(Text::PeekKey),
                    PeekKey::ALL
                        .into_iter()
                        .enumerate()
                        .map(|(index, key)| {
                            (
                                4000 + index,
                                key.label().into(),
                                self.preferences.peek == key,
                            )
                        })
                        .collect(),
                )?;
                AppendMenuW(
                    menu,
                    MF_STRING | MF_DISABLED,
                    0,
                    &HSTRING::from(self.language.hint(self.preferences.peek.label())),
                )?;
                if let Some(error) = &self.texture_error {
                    AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, &HSTRING::from(error))?;
                }
                if let Some(error) = &self.action_error {
                    AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, &HSTRING::from(error))?;
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                append(
                    menu,
                    EXIT,
                    &format!("{}\tCtrl+Q", self.language.text(Text::Quit)),
                    false,
                )?;
                let mut point = POINT::default();
                GetCursorPos(&mut point)?;
                let _ = SetForegroundWindow(self.control);
                let command = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    None,
                    self.control,
                    None,
                )
                .0;
                let _ = PostMessageW(Some(self.control), WM_NULL, WPARAM(0), LPARAM(0));
                if command > 0 {
                    self.command(command as usize)?;
                }
                Ok(())
            })();
            let _ = DestroyMenu(menu);
            result
        }
    }
    fn offer_tray_drop(&mut self, cursor: POINT) -> Result<()> {
        if self.drop_shown.is_some() {
            return Ok(());
        }
        let identifier = NOTIFYICONIDENTIFIER {
            cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: self.control,
            uID: 1,
            ..Default::default()
        };
        unsafe {
            if let Ok(icon) = Shell_NotifyIconGetRect(&identifier)
                && cursor.x >= icon.left - 3
                    && cursor.x <= icon.right + 3
                    && cursor.y >= icon.top - 3
                    && cursor.y <= icon.bottom + 3
                {
                    let monitor = MonitorFromRect(&icon, MONITOR_DEFAULTTONEAREST);
                    let mut info = MONITORINFO {
                        cbSize: size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    let _ = GetMonitorInfoW(monitor, &mut info);
                    let x = (icon.left - 100).clamp(
                        info.rcMonitor.left,
                        (info.rcMonitor.right - 240).max(info.rcMonitor.left),
                    );
                    let y = (icon.top - 60).clamp(
                        info.rcMonitor.top,
                        (info.rcMonitor.bottom - 90).max(info.rcMonitor.top),
                    );
                    // Only this temporary drop panel floats over the notification area; rug windows never use TOPMOST.
                    SetWindowPos(
                        self.drop_window,
                        Some(HWND_TOPMOST),
                        x,
                        y,
                        240,
                        90,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    )?;
                    self.drop_shown = Some(Instant::now());
                }
        }
        Ok(())
    }
    fn hide_drop_panel(&mut self) {
        unsafe {
            let _ = ShowWindow(self.drop_window, SW_HIDE);
        }
        self.drop_shown = None;
    }
    fn smoke_test(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let _ = LoadIconW(
                Some(instance.into()),
                resource_id(1),
            )?;
            let _ = LoadCursorW(
                Some(instance.into()),
                resource_id(2),
            )?;
        }
        if !self.preview {
            self.cloth.bumps = self.desktop.icon_positions()?;
        }
        for _ in 0..240 {
            self.cloth.step();
        }
        self.renderer
            .render_sync(&self.cloth, 1., self.shading())?;
        self.present()?;
        for overlay in &self.overlays {
            unsafe {
                if GetParent(overlay.window)? != self.desktop.host
                    || GetWindowLongPtrW(overlay.window, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0
                {
                    return Err(Error::new(E_FAIL, "Invalid desktop window hierarchy"));
                }
            }
        }
        let center = [
            self.cloth.position.iter().map(|p| p[0]).sum::<f32>()
                / self.cloth.position.len() as f32,
            self.cloth.position.iter().map(|p| p[1]).sum::<f32>()
                / self.cloth.position.len() as f32,
        ];
        self.left_down = false;
        self.update_hit(center);
        let point = POINT {
            x: center[0] as i32,
            y: center[1] as i32,
        };
        let normal = unsafe {
            ChildWindowFromPointEx(
                self.desktop.host,
                point,
                CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT,
            )
        };
        if !self.overlays.iter().any(|overlay| overlay.window == normal) {
            return Err(Error::new(E_FAIL, "Desktop hit test did not select rug"));
        }
        self.peek = true;
        self.update_hit(center);
        let peek = unsafe {
            ChildWindowFromPointEx(
                self.desktop.host,
                point,
                CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT,
            )
        };
        if self.overlays.iter().any(|overlay| overlay.window == peek) {
            return Err(Error::new(E_FAIL, "Peek did not pass through"));
        }
        self.peek = false;
        self.command(SHOW)?;
        self.command(RESET)?;
        if self.hidden {
            return Err(Error::new(E_FAIL, "Reset failed to show hidden rug"));
        }
        for _ in 0..200 {
            self.cloth.step();
        }
        self.command(3002)?;
        if self.preferences.size != RugSize::Large {
            return Err(Error::new(E_FAIL, "Size command failed"));
        }
        self.command(2002)?;
        if self.preferences.light != LightMode::Evening {
            return Err(Error::new(E_FAIL, "Evening light command failed"));
        }
        let started = Instant::now();
        for _ in 0..30 {
            self.cloth.step();
            self.renderer
                .render_sync(&self.cloth, 1., self.shading())?;
            self.present()?;
        }
        println!(
            "Desktop smoke passed: displays={}, virtual={:?}, layers=desktop, peek=pass-through, hidden-reset=visible, sizes=source, light=evening, crop={:?}, msaa=4; {:.2} ms/frame",
            self.overlays.len(),
            self.desktop.size,
            self.renderer.allocation_size(),
            started.elapsed().as_secs_f64() * 1000. / 30.
        );
        Ok(())
    }
}
impl Drop for App {
    fn drop(&mut self) {
        self.drop_receiver.take();
        unsafe {
            if let Err(error) = RegisterRawInputDevices(
                &[RAWINPUTDEVICE {
                    usUsagePage: 1,
                    usUsage: 2,
                    dwFlags: RIDEV_REMOVE,
                    hwndTarget: HWND::default(),
                }],
                size_of::<RAWINPUTDEVICE>() as u32,
            ) {
                logging::write(format!("raw mouse unregister: {error}"));
            }
            let _ = KillTimer(Some(self.control), TIMER_ID);
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.tray_data());
            for overlay in &self.overlays {
                if IsWindow(Some(overlay.window)).as_bool() {
                    let _ = DestroyWindow(overlay.window);
                }
            }
            let _ = DestroyWindow(self.drop_window);
            let _ = DestroyWindow(self.control);
        }
    }
}
fn io_error(error: std::io::Error) -> Error {
    Error::new(E_FAIL, error.to_string())
}
fn next_fold_contrast(current: f32, returning_from_grab: bool, elapsed: Duration) -> f32 {
    let delta = (1. - RELEASE_FOLD_CONTRAST) * elapsed.as_secs_f32();
    if returning_from_grab {
        (current - delta / RELEASE_FOLD_FADE).max(RELEASE_FOLD_CONTRAST)
    } else {
        (current + delta / FOLD_LIGHT_RECOVERY).min(1.)
    }
}
fn resource_id(id:u16)->windows::core::PCWSTR{
    // MAKEINTRESOURCEW encodes an integer tag in a pointer-sized argument; it is never dereferenced.
    windows::core::PCWSTR(std::ptr::without_provenance(usize::from(id)))
}
/// Extra sheet around the desktop so its woven border and fringe stay off the visible area.
const FULLSCREEN_BLEED: f32 = 64.;
fn rug_width(desktop: &Desktop, size: RugSize) -> f32 {
    let screen = desktop.primary();
    let width = screen.rect.width as f32;
    let scale = screen.scale;
    (((width / scale) * 0.36).clamp(420., 760.) * size.factor() * scale).min(width * 0.92)
}
fn place(cloth: &mut Cloth, desktop: &Desktop, settings: &Settings) {
    if settings.fullscreen {
        cloth.reset_fullscreen(desktop.size, FULLSCREEN_BLEED * desktop.primary().scale);
    } else {
        place_rug(cloth, desktop, settings.size);
    }
}
fn place_rug(cloth: &mut Cloth, desktop: &Desktop, size: RugSize) {
    let screen = desktop.primary().rect;
    cloth.reset_pose(
        [
            screen.x as f32 + screen.width as f32 * 0.4,
            screen.y as f32 + screen.height as f32 * 0.52,
        ],
        rug_width(desktop, size),
        130.,
        -0.12,
    );
}
fn preview_desktop(control: HWND) -> Result<Desktop> {
    let mut rect = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        GetClientRect(control, &mut rect)?;
        if !ClientToScreen(control, &mut origin).as_bool() {
            return Err(Error::from_thread());
        }
    }
    let screen = Screen {
        rect: Rect {
            x: 0,
            y: 0,
            width: (rect.right - rect.left).max(1) as u32,
            height: (rect.bottom - rect.top).max(1) as u32,
        },
        primary: true,
        scale: unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(control) }.max(96) as f32 / 96.,
    };
    Ok(Desktop {
        host: control,
        list: HWND::default(),
        origin,
        size: [screen.rect.width as f32, screen.rect.height as f32],
        screens: vec![screen],
    })
}
fn append(menu: HMENU, id: usize, label: &str, checked: bool) -> Result<()> {
    unsafe {
        AppendMenuW(
            menu,
            MF_STRING | if checked { MF_CHECKED } else { MF_UNCHECKED },
            id,
            &HSTRING::from(label),
        )
    }
}
/// Pattern submenu: the fixed presets, then, only when a custom picture exists, a separator, the
/// checked custom pattern and the picture picker action.
fn pattern_entries(
    language: &L10n,
    theme: &str,
    custom_available: bool,
) -> Vec<(usize, String, bool)> {
    let mut entries: Vec<_> = texture::PRESETS
        .iter()
        .enumerate()
        .map(|(index, preset)| (1000 + index, language.theme(preset.key), theme == preset.key))
        .collect();
    if custom_available {
        entries.push((SEPARATOR, String::new(), false));
        entries.push((1015, language.custom_theme(), theme == "custom"));
    }
    entries.push((CUSTOM, language.text(Text::CustomRug), false));
    entries
}
fn submenu(root: HMENU, label: &str, items: Vec<(usize, String, bool)>) -> Result<()> {
    unsafe {
        let sub = CreatePopupMenu()?;
        let result = (|| {
            for (id, title, checked) in items {
                if id == SEPARATOR {
                    AppendMenuW(sub, MF_SEPARATOR, 0, None)?;
                } else {
                    append(sub, id, &title, checked)?;
                }
            }
            AppendMenuW(root, MF_POPUP, sub.0 as usize, &HSTRING::from(label))
        })();
        if result.is_err() {
            let _ = DestroyMenu(sub);
        }
        result
    }
}
fn enqueue_down(double: bool, window: HWND, parameter: LPARAM) {
    let packed = parameter.0 as u32;
    let mut point = POINT {
        x: (packed as u16 as i16) as i32,
        y: ((packed >> 16) as u16 as i16) as i32,
    };
    // Use event client coordinates, not the cursor's potentially later position.
    if unsafe { ClientToScreen(window, &mut point) }.as_bool() {
        enqueue(Event::Down(double, window, [point.x, point.y]));
    }
}
fn raw_mouse_buttons(flags: u16, point: [i32; 2]) {
    // Movement is sampled by tick. Button transitions retain their ordering, including a click
    // entirely between two frames, so desktop icon dragging still sees its starting position.
    for (flag, message) in [
        (RI_MOUSE_LEFT_BUTTON_DOWN, WM_LBUTTONDOWN),
        (RI_MOUSE_LEFT_BUTTON_UP, WM_LBUTTONUP),
    ] {
        if u32::from(flags) & flag != 0 {
            enqueue(Event::Mouse(message, point));
        }
    }
}
fn read_raw_mouse(lparam: LPARAM) -> Result<()> {
    let mut input = RAWINPUT::default();
    let mut size = size_of::<RAWINPUT>() as u32;
    // SAFETY: only mice are registered; RAWINPUT is aligned and large enough for RAWMOUSE.
    // The union is accessed only after its type/size are checked, before DefWindowProc cleanup.
    unsafe {
        let copied = GetRawInputData(
            HRAWINPUT(lparam.0 as *mut _),
            RID_INPUT,
            Some((&mut input as *mut RAWINPUT).cast()),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        );
        if copied == u32::MAX {
            return Err(Error::from_thread());
        }
        if copied < (size_of::<RAWINPUTHEADER>() + size_of::<RAWMOUSE>()) as u32
            || input.header.dwType != RIM_TYPEMOUSE.0
        {
            return Err(Error::new(E_FAIL, "Invalid raw mouse packet"));
        }
        let flags = input.data.mouse.Anonymous.Anonymous.usButtonFlags;
        let packed = GetMessagePos();
        let point = [
            (packed as u16 as i16) as i32,
            ((packed >> 16) as u16 as i16) as i32,
        ];
        if u32::from(flags) & (RI_MOUSE_LEFT_BUTTON_DOWN | RI_MOUSE_LEFT_BUTTON_UP) != 0 {
            raw_mouse_buttons(flags, point);
        } else {
            // Only hit transparency needs prompt hover updates. Physics, dragging and audio
            // still consume one latest pointer per frame, never one step per device report.
            enqueue(Event::Hover(point));
        }
    }
    Ok(())
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TIMER => enqueue(Event::Tick),
        WM_LBUTTONDOWN => enqueue_down(false, hwnd, lparam),
        WM_LBUTTONDBLCLK => enqueue_down(true, hwnd, lparam),
        WM_INPUT => {
            if let Err(error) = read_raw_mouse(lparam) {
                logging::write(format!("raw mouse input: {error}"));
            }
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
        WM_LBUTTONUP => {
            let packed = unsafe { GetMessagePos() };
            enqueue(Event::Mouse(
                WM_LBUTTONUP,
                [
                    (packed as u16 as i16) as i32,
                    ((packed >> 16) as u16 as i16) as i32,
                ],
            ));
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => enqueue(Event::Up),
        WM_RBUTTONUP => enqueue(Event::Menu),
        WM_COMMAND => enqueue(Event::Command(wparam.0 & 0xffff)),
        WM_DISPLAYCHANGE | WM_DPICHANGED => enqueue(Event::Display),
        WM_CLOSE => enqueue(Event::Command(EXIT)),
        WM_KEYDOWN if unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 } => {
            match wparam.0 as u32 {
                0x52 => enqueue(Event::Command(RESET)),
                0x51 => enqueue(Event::Command(EXIT)),
                _ => {}
            }
        }
        WM_MENUCHAR if unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 } => {
            match (wparam.0 & 0xffff) as u8 {
                b'r' | b'R' => enqueue(Event::Command(RESET)),
                b'q' | b'Q' => enqueue(Event::Command(EXIT)),
                _ => {}
            }
            return LRESULT((MNC_CLOSE as isize) << 16);
        }
        WM_SIZE
            if unsafe {
                GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_CHILD.0 == 0
                    && IsWindowVisible(hwnd).as_bool()
            } =>
        {
            enqueue(Event::Display)
        }
        WM_ERASEBKGND => {
            unsafe {
                if GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_CHILD.0 == 0 {
                    let mut rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rect);
                    FillRect(
                        HDC(wparam.0 as *mut _),
                        &rect,
                        GetSysColorBrush(COLOR_WINDOW),
                    );
                }
            }
            return LRESULT(1);
        }
        TRAY_MESSAGE
            if matches!(
                lparam.0 as u32,
                WM_RBUTTONUP | WM_LBUTTONUP | WM_CONTEXTMENU
            ) =>
        {
            enqueue(Event::Menu)
        }
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
    LRESULT(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_lighting_fades_through_a_late_frame_and_recovers_on_regrab() {
        let middle = next_fold_contrast(1., true, Duration::from_millis(175));
        assert!((middle - 0.65).abs() < 0.0001);
        let reduced = next_fold_contrast(middle, true, Duration::from_millis(250));
        assert_eq!(reduced, 0.30);
        assert_eq!(
            next_fold_contrast(reduced, true, Duration::from_secs(1)),
            reduced
        );
        let returning = next_fold_contrast(reduced, false, Duration::from_millis(60));
        assert!((returning - 0.65).abs() < 0.0001);
        assert_eq!(
            next_fold_contrast(returning, false, Duration::from_millis(61)),
            1.
        );
    }

    #[test]
    fn raw_mouse_motion_flood_keeps_button_edges_without_queuing_moves() {
        EVENTS.with(|events| events.borrow_mut().clear());
        raw_mouse_buttons(RI_MOUSE_LEFT_BUTTON_DOWN as u16, [12, 34]);
        for _ in 0..8000 {
            raw_mouse_buttons(0, [99, 99]);
        }
        raw_mouse_buttons(RI_MOUSE_LEFT_BUTTON_UP as u16, [56, 78]);
        let events = EVENTS.with(|events| events.borrow_mut().drain(..).collect::<Vec<_>>());
        assert!(matches!(
            events.as_slice(),
            [
                Event::Mouse(WM_LBUTTONDOWN, [12, 34]),
                Event::Mouse(WM_LBUTTONUP, [56, 78]),
            ]
        ));
        raw_mouse_buttons(
            (RI_MOUSE_LEFT_BUTTON_DOWN | RI_MOUSE_LEFT_BUTTON_UP) as u16,
            [1, 2],
        );
        let events = EVENTS.with(|events| events.borrow_mut().drain(..).collect::<Vec<_>>());
        assert!(matches!(
            events.as_slice(),
            [
                Event::Mouse(WM_LBUTTONDOWN, _),
                Event::Mouse(WM_LBUTTONUP, _),
            ]
        ));
    }
    #[test]
    fn modal_message_pumps_cannot_accumulate_timer_or_mouse_floods() {
        EVENTS.with(|events| events.borrow_mut().clear());
        TICK_QUEUED.with(|pending| pending.set(false));
        for index in 0..1000 {
            enqueue(Event::Mouse(WM_MOUSEMOVE, [index, index]));
            enqueue(Event::Tick);
        }
        let events = EVENTS.with(|events| events.borrow().iter().copied().collect::<Vec<_>>());
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::Tick))
                .count(),
            1
        );
        assert!(events.len() <= 3);
        assert!(matches!(
            events.last(),
            Some(Event::Mouse(WM_MOUSEMOVE, [999, 999]))
        ));
        EVENTS.with(|events| events.borrow_mut().clear());
        TICK_QUEUED.with(|pending| pending.set(false));
    }
    #[test]
    fn pattern_menu_shows_one_separated_custom_entry_plus_picker() {
        let language = L10n::new("zh");
        let plain = pattern_entries(&language, "kilim", false);
        assert_eq!(plain.len(), texture::PRESETS.len() + 1);
        assert_eq!(plain.last().map(|entry| entry.0), Some(CUSTOM));
        assert!(plain.iter().all(|entry| entry.0 != SEPARATOR));
        let custom = pattern_entries(&language, "custom", true);
        assert_eq!(custom.len(), texture::PRESETS.len() + 3);
        let separator = custom.len() - 3;
        assert_eq!(custom[separator].0, SEPARATOR);
        assert_eq!(custom[separator + 1].0, 1015);
        assert!(custom[separator + 1].2, "the active custom pattern is checked");
        let action = custom.last().expect("picker action");
        assert_eq!(action.0, CUSTOM);
        assert!(
            action.1.ends_with('…'),
            "the picker action keeps the ellipsis hint: {:?}",
            action.1
        );
        assert_ne!(
            custom[separator + 1].1, action.1,
            "the checked pattern and the picker must not share a label"
        );
        let labels: Vec<&str> = custom.iter().map(|entry| entry.1.as_str()).collect();
        let mut unique = labels.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            labels.len(),
            "duplicate pattern menu labels: {labels:?}"
        );
    }
    #[test]
    fn pattern_submenu_renders_a_real_separator() {
        let language = L10n::new("zh");
        let entries = pattern_entries(&language, "custom", true);
        let count = entries.len();
        // SAFETY: the root menu owns its submenu and is destroyed once at the end of the test.
        unsafe {
            let root = CreatePopupMenu().expect("root menu");
            submenu(root, "Pattern", entries).expect("pattern submenu");
            let sub = GetSubMenu(root, 0);
            assert!(!sub.is_invalid(), "submenu was not attached");
            assert_eq!(GetMenuItemCount(Some(sub)), count as i32);
            let separator = GetMenuState(sub, count as u32 - 3, MF_BYPOSITION);
            assert_eq!(
                separator & MF_SEPARATOR.0,
                MF_SEPARATOR.0,
                "the custom pattern group must start with a separator, not an empty item"
            );
            let _ = DestroyMenu(root);
        }
    }
}
