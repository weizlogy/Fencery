use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::UpdateWindow,
    UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW,
    },
};

/// ウィンドウを表示し、即座に再描画する。
pub fn show_window(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
    }
}

/// ウィンドウを最背面へ移動する。
pub fn move_to_bottom(hwnd: HWND) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND_BOTTOM,
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}
