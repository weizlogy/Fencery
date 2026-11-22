use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_MOUSEWHEEL,
    HHOOK, MSLLHOOKSTRUCT, MSG, MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
    GetCursorPos, GetWindowRect,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, GetAsyncKeyState, VK_CONTROL};
use tray_icon::{TrayIconEvent, menu::MenuEvent};
use crate::ui::group::GroupWindow;
use crate::ui::help::HelpWindow;
use crate::ui::settings::SettingsWindow;
use crate::graphics::GraphicsEngine;
use crate::settings::{manager, models::ChildSettings};
use crate::ui::group::interaction::InteractionAction;
use crate::win32::vproc::window_proc;
use crate::win32::api;

/// マウスフックで受け取ったホイール量をメインループへ渡すよ。
/// コールバックは設定ウィンドウを直接触れないので、差分だけを溜めておく。
static PENDING_WHEEL: AtomicI32 = AtomicI32::new(0);

/// 低レベルマウスフック。フォーカスに関係なくホイールを捕捉するよ。
unsafe extern "system" fn mouse_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && wparam.0 == WM_MOUSEWHEEL as usize {
        let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        // mouseData の上位 16bit がホイール量 (通常 ±120)
        let delta = ((data.mouseData >> 16) & 0xFFFF) as i16;
        PENDING_WHEEL.fetch_add(delta as i32, Ordering::Relaxed);
    }
    unsafe { CallNextHookEx(HHOOK::default(), code, wparam, lparam) }
}

/// ウィンドウメッセージとトレイイベントを処理し続けるループだよ！
pub fn run_message_loop(engine: Rc<GraphicsEngine>) -> Result<(), windows::core::Error> {
    unsafe {
        let mut msg = MSG::default();

        // 設定画面は WS_EX_NOACTIVATE でフォーカスを取れないから、
        // 低レベルマウスフックでホイールを捕捉するよ。
        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            HINSTANCE::default(),
            0,
        )?;

        let tray_channel = TrayIconEvent::receiver();
        let menu_channel = MenuEvent::receiver();

        // 1. ウィンドウクラスを1回だけ登録する
        let instance = GetModuleHandleW(None)?;
        let class_name_str = "FenceryGroupClass";
        let class_name = api::utils::to_wide(class_name_str);
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());

        api::register_class::register_window_class(
            instance.into(),
            class_pcwstr,
            Some(window_proc),
        )?;

        // 複数のグループウィンドウを管理する
        let mut windows: Vec<Box<GroupWindow>> = Vec::new();
        let mut help_window: Option<Box<HelpWindow>> = None;
        let mut settings_window: Option<Box<SettingsWindow>> = None;

        // キーの状態管理
        let mut v_was_down = false;

        // 起動時に設定から既存のグループを復元するよ
        {
            let settings = manager::get_settings_reader();
            for (id, child) in &settings.children {
                log::info!("Restoring group: {}", id);
                match GroupWindow::create(
                    engine.clone(),
                    id.clone(),
                    "Restored Group".to_string(),
                    child
                ) {
                    Ok(mut window) => {
                        windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                            window.hwnd,
                            windows::Win32::UI::WindowsAndMessaging::HWND_BOTTOM,
                            child.x,
                            child.y,
                            0,
                            0,
                            windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                        ).ok();

                        let _ = window.draw();
                        windows.push(window);
                    }
                    Err(e) => log::error!("Failed to restore group {}: {}", id, e),
                }
            }
        }

        loop {
            // 1. Win32 メッセージを全て処理する
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    let _ = UnhookWindowsHookEx(mouse_hook);
                    return Ok(());
                }

                // カスタムメッセージの処理: ウィンドウ削除通知
                if msg.message == api::WM_REMOVE_WINDOW {
                    let target_hwnd = windows::Win32::Foundation::HWND(msg.wParam.0 as isize);
                    log::info!("Removing window from management list: {:?}", target_hwnd);
                    windows.retain(|w| w.hwnd != target_hwnd);
                    if let Some(ref h) = help_window {
                        if h.hwnd == target_hwnd {
                            help_window = None;
                        }
                    }
                    if let Some(ref h) = settings_window {
                        if h.hwnd == target_hwnd {
                            settings_window = None;
                        }
                    }
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // マウスフックで溜めたホイール量を設定画面へ渡すよ。
            // フォーカスがどこにあっても届くので、表示直後だけ効くという問題は起きない。
            let wheel_delta = PENDING_WHEEL.swap(0, Ordering::Relaxed);
            if wheel_delta != 0 {
                if let Some(settings) = settings_window.as_mut() {
                    settings.handle_mouse_wheel(wheel_delta as i16);
                }
            }

            // 設定が保存されたら、フォントを使う既存ウィンドウを全部再描画するよ。
            if manager::take_redraw_request() {
                for window in &mut windows {
                    // 設定画面で変更した色・透過度・アイコンサイズを反映してから描画するよ。
                    window.reload_from_settings();
                    if let Err(e) = window.draw() {
                        log::error!("Group redraw error: {}", e);
                    }
                }
                if let Some(help) = help_window.as_mut() {
                    if let Err(e) = help.draw() {
                        log::error!("Help redraw error: {}", e);
                    }
                }
                if let Some(settings) = settings_window.as_mut() {
                    if let Err(e) = settings.draw() {
                        log::error!("Settings redraw error: {}", e);
                    }
                }
            }

            // 2. トレイアイコンのイベントを処理する
            if let Ok(event) = tray_channel.try_recv() {
                handle_tray_event(event);
            }

            // 3. メニューのイベントを処理する
            if let Ok(event) = menu_channel.try_recv() {
                handle_menu_event(event, &engine, &mut windows, &mut help_window, &mut settings_window);
            }

            // 4. キー入力を監視
            let ctrl_down = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let v_is_down = (GetAsyncKeyState(0x56) as u16 & 0x8000) != 0;

            if ctrl_down && v_is_down && !v_was_down {
                let mut pt = POINT::default();
                if GetCursorPos(&mut pt).is_ok() {
                    let mut handled = false;
                    for window in &mut windows {
                        let mut rect = RECT::default();
                        if GetWindowRect(window.hwnd, &mut rect).is_ok() {
                            if pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom {
                                let _ = window.perform_action(InteractionAction::PasteColor);
                                handled = true;
                                break;
                            }
                        }
                    }
                    // 設定画面はグループとは別に扱うよ (フォント名やサイズも貼り付けられる)。
                    if !handled {
                        if let Some(settings) = settings_window.as_mut() {
                            let mut rect = RECT::default();
                            if GetWindowRect(settings.hwnd, &mut rect).is_ok()
                                && pt.x >= rect.left && pt.x <= rect.right
                                && pt.y >= rect.top && pt.y <= rect.bottom
                            {
                                settings.perform_paste();
                            }
                        }
                    }
                }
            }
            v_was_down = v_is_down;

            MsgWaitForMultipleObjectsEx(None, 10, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }
}

fn handle_tray_event(event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click { .. } => {
            log::info!("Tray icon clicked!");
        }
        _ => {}
    }
}

fn handle_menu_event(
    event: MenuEvent,
    engine: &Rc<GraphicsEngine>,
    windows: &mut Vec<Box<GroupWindow>>,
    help_window: &mut Option<Box<HelpWindow>>,
    settings_window: &mut Option<Box<SettingsWindow>>
) {
    match event.id.0.as_str() {
        "1001" => { // New Group
            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string();
            let title = "New Group".to_string();
            let bg_color = "#000000".to_string();
            let opacity = 0.5f32;
            let icon_size = 48.0f32;
            let width = 300u32;
            let height = 200u32;

            {
                let mut settings = manager::get_settings_writer();
                settings.children.insert(id.clone(), ChildSettings {
                    x: 100, y: 100, width, height, bg_color: bg_color.clone(), opacity, icon_size, ..Default::default()
                });
                drop(settings);
                manager::save();
            }

            let settings = manager::get_settings_reader();
            match GroupWindow::create(engine.clone(), id, title, settings.children.iter().last().expect("Get newest child settings.").1) {
                Ok(mut window) => {
                    let _ = window.draw();
                    windows.push(window);
                }
                Err(e) => log::error!("Failed to create group window: {}", e),
            }
        }
        "1003" => { // Help
            if help_window.is_none() {
                let initial = {
                    let settings = manager::get_settings_reader();
                    settings
                        .windows
                        .help
                        .filter(|p| p.is_on_screen())
                        .map(|p| (p.x, p.y))
                };
                match HelpWindow::create(engine.clone(), initial) {
                    Ok(mut window) => {
                        let _ = window.draw();
                        *help_window = Some(window);
                    }
                    Err(e) => log::error!("Failed to create help window: {}", e),
                }
            } else {
                log::info!("Help window is already open.");
            }
        }
        "1004" => { // Open Settings Location
            if let Ok(path) = crate::settings::storage::get_config_path() {
                if let Some(parent) = path.parent() {
                    let _ = api::shell::open_file_location(parent);
                }
            }
        }
        "1005" => { // Settings GUI
            if settings_window.is_none() {
                let initial = {
                    let settings = manager::get_settings_reader();
                    settings
                        .windows
                        .settings
                        .filter(|p| p.is_on_screen())
                        .map(|p| (p.x, p.y))
                };
                match SettingsWindow::create(engine.clone(), initial) {
                    Ok(mut window) => {
                        let _ = window.draw();
                        *settings_window = Some(window);
                    }
                    Err(e) => log::error!("Failed to create settings window: {}", e),
                }
            } else {
                log::info!("Settings window is already open.");
            }
        }
        "1002" => { // Quit
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            }
        }
        _ => {}
    }
}
