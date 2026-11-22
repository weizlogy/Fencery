use std::rc::Rc;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowLongPtrW, SetWindowPos, GWLP_USERDATA, HWND_BOTTOM, SWP_NOACTIVATE,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
    GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};
use crate::graphics::GraphicsEngine;
use crate::settings::{manager, WindowPosition};
use crate::ui::help::renderer::HelpRenderer;
use crate::win32::api;

pub const HELP_WINDOW_ID: &str = "HELP_GUIDE";

#[repr(C)]
pub struct HelpWindow {
    pub window_type: crate::ui::WindowType,
    pub hwnd: HWND,
    pub renderer: HelpRenderer,
    pub bg_color_hex: String,
    pub opacity: f32,
    pub is_dragging: bool,
    pub last_mouse_pos: (i32, i32),
}

impl HelpWindow {
    pub fn create(
        engine: Rc<GraphicsEngine>,
        initial: Option<(i32, i32)>,
    ) -> Result<Box<Self>, windows::core::Error> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class_name = api::utils::to_wide("FenceryGroupClass");
        let window_name = api::utils::to_wide("Fencery Help");
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());
        let window_pcwstr = PCWSTR::from_raw(window_name.as_ptr());

        let width = 450;
        let height = 600;

        // 保存位置が画面外の場合は中央付近へ配置する。
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let default_x = (screen_w - width) / 2 + 50; // 中央より右へずらす。
        let default_y = (screen_h - height) / 2 - 50; // 中央より上へずらす。
        let (x, y) = match initial {
            Some((x, y)) if WindowPosition { x, y }.is_on_screen() => (x, y),
            _ => (default_x, default_y),
        };

        let options = api::create_window::WindowOptions {
            x, y, width, height,
            ex_style: Some(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0x00200000)
            ),
            style: Some(WS_POPUP | WS_VISIBLE),
            ..Default::default()
        };

        let hwnd = api::create_window::create_window(instance.into(), class_pcwstr, window_pcwstr, options)?;

        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                hwnd, windows::Win32::Foundation::COLORREF(0), 255, windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA
            )?;
        }

        api::show_window::move_to_bottom(hwnd);

        // 表示ごとに異なる外観を与える。
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bg_color_hex = format!("#{:02X}{:02X}{:02X}", rng.gen_range(50..200), rng.gen_range(50..200), rng.gen_range(50..200));
        let opacity = rng.gen_range(0.6..0.85);

        let renderer = HelpRenderer::new(engine, hwnd, width as u32, height as u32)?;
        let window = Box::new(Self {
            window_type: crate::ui::WindowType::Help,
            hwnd,
            renderer,
            bg_color_hex,
            opacity,
            is_dragging: false,
            last_mouse_pos: (0, 0),
        });

        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*window as *const Self as isize);
        }

        Ok(window)
    }

    pub fn draw(&mut self) -> Result<(), windows::core::Error> {
        let mut rect = RECT::default();
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(self.hwnd, &mut rect)?; }
        let width = (rect.right - rect.left) as f32;
        let height = (rect.bottom - rect.top) as f32;
        self.renderer.render(width, height, &self.bg_color_hex, self.opacity)
    }

    pub fn handle_lbutton_down(&mut self, x: i32, y: i32) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
        let ctrl_down = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };

        if ctrl_down {
            self.is_dragging = true;
            self.last_mouse_pos = (x, y);
            unsafe { windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(self.hwnd); }
        }
    }

    pub fn handle_mouse_move(&mut self, x: i32, y: i32) -> Result<(), windows::core::Error> {
        if self.is_dragging {
            let dx = x - self.last_mouse_pos.0;
            let dy = y - self.last_mouse_pos.1;

            let mut rect = RECT::default();
            unsafe {
                GetWindowRect(self.hwnd, &mut rect)?;
                SetWindowPos(self.hwnd, HWND_BOTTOM, rect.left + dx, rect.top + dy, 0, 0, windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE | SWP_NOACTIVATE)?;
            }
        }
        Ok(())
    }

    pub fn handle_lbutton_up(&mut self) {
        let was_dragging = self.is_dragging;
        self.is_dragging = false;
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture().ok(); }
        if was_dragging {
            self.save_position();
        }
    }

    /// 現在の表示位置を設定ファイルへ保存する。
    ///
    /// 位置のみの変更であるため再描画は不要。
    fn save_position(&self) {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_err() {
            return;
        }
        {
            let mut settings = manager::get_settings_writer();
            settings.windows.help = Some(WindowPosition { x: rect.left, y: rect.top });
        }
        manager::save_quiet();
    }

    pub fn handle_rbutton_down(&mut self) -> Result<(), windows::core::Error> {
        // 削除処理はボタン解放時に行う。
        Ok(())
    }

    pub fn handle_rbutton_up(&mut self) -> Result<(), windows::core::Error> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
        let ctrl_down = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };

        if ctrl_down {
            // 閉じる前に最終位置を保存する。
            self.save_position();
            // 解放時に閉じることでデスクトップのコンテキストメニュー表示を避ける。
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    windows::Win32::Foundation::HWND(0), // スレッドメッセージとして送信する。
                    api::WM_REMOVE_WINDOW,
                    windows::Win32::Foundation::WPARAM(self.hwnd.0 as usize),
                    windows::Win32::Foundation::LPARAM(0),
                ).ok();
                windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd).ok();
            }
        }
        Ok(())
    }
}
