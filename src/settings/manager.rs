use std::sync::{
    atomic::{AtomicBool, Ordering},
    LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use super::models::Settings;
use super::storage;

/// プロセス全体で共有する設定インスタンス。
static GLOBAL_SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| {
    // 初回アクセス時に設定ファイルを読み込む。
    match storage::load_settings() {
        Ok(settings) => RwLock::new(settings),
        Err(e) => {
            log::error!("CRITICAL: {}", e);
            log::error!("Settings will be reset to defaults if you save. Check config.toml manualy.");
            RwLock::new(Settings::default())
        }
    }
});

/// 保存後にウィンドウへ再描画を要求するためのフラグ。
static REDRAW_PENDING: AtomicBool = AtomicBool::new(false);

/// 再描画要求を取得し、フラグをクリアする。
pub fn take_redraw_request() -> bool {
    REDRAW_PENDING.swap(false, Ordering::Relaxed)
}

/// 設定への読み取り専用アクセスを取得する。
pub fn get_settings_reader() -> RwLockReadGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .read()
        .expect("Failed to acquire read lock on settings")
}

/// 設定への書き込みアクセスを取得する。
pub fn get_settings_writer() -> RwLockWriteGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .write()
        .expect("Failed to acquire write lock on settings")
}

fn write_settings(flag_redraw: bool) {
    let settings = get_settings_reader();
    match storage::save_settings(&*settings) {
        Ok(()) => {
            if flag_redraw {
                REDRAW_PENDING.store(true, Ordering::Relaxed);
            }
        }
        Err(e) => log::error!("Failed to save settings: {}", e),
    }
}

/// 現在の状態を保存し、再描画を要求する。
pub fn save() {
    write_settings(true);
}

/// 位置変更など再描画が不要な保存に使用する。
pub fn save_quiet() {
    write_settings(false);
}
