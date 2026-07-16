#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    ffi::c_void,
    mem::size_of,
    path::PathBuf,
    ptr,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use windows_sys::Win32::{
    Foundation::{
        BOOL, COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM,
        LRESULT, RECT, TRUE, WPARAM,
    },
    Graphics::{
        Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        Gdi::{
            BeginPaint, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER,
            DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_NORMAL,
            FillRect, GdiFlush, HGDIOBJ, InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
            SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
        },
    },
    System::{
        LibraryLoader::GetModuleHandleW,
        Threading::{
            CreateMutexW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
    },
    UI::{
        HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
            SetProcessDpiAwarenessContext,
        },
        Input::KeyboardAndMouse::ReleaseCapture,
        WindowsAndMessaging::{
            CREATESTRUCTW, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW,
            DispatchMessageW, EnumWindows, GWL_EXSTYLE, GWL_STYLE, GWLP_HWNDPARENT, GWLP_USERDATA,
            GetClientRect, GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId,
            HCURSOR, HTCAPTION, HWND_TOP, IDC_ARROW, IsIconic, IsWindowVisible, LWA_ALPHA,
            LoadCursorW, MA_NOACTIVATE, MSG, PostQuitMessage, RegisterClassW, SW_HIDE,
            SWP_NOACTIVATE, SWP_SHOWWINDOW, SendMessageW, SetLayeredWindowAttributes, SetTimer,
            SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, WM_DESTROY,
            WM_ERASEBKGND, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_NCCREATE,
            WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
            WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
        },
    },
};

use crate::{
    locale::AppLocale,
    model::{LimitWindow, UsageSnapshot},
};

const CLASS_NAME: &str = "ConfigCrate.CodexTitlebarMeter.Overlay";
const WINDOW_NAME: &str = "Codex Titlebar Meter";
const MUTEX_NAME: &str = "Local\\ConfigCrate.CodexTitlebarMeter.4BC6AD61";
const TRACK_TIMER: usize = 1;
const TRACK_INTERVAL_MS: u32 = 250;
const LOCALE_CHECK_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
struct Palette {
    accent: COLORREF,
}

impl Palette {
    const fn blue() -> Self {
        Self {
            accent: rgb(95, 145, 255),
        }
    }

    const fn green() -> Self {
        Self {
            accent: rgb(64, 196, 126),
        }
    }

    const fn purple() -> Self {
        Self {
            accent: rgb(173, 117, 255),
        }
    }
}

struct AppState {
    overlay: HWND,
    target: HWND,
    snapshot: UsageSnapshot,
    palette_index: usize,
    locale: AppLocale,
    next_locale_check: Instant,
}

unsafe impl Send for AppState {}

static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
static PALETTES: [Palette; 3] = [Palette::blue(), Palette::green(), Palette::purple()];
static CODEX_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn run() -> Result<()> {
    let Some(_instance) = InstanceMutex::acquire()? else {
        return Ok(());
    };
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    if instance.is_null() {
        bail!(
            "GetModuleHandleW failed: {}",
            std::io::Error::last_os_error()
        );
    }

    let class_name = wide(CLASS_NAME);
    let cursor = unsafe { LoadCursorW(ptr::null_mut(), IDC_ARROW) };
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: ptr::null_mut(),
        hCursor: cursor as HCURSOR,
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        bail!("RegisterClassW failed: {}", std::io::Error::last_os_error());
    }

    let window_name = wide(WINDOW_NAME);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        bail!(
            "CreateWindowExW failed: {}",
            std::io::Error::last_os_error()
        );
    }
    unsafe {
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
    }
    let saved_settings = crate::settings::Settings::load();

    STATE
        .set(Mutex::new(AppState {
            overlay: hwnd,
            target: ptr::null_mut(),
            snapshot: UsageSnapshot::default(),
            palette_index: saved_settings.palette_index % PALETTES.len(),
            locale: AppLocale::detect(),
            next_locale_check: Instant::now() + LOCALE_CHECK_INTERVAL,
        }))
        .map_err(|_| anyhow::anyhow!("application state was already initialized"))?;

    if unsafe { SetTimer(hwnd, TRACK_TIMER, TRACK_INTERVAL_MS, None) } == 0 {
        bail!("SetTimer failed: {}", std::io::Error::last_os_error());
    }
    track_codex_window();
    crate::codex::start_worker();

    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result == -1 {
            return Err(std::io::Error::last_os_error()).context("GetMessageW failed");
        }
        if result == 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

struct InstanceMutex(HANDLE);

impl InstanceMutex {
    fn acquire() -> Result<Option<Self>> {
        let name = wide(MUTEX_NAME);
        let handle = unsafe { CreateMutexW(ptr::null(), 1, name.as_ptr()) };
        if handle.is_null() {
            bail!("CreateMutexW failed: {}", std::io::Error::last_os_error());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(handle) };
            return Ok(None);
        }
        Ok(Some(Self(handle)))
    }
}

impl Drop for InstanceMutex {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            let create = lparam as *const CREATESTRUCTW;
            if !create.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize);
            }
            TRUE as LRESULT
        }
        WM_TIMER if wparam == TRACK_TIMER => {
            track_codex_window();
            0
        }
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONDOWN => {
            let x = (lparam as u16) as i32;
            let mut client = empty_rect();
            GetClientRect(hwnd, &mut client);
            let dpi = GetDpiForWindow(hwnd).max(96);
            let settings_width = (24 * dpi as i32 / 96).max(24);
            if x >= client.right - settings_width {
                cycle_palette(hwnd);
                return 0;
            }
            if let Some(state) = STATE.get().and_then(|state| state.lock().ok())
                && !state.target.is_null()
            {
                ReleaseCapture();
                SendMessageW(state.target, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            if let Some(state) = STATE.get().and_then(|state| state.lock().ok())
                && !state.target.is_null()
            {
                SendMessageW(state.target, WM_NCLBUTTONDBLCLK, HTCAPTION as usize, 0);
            }
            0
        }
        WM_RBUTTONUP => {
            cycle_palette(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn track_codex_window() {
    let Some(state_lock) = STATE.get() else {
        return;
    };
    let mut state = match state_lock.lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    let now = Instant::now();
    if now >= state.next_locale_check {
        let locale = AppLocale::detect();
        if locale != state.locale {
            state.locale = locale;
            unsafe { InvalidateRect(state.overlay, ptr::null(), 0) };
        }
        state.next_locale_check = now + LOCALE_CHECK_INTERVAL;
    }
    let target = find_codex_window();
    if target.is_null() || unsafe { IsIconic(target) } != 0 {
        CODEX_ACTIVE.store(false, Ordering::Relaxed);
        state.target = target;
        unsafe { ShowWindow(state.overlay, SW_HIDE) };
        return;
    }

    let mut bounds = empty_rect();
    if unsafe { GetWindowRect(target, &mut bounds) } == 0 {
        CODEX_ACTIVE.store(false, Ordering::Relaxed);
        unsafe { ShowWindow(state.overlay, SW_HIDE) };
        return;
    }

    let dpi = unsafe { GetDpiForWindow(target) }.max(96);
    let scale: f32 = dpi as f32 / 96.0_f32;
    let total_width = bounds.right - bounds.left;
    let top_margin = (5.0_f32 * scale).round() as i32;
    let height = (30.0_f32 * scale).round() as i32;
    let dual_window = state.snapshot.primary.is_some() && state.snapshot.weekly.is_some();
    let Some((relative_left, width)) = overlay_layout(total_width, scale, dual_window) else {
        CODEX_ACTIVE.store(true, Ordering::Relaxed);
        unsafe { ShowWindow(state.overlay, SW_HIDE) };
        return;
    };
    let left = bounds.left + relative_left;

    if state.target != target {
        unsafe {
            SetWindowLongPtrW(state.overlay, GWLP_HWNDPARENT, target as isize);
        }
    }
    state.target = target;
    CODEX_ACTIVE.store(true, Ordering::Relaxed);
    unsafe {
        SetWindowPos(
            state.overlay,
            HWND_TOP,
            left,
            bounds.top + top_margin,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        InvalidateRect(state.overlay, ptr::null(), 0);
    }
}

fn overlay_layout(total_width: i32, scale: f32, dual_window: bool) -> Option<(i32, i32)> {
    let left_reserve = (220.0_f32 * scale).round() as i32;
    let right_reserve = (158.0_f32 * scale).round() as i32;
    let preferred_width = if dual_window { 380.0_f32 } else { 220.0_f32 };
    let preferred_width = (preferred_width * scale).round() as i32;
    let available_width = total_width - left_reserve - right_reserve;
    let minimum_width = (170.0_f32 * scale).round() as i32;
    if available_width < minimum_width {
        return None;
    }
    let width = preferred_width.min(available_width);
    let left = total_width - right_reserve - width;
    Some((left, width))
}

fn find_codex_window() -> HWND {
    let mut result: HWND = ptr::null_mut();
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            (&mut result as *mut HWND) as LPARAM,
        );
    }
    result
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
        return TRUE;
    }
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let extended_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if !is_main_window_candidate(style, extended_style) {
        return TRUE;
    }
    let mut cloaked: u32 = 0;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        (&mut cloaked as *mut u32).cast(),
        size_of::<u32>() as u32,
    ) == 0
        && cloaked != 0
    {
        return TRUE;
    }

    let Some(path) = process_path_for_window(hwnd) else {
        return TRUE;
    };
    let normalized = path.to_string_lossy().to_ascii_lowercase();
    if normalized.contains("\\openai.codex_") && normalized.ends_with("\\app\\chatgpt.exe") {
        *(lparam as *mut HWND) = hwnd;
        return 0;
    }
    TRUE
}

fn is_main_window_candidate(style: u32, extended_style: u32) -> bool {
    style & WS_CAPTION != 0 && extended_style & WS_EX_TOOLWINDOW == 0
}

unsafe fn paint(hwnd: HWND) {
    let mut paint: PAINTSTRUCT = std::mem::zeroed();
    let dc = BeginPaint(hwnd, &mut paint);
    if dc.is_null() {
        return;
    }
    let mut client = empty_rect();
    GetClientRect(hwnd, &mut client);
    let dpi = GetDpiForWindow(hwnd).max(96);
    let scale: f32 = dpi as f32 / 96.0_f32;

    let (snapshot, accent, locale) = STATE
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| {
            (
                state.snapshot.clone(),
                PALETTES[state.palette_index].accent,
                state.locale,
            )
        })
        .unwrap_or((
            UsageSnapshot::default(),
            PALETTES[0].accent,
            AppLocale::English,
        ));

    let background = CreateSolidBrush(rgb(31, 31, 31));
    FillRect(dc, &client, background);
    DeleteObject(background as HGDIOBJ);

    let font_height = -((12.0_f32 * scale).round() as i32);
    let face = wide("Segoe UI");
    let font = CreateFontW(
        font_height,
        0,
        0,
        0,
        FW_NORMAL as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET.into(),
        OUT_DEFAULT_PRECIS.into(),
        0,
        0,
        (DEFAULT_PITCH | FF_DONTCARE).into(),
        face.as_ptr(),
    );
    let previous = SelectObject(dc, font as HGDIOBJ);
    SetBkMode(dc, TRANSPARENT as i32);

    let width = client.right - client.left;
    let settings_width = (24.0_f32 * scale).round() as i32;
    let content_width = (width - settings_width).max(1);
    let text_bottom = client.bottom - (5.0_f32 * scale).round() as i32;
    match (&snapshot.primary, &snapshot.weekly) {
        (Some(primary), Some(weekly)) => {
            let gap = (10.0_f32 * scale).round() as i32;
            let side = ((content_width - gap) / 2).max(1);
            draw_metric(
                dc,
                RECT {
                    left: 0,
                    top: 0,
                    right: side,
                    bottom: text_bottom,
                },
                primary,
                locale,
                accent,
                scale,
            );
            draw_metric(
                dc,
                RECT {
                    left: side + gap,
                    top: 0,
                    right: content_width,
                    bottom: text_bottom,
                },
                weekly,
                locale,
                accent,
                scale,
            );
        }
        (Some(window), None) | (None, Some(window)) => draw_metric(
            dc,
            RECT {
                left: 0,
                top: 0,
                right: content_width,
                bottom: text_bottom,
            },
            window,
            locale,
            accent,
            scale,
        ),
        (None, None) => {
            let status = wide(
                snapshot
                    .status
                    .map(|status| locale.status_text(status))
                    .unwrap_or_else(|| locale.status_text(crate::model::UsageStatus::Retrying)),
            );
            let mut status_rect = RECT {
                right: content_width,
                ..client
            };
            SetTextColor(dc, rgb(150, 150, 150));
            DrawTextW(
                dc,
                status.as_ptr(),
                -1,
                &mut status_rect,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    let mut dot = RECT {
        left: client.right - settings_width,
        top: 0,
        right: client.right,
        bottom: client.bottom,
    };
    let dot_text = wide("...");
    SetTextColor(dc, accent);
    DrawTextW(
        dc,
        dot_text.as_ptr(),
        -1,
        &mut dot,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );

    SelectObject(dc, previous);
    DeleteObject(font as HGDIOBJ);
    GdiFlush();
    EndPaint(hwnd, &paint);
}

unsafe fn draw_metric(
    dc: *mut c_void,
    rect: RECT,
    window: &LimitWindow,
    locale: AppLocale,
    accent: COLORREF,
    scale: f32,
) {
    let padding = (6.0_f32 * scale).round() as i32;
    let text = locale.metric_text(window);
    let text = wide(&text);
    let mut text_rect = RECT {
        left: rect.left + padding,
        top: rect.top,
        right: rect.right - padding,
        bottom: rect.bottom,
    };
    SetTextColor(dc, rgb(224, 224, 224));
    DrawTextW(
        dc,
        text.as_ptr(),
        -1,
        &mut text_rect,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );

    let bar_height = (2.0_f32 * scale).round().max(2.0_f32) as i32;
    let bar_top = rect.bottom - bar_height;
    let full = RECT {
        left: rect.left + padding,
        top: bar_top,
        right: rect.right - padding,
        bottom: rect.bottom,
    };
    let track = CreateSolidBrush(rgb(61, 61, 61));
    FillRect(dc, &full, track);
    GdiFlush();
    DeleteObject(track as HGDIOBJ);

    let available_width = (full.right - full.left).max(0);
    let fill_width = available_width * window.remaining_percent as i32 / 100;
    let fill = RECT {
        left: full.left,
        top: full.top,
        right: full.left + fill_width,
        bottom: full.bottom,
    };
    let brush = CreateSolidBrush(accent);
    FillRect(dc, &fill, brush);
    GdiFlush();
    DeleteObject(brush as HGDIOBJ);
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

const fn empty_rect() -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    }
}

pub(crate) fn codex_is_active() -> bool {
    CODEX_ACTIVE.load(Ordering::Relaxed)
}

pub(crate) fn update_snapshot(snapshot: UsageSnapshot) {
    let Some(state) = STATE.get() else { return };
    let hwnd = match state.lock() {
        Ok(mut state) => {
            state.snapshot = snapshot;
            state.overlay
        }
        Err(_) => return,
    };
    unsafe {
        InvalidateRect(hwnd, ptr::null(), 0);
    }
}

fn cycle_palette(hwnd: HWND) {
    let Some(state) = STATE.get() else { return };
    let palette_index = match state.lock() {
        Ok(mut state) => {
            state.palette_index = (state.palette_index + 1) % PALETTES.len();
            state.palette_index
        }
        Err(_) => return,
    };
    crate::settings::Settings { palette_index }.save();
    unsafe {
        InvalidateRect(hwnd, ptr::null(), 0);
    }
}

pub(crate) fn codex_desktop_cli_source() -> Option<PathBuf> {
    let target = STATE.get()?.lock().ok()?.target;
    if target.is_null() {
        return None;
    }
    let app_executable = unsafe { process_path_for_window(target) }?;
    let app_directory = app_executable.parent()?;
    let source = app_directory.join("resources").join("codex.exe");
    source.is_file().then_some(source)
}

unsafe fn process_path_for_window(hwnd: HWND) -> Option<PathBuf> {
    let mut process_id = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);
    if process_id == 0 {
        return None;
    }
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
    if process.is_null() {
        return None;
    }
    let mut buffer = [0_u16; 1024];
    let mut length = buffer.len() as u32;
    let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
    CloseHandle(process);
    (ok != 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_layout_is_right_aligned_and_preserves_menu_space() {
        let (left, width) = overlay_layout(1_200, 1.0, false).expect("layout");
        assert_eq!(width, 220);
        assert_eq!(left, 822);
        assert_eq!(1_200 - left - width, 158);
        assert!(left >= 220);
    }

    #[test]
    fn compact_layout_scales_with_dpi() {
        let (left, width) = overlay_layout(1_800, 1.5, false).expect("layout");
        assert_eq!(width, 330);
        assert_eq!(1_800 - left - width, 237);
    }

    #[test]
    fn compact_layout_hides_instead_of_covering_menus() {
        assert_eq!(overlay_layout(500, 1.0, false), None);
    }

    #[test]
    fn codex_main_window_is_an_attachment_candidate() {
        assert!(is_main_window_candidate(WS_CAPTION, 0));
    }

    #[test]
    fn codex_pet_tool_window_is_not_an_attachment_candidate() {
        assert!(!is_main_window_candidate(WS_CAPTION, WS_EX_TOOLWINDOW));
        assert!(!is_main_window_candidate(WS_POPUP, WS_EX_TOOLWINDOW));
    }
}
