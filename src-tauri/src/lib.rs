mod google_calendar;
mod state;

use state::AppState;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[tauri::command]
async fn save_local_events(app_handle: tauri::AppHandle, events_json: String) -> Result<(), String> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let path = data_dir.join("local_events.json");
    std::fs::write(&path, events_json).map_err(|e| format!("Failed to save events: {}", e))
}

#[tauri::command]
async fn load_local_events(app_handle: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = data_dir.join("local_events.json");
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to load events: {}", e))
    } else {
        Ok("[]".to_string())
    }
}

#[tauri::command]
async fn move_window(app_handle: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOSIZE, SWP_NOZORDER, SWP_NOACTIVATE, HWND_TOP};
        if let Some(window) = app_handle.get_webview_window("main") {
            let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
            unsafe {
                let hwnd = windows::Win32::Foundation::HWND(raw_hwnd.0);
                SetWindowPos(hwnd, Some(HWND_TOP), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_window_position(app_handle: tauri::AppHandle) -> Result<(i32, i32), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        use windows::Win32::Foundation::RECT;
        if let Some(window) = app_handle.get_webview_window("main") {
            let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
            unsafe {
                let hwnd = windows::Win32::Foundation::HWND(raw_hwnd.0);
                let mut rect = RECT::default();
                GetWindowRect(hwnd, &mut rect).map_err(|e| e.to_string())?;
                return Ok((rect.left, rect.top));
            }
        }
    }
    Err("Window not found".into())
}

#[tauri::command]
async fn get_window_rect(app_handle: tauri::AppHandle) -> Result<(i32, i32, i32, i32), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        use windows::Win32::Foundation::RECT;
        if let Some(window) = app_handle.get_webview_window("main") {
            let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
            unsafe {
                let hwnd = windows::Win32::Foundation::HWND(raw_hwnd.0);
                let mut rect = RECT::default();
                GetWindowRect(hwnd, &mut rect).map_err(|e| e.to_string())?;
                return Ok((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
            }
        }
    }
    Err("Window not found".into())
}

#[tauri::command]
async fn set_window_rect(app_handle: tauri::AppHandle, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOZORDER, SWP_NOACTIVATE, HWND_TOP};
        if let Some(window) = app_handle.get_webview_window("main") {
            let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
            unsafe {
                let hwnd = windows::Win32::Foundation::HWND(raw_hwnd.0);
                SetWindowPos(hwnd, Some(HWND_TOP), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE)
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn embed_in_desktop(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, GetWindowLongPtrW, SendMessageTimeoutW,
        SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, GWL_STYLE, SMTO_NORMAL, SW_HIDE,
        WS_CHILD, WS_POPUP, WS_CAPTION, WS_THICKFRAME,
        WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE,
        HWND_TOP, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    };
    use windows::core::{BOOL, PCWSTR};

    let raw_hwnd = window.hwnd().expect("failed to get HWND from Tauri window");

    unsafe {
        // Step 1: Find the Progman window
        let progman = FindWindowW(
            &windows::core::HSTRING::from("Progman"),
            PCWSTR::null(),
        );
        let progman = match progman {
            Ok(h) => h,
            Err(e) => {
                eprintln!("embed_in_desktop: FindWindowW(Progman) failed: {e}");
                return;
            }
        };

        // Step 2: Send 0x052C to Progman to spawn a WorkerW behind the desktop icons
        let mut _result = 0usize;
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            Some(&mut _result),
        );

        // Step 3: Find the WorkerW behind desktop icons and HIDE it
        use std::sync::Mutex;
        static WORKER_W: Mutex<Option<isize>> = Mutex::new(None);

        *WORKER_W.lock().unwrap() = None;

        unsafe extern "system" fn enum_callback(hwnd: HWND, _lparam: LPARAM) -> BOOL {
            let shell_view = FindWindowExW(
                Some(hwnd),
                None,
                &windows::core::HSTRING::from("SHELLDLL_DefView"),
                PCWSTR::null(),
            );
            if shell_view.is_ok() {
                let next_worker = FindWindowExW(
                    None,
                    Some(hwnd),
                    &windows::core::HSTRING::from("WorkerW"),
                    PCWSTR::null(),
                );
                if let Ok(w) = next_worker {
                    *WORKER_W.lock().unwrap() = Some(w.0 as isize);
                }
            }
            BOOL::from(true)
        }

        let _ = EnumWindows(Some(enum_callback), LPARAM(0));

        // Hide the WorkerW so Progman becomes the direct desktop background layer
        let worker_w_value = WORKER_W.lock().unwrap().take();
        if let Some(worker_raw) = worker_w_value {
            let worker_hwnd = HWND(worker_raw as *mut std::ffi::c_void);
            let _ = ShowWindow(worker_hwnd, SW_HIDE);
            println!("embed_in_desktop: WorkerW hidden");
        }

        // Step 4: Parent directly to Progman (immune to Win+D)
        let tauri_hwnd = HWND(raw_hwnd.0);

        // 4a: Set WS_CHILD, remove WS_POPUP/WS_CAPTION/WS_THICKFRAME
        let style = GetWindowLongPtrW(tauri_hwnd, GWL_STYLE);
        let new_style = (style
            & !(WS_POPUP.0 as isize)
            & !(WS_CAPTION.0 as isize)
            & !(WS_THICKFRAME.0 as isize))
            | WS_CHILD.0 as isize;
        SetWindowLongPtrW(tauri_hwnd, GWL_STYLE, new_style);

        // 4b: Set extended styles
        let ex_style = GetWindowLongPtrW(tauri_hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            tauri_hwnd,
            GWL_EXSTYLE,
            ex_style | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize,
        );

        // 4c: SetParent to Progman directly (not WorkerW)
        match SetParent(tauri_hwnd, Some(progman)) {
            Ok(_) => {
                println!("embed_in_desktop: successfully parented window into Progman");

                // 4d: Apply style changes with SWP_FRAMECHANGED
                let _ = SetWindowPos(
                    tauri_hwnd,
                    Some(HWND_TOP),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
                println!("embed_in_desktop: styles applied, window is now a Progman child");
            }
            Err(e) => {
                eprintln!("embed_in_desktop: SetParent to Progman failed: {e}");
            }
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            google_calendar::google_auth_start,
            google_calendar::google_auth_status,
            google_calendar::fetch_events,
            google_calendar::create_event,
            google_calendar::delete_event,
            google_calendar::update_event,
            move_window,
            get_window_position,
            get_window_rect,
            set_window_rect,
            save_local_events,
            load_local_events,
        ])
        .setup(|app| {
            setup_tray(app)?;

            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    embed_in_desktop(&window);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItemBuilder::with_id("show_hide", "Show/Hide").build(app)?;
    let google_sync =
        MenuItemBuilder::with_id("google_sync", "Google Calendar 연동").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "종료").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show_hide)
        .separator()
        .item(&google_sync)
        .separator()
        .item(&quit)
        .build()?;

    let mut tray_builder = TrayIconBuilder::new()
        .tooltip("miniCalender")
        .menu(&menu);

    // Use the app's default window icon for the tray if available
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    let _tray = tray_builder
        .on_menu_event(move |app_handle, event| {
            let id = event.id().as_ref();
            match id {
                "show_hide" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                "google_sync" => {
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = google_calendar::google_auth_start(handle).await;
                    });
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray_handle, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app_handle = tray_handle.app_handle();
                if let Some(window) = app_handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
