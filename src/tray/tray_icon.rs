use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};

/// トレイメニューの項目 ID。
pub const MENU_ID_NEW_GROUP: &str = "1001";
pub const MENU_ID_HELP: &str = "1003";
pub const MENU_ID_OPEN_SETTINGS: &str = "1004";
pub const MENU_ID_OPEN_SETTINGS_GUI: &str = "1005";
pub const MENU_ID_QUIT: &str = "1002";

/// トレイアイコンを作成する。
///
/// メニューを構築し、アイコンリソースとツールチップを設定してトレイへ登録する。
///
/// # 戻り値
///
/// 登録済みのトレイアイコン。
pub fn create_tray() -> TrayIcon {
    let menu = Menu::new();
    let new_group = MenuItem::with_id("1001", "New Group", true, None);
    let help_item = MenuItem::with_id("1003", "Help", true, None);
    let open_settings_gui = MenuItem::with_id("1005", "Settings GUI", true, None);
    let open_settings = MenuItem::with_id("1004", "Open Settings Location", true, None);
    let quit_i = MenuItem::with_id("1002", "Quit", true, None);

    menu.append_items(&[
        &new_group,
        &help_item,
        &open_settings_gui,
        &open_settings,
        &PredefinedMenuItem::separator(),
        &quit_i,
    ])
    .expect("menu append.");

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Fencery")
        // 実行ファイルに埋め込まれたリソース ID 1 のアイコンを使用する。
        .with_icon(Icon::from_resource(1, None).unwrap())
        .build()
        .expect("Failed to create tray icon")
}
