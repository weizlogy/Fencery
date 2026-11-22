use crate::graphics::{Canvas};
use crate::win32::api;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HWND,
        System::LibraryLoader::GetModuleHandleW,
    },
};

use super::vproc::window_proc;

/// ウィンドウハンドルと描画サーフェスを保持する。
pub struct Window {
    pub hwnd: HWND,
    pub canvas: Option<Canvas>, // 描画開始まで初期化を遅延する。
}

impl Window {
    /// ウィンドウクラスを登録し、新しいウィンドウを作成する。
    pub fn new(class_name_str: &str, window_name_str: &str) -> Result<Self, windows::core::Error> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class_name = api::utils::to_wide(class_name_str);
        let window_name = api::utils::to_wide(window_name_str);
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());
        let window_pcwstr = PCWSTR::from_raw(window_name.as_ptr());

        api::register_class::register_window_class(
            instance.into(),
            class_pcwstr,
            Some(window_proc),
        )?;

        let hwnd = api::create_window::create_window(
            instance.into(),
            class_pcwstr,
            window_pcwstr,
            api::create_window::WindowOptions::default(),
        )?;

        Ok(Self { hwnd, canvas: None })
    }

    /// ウィンドウを表示し、即座に再描画する。
    pub fn show(&self) {
        api::show_window::show_window(self.hwnd);
    }

    /// ウィンドウを最背面へ移動する。
    pub fn keep_bottom(&self) {
        api::show_window::move_to_bottom(self.hwnd);
    }
}
