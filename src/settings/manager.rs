use std::sync::{
    atomic::{AtomicBool, Ordering},
    LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use super::models::Settings;
use super::storage;

/// 全体で共有する設定インスタンスだよ！
static GLOBAL_SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| {
    // 起動時にファイルを読み込む
    match storage::load_settings() {
        Ok(settings) => RwLock::new(settings),
        Err(e) => {
            log::error!("CRITICAL: {}", e);
            log::error!("Settings will be reset to defaults if you save. Check config.toml manualy.");
            RwLock::new(Settings::default())
        }
    }
});

/// 設定が保存された後にウィンドウを再描画するためのフラグだよ。
static REDRAW_PENDING: AtomicBool = AtomicBool::new(false);

/// 再描画要求を取得してクリアするよ。
pub fn take_redraw_request() -> bool {
    REDRAW_PENDING.swap(false, Ordering::Relaxed)
}

/// 設定値へのアクセサ関数 (読み取り用)
pub fn get_settings_reader() -> RwLockReadGuard<'static, Settings> {
    GLOBAL_SETTINGS
        .read()
        .expect("Failed to acquire read lock on settings")
}

/// 設定値へのアクセサ関数 (書き込み用)
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

/// 現在の状態をファイルに保存するよ！ (再描画も要求する)
pub fn save() {
    write_settings(true);
}

/// ウィンドウ位置だけが変わったときなど、再描画が不要な保存に使うよ。
pub fn save_quiet() {
    write_settings(false);
}
