#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;
use fencery::{graphics, logger, tray, win32, settings::manager};

/// アプリケーションのエントリポイント。
///
/// コマンドライン引数を設定へ反映したのち、常駐用の基盤を初期化し、
/// メッセージループへ移行する。
fn main() -> Result<(), windows::core::Error> {
    logger::init();
    log::info!("Fencery v3.0.0 (Native) Starting...");

    // コマンドライン引数は永続化された設定より優先する。
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    let mut settings_changed = false;

    while i < args.len() {
        match args[i].as_str() {
            "--font" if i + 1 < args.len() => {
                let font_family = args[i + 1].clone();
                let mut settings = manager::get_settings_writer();
                settings.app.font_family = font_family;
                settings_changed = true;
                i += 2;
            }
            "--fsize" if i + 1 < args.len() => {
                if let Ok(size) = args[i + 1].parse::<f32>() {
                    let mut settings = manager::get_settings_writer();
                    settings.app.font_size = size;
                    settings_changed = true;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }

    if settings_changed {
        manager::save();
        log::info!("Settings updated from command line arguments.");
    }

    // WIC と DirectComposition は COM の初期化を前提とする。
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )?;
    }

    // グラフィックス基盤は全ウィンドウで単一のインスタンスを共有する。
    let engine = Rc::new(graphics::GraphicsEngine::new()?);

    // 非表示の常駐用ウィンドウ。メッセージの受け皿として機能する。
    let _window = win32::Window::new("FenceryClass", "Fencery Native")?;

    let _tray = tray::tray_icon::create_tray();
    log::info!("Tray icon created. Running in background...");

    win32::run_message_loop(engine)?;

    log::info!("Application exiting.");
    Ok(())
}
