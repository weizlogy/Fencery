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

/// フックからメインループへ渡すホイール量の累積値。
///
/// コールバックはウィンドウへ直接アクセスできないため、差分のみを保持する。
static PENDING_WHEEL: AtomicI32 = AtomicI32::new(0);

/// フォーカスに依存せずホイール入力を捕捉する低レベルフック。
extern "system" fn mouse_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && wparam.0 == WM_MOUSEWHEEL as usize {
        unsafe {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            // mouseData の上位 16 ビットがホイール量（通常 ±120）。
            let delta = ((data.mouseData >> 16) & 0xFFFF) as i16;
            PENDING_WHEEL.fetch_add(delta as i32, Ordering::Relaxed);
        }
    }
    unsafe { CallNextHookEx(HHOOK::default(), code, wparam, lparam) }
}

/// ウィンドウメッセージとトレイ／メニューイベントを処理するメインループ。
pub fn run_message_loop(engine: Rc<GraphicsEngine>) -> Result<(), windows::core::Error> {
    unsafe {
        let mut msg = MSG::default();

        // 設定ウィンドウは WS_EX_NOACTIVATE によりフォーカスを取得できないため、
        // 低レベルフックでホイールを捕捉する。
        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            HINSTANCE::default(),
            0,
        )?;

        let tray_channel = TrayIconEvent::receiver();
        let menu_channel = MenuEvent::receiver();

        // ウィンドウクラスはプロセスごとに一度だけ登録する。
        let instance = GetModuleHandleW(None)?;
        let class_name_str = "FenceryGroupClass";
        let class_name = api::utils::to_wide(class_name_str);
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());

        api::register_class::register_window_class(
            instance.into(),
            class_pcwstr,
            Some(window_proc),
        )?;

        // メッセージハンドラから参照できるよう Box で保持する。
        let mut windows: Vec<Box<GroupWindow>> = Vec::new();
        let mut help_window: Option<Box<HelpWindow>> = None;
        let mut settings_window: Option<Box<SettingsWindow>> = None;

        // キー押下の立ち上がりを検出するため前回の状態を保持する。
        let mut v_was_down = false;

        // 永続化されたグループを起動時に復元する。
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
            // 保留中の Win32 メッセージをすべて処理する。
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    let _ = UnhookWindowsHookEx(mouse_hook);
                    return Ok(());
                }

                // 破棄されたウィンドウを管理対象から外す。
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

            // 蓄積したホイール量を設定ウィンドウへ渡す。
            // フォーカスに依存しないため、表示直後のみ有効になる問題を避けられる。
            let wheel_delta = PENDING_WHEEL.swap(0, Ordering::Relaxed);
            if wheel_delta != 0 {
                if let Some(settings) = settings_window.as_mut() {
                    settings.handle_mouse_wheel(wheel_delta as i16);
                }
            }

            // 設定の変更を既存ウィンドウへ反映し、再描画する。
            if manager::take_redraw_request() {
                for window in &mut windows {
                    // 再描画の前に変更済みの外観設定を反映する。
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

            // トレイアイコンのイベントを処理する。
            if let Ok(event) = tray_channel.try_recv() {
                handle_tray_event(event);
            }

            // メニューイベントを処理する。
            if let Ok(event) = menu_channel.try_recv() {
                handle_menu_event(event, &engine, &mut windows, &mut help_window, &mut settings_window);
            }

            // キー入力を監視する。
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
                    // 設定ウィンドウはフォント名・サイズも貼り付け対象とするため別扱いとする。
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
        "1001" => { // 新しいグループを作成する。
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
        "1003" => { // ヘルプウィンドウを開く。
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
        "1004" => { // 設定ファイルの場所を開く。
            if let Ok(path) = crate::settings::storage::get_config_path() {
                if let Some(parent) = path.parent() {
                    let _ = api::shell::open_file_location(parent);
                }
            }
        }
        "1005" => { // 設定 GUI を開く。
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
        "1002" => { // アプリケーションを終了する。
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            }
        }
        _ => {}
    }
}
