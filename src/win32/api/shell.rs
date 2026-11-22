use windows::core::{PCWSTR};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_SYSICONINDEX, SHGetImageList, SHIL_EXTRALARGE, ShellExecuteW};
use windows::Win32::UI::Controls::IImageList;
use windows::Win32::UI::WindowsAndMessaging::{HICON, SW_SHOWNORMAL};
use crate::win32::api::utils::to_wide;
use std::path::Path;

/// ファイルのアイコンをシステムイメージリストから取得する。
///
/// 返された `HICON` の所有権は呼び出し側に移る。不要になったら `DestroyIcon` で解放する。
pub fn get_icon_for_path(path: &Path) -> Option<HICON> {
    let path_str = path.to_string_lossy();
    let wide_path = to_wide(&path_str);

    let mut shfi = SHFILEINFOW::default();

    // システムイメージリスト内のアイコンインデックスを取得する。
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };

    if result == 0 {
        return None;
    }

    // イメージリストからアイコンハンドルを抽出する。
    unsafe {
        if let Ok(image_list) = SHGetImageList::<IImageList>(SHIL_EXTRALARGE as i32) {
            if let Ok(hicon) = image_list.GetIcon(shfi.iIcon, 0) {
                return Some(hicon);
            }
        }
    }

    None
}

/// 指定されたパスを関連付けられたアプリケーションで開く。
pub fn execute_path(path: &Path) -> Result<(), windows::core::Error> {
    let wide_path = to_wide(&path.to_string_lossy());
    unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR::from_raw(wide_path.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
    Ok(())
}

/// エクスプローラーでファイルを選択状態にして表示する。
pub fn open_file_location(path: &Path) -> Result<(), windows::core::Error> {
    let path_str = path.to_string_lossy();
    let arg = format!(r#"/select,"{}""#, path_str);
    let wide_arg = to_wide(&arg);
    let wide_explorer = to_wide("explorer.exe");

    unsafe {
        ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR::from_raw(wide_explorer.as_ptr()),
            PCWSTR::from_raw(wide_arg.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
    }
    Ok(())
}
