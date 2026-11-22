use std::rc::Rc;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, GetWindowRect, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos,
    GWLP_USERDATA, HWND_BOTTOM, LWA_ALPHA, SM_CXSCREEN, SM_CYSCREEN,
    SWP_NOSIZE, SWP_NOACTIVATE, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE,
};
use crate::graphics::GraphicsEngine;
use crate::graphics::api::dwrite as dwrite_api;
use crate::settings::{manager, WindowPosition};
use crate::ui::settings::model::{HoverTarget, NumericField, SettingsModel, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::ui::settings::renderer::SettingsRenderer;
use crate::win32::api;

pub const SETTINGS_WINDOW_ID: &str = "SETTINGS";

#[repr(C)]
pub struct SettingsWindow {
    pub window_type: crate::ui::WindowType,
    pub hwnd: HWND,
    pub renderer: SettingsRenderer,
    pub model: SettingsModel,
    /// ウィンドウ自身の外観。設定項目ではないため保存対象外とする。
    bg_color_hex: String,
    opacity: f32,
    is_dragging: bool,
    last_mouse_pos: (i32, i32),
}

impl SettingsWindow {
    pub fn create(
        engine: Rc<GraphicsEngine>,
        initial: Option<(i32, i32)>,
    ) -> Result<Box<Self>, windows::core::Error> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class_name = api::utils::to_wide("FenceryGroupClass");
        let window_name = api::utils::to_wide("Fencery Settings");
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());
        let window_pcwstr = PCWSTR::from_raw(window_name.as_ptr());

        let width = WINDOW_WIDTH as i32;
        let height = WINDOW_HEIGHT as i32;

        // 保存位置が画面外の場合は中央付近へ配置する。
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let default_x = (screen_w - width) / 2 + 50;
        let default_y = (screen_h - height) / 2 - 50;
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
            SetLayeredWindowAttributes(
                hwnd, windows::Win32::Foundation::COLORREF(0), 255, LWA_ALPHA
            )?;
        }

        api::show_window::move_to_bottom(hwnd);

        // 設定ウィンドウ自身の外観はランダムとし、編集対象から除外する。
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bg_color_hex = format!("#{:02X}{:02X}{:02X}", rng.gen_range(50..200), rng.gen_range(50..200), rng.gen_range(50..200));
        let opacity = rng.gen_range(0.6..0.85);

        // インストール済みフォントを DirectWrite から列挙する。
        let font_families = match dwrite_api::get_installed_font_families(&engine.dwrite_factory) {
            Ok(list) => list,
            Err(e) => {
                log::error!("Failed to enumerate fonts: {}", e);
                Vec::new()
            }
        };

        // 全体設定と子設定を編集用の draft へ複製する。
        let settings = manager::get_settings_reader().clone();
        let model = SettingsModel::new(&settings, font_families);

        let renderer = SettingsRenderer::new(engine, hwnd, width as u32, height as u32)?;
        let window = Box::new(Self {
            window_type: crate::ui::WindowType::Settings,
            hwnd,
            renderer,
            model,
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
        self.renderer.render(&self.model, width, height, &self.bg_color_hex, self.opacity)
    }

    pub fn handle_lbutton_down(&mut self, x: i32, y: i32) {
        let ctrl_down = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };

        if ctrl_down {
            // Ctrl + ドラッグではウィンドウを移動する。
            self.is_dragging = true;
            self.last_mouse_pos = (x, y);
            unsafe { SetCapture(self.hwnd); }
            return;
        }

        let target = self.model.hit_test(x as f32, y as f32);
        self.model.pressed = target;

        match target {
            Some(HoverTarget::FontSizeSlider) => {
                self.model.active_slider = Some((NumericField::FontSize, self.model.draft.font_size, x as f32));
                self.model.set_font_size_from_x(x as f32);
            }
            Some(HoverTarget::ChildFontSizeSlider) => {
                let value = self.model.current_child().map(|c| c.draft.font_size).unwrap_or(12.0);
                self.model.active_slider = Some((NumericField::ChildFontSize, value, x as f32));
                self.model.child_set_font_size_from_x(x as f32);
            }
            Some(HoverTarget::ChildOpacitySlider) => {
                let value = self.model.current_child().map(|c| c.draft.opacity).unwrap_or(0.5);
                self.model.active_slider = Some((NumericField::ChildOpacity, value, x as f32));
                self.model.child_set_opacity_from_x(x as f32);
            }
            Some(HoverTarget::ChildIconSizeSlider) => {
                let value = self.model.current_child().map(|c| c.draft.icon_size).unwrap_or(48.0);
                self.model.active_slider = Some((NumericField::ChildIconSize, value, x as f32));
                self.model.child_set_icon_size_from_x(x as f32);
            }
            Some(HoverTarget::FontList) => {
                if let Some(index) = self.model.font_row_at(x as f32, y as f32) {
                    self.model.select_font(index);
                }
            }
            Some(HoverTarget::ChildFontList) => {
                if let Some(index) = self.model.child_font_row_at(x as f32, y as f32) {
                    self.model.child_select_font(index);
                }
            }
            Some(HoverTarget::ChildBgColor) => self.paste_color_to_child(),
            _ => {}
        }
        let _ = self.draw();
    }

    pub fn handle_mouse_move(&mut self, x: i32, y: i32) -> Result<(), windows::core::Error> {
        if self.is_dragging {
            let dx = x - self.last_mouse_pos.0;
            let dy = y - self.last_mouse_pos.1;

            let mut rect = RECT::default();
            unsafe {
                GetWindowRect(self.hwnd, &mut rect)?;
                SetWindowPos(self.hwnd, HWND_BOTTOM, rect.left + dx, rect.top + dy, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE)?;
            }
            return Ok(());
        }

        // スライダー操作中は x 座標から値を更新する。
        if let Some((field, _, _)) = self.model.active_slider {
            match field {
                NumericField::FontSize => self.model.set_font_size_from_x(x as f32),
                NumericField::ChildFontSize => self.model.child_set_font_size_from_x(x as f32),
                NumericField::ChildOpacity => self.model.child_set_opacity_from_x(x as f32),
                NumericField::ChildIconSize => self.model.child_set_icon_size_from_x(x as f32),
            }
            self.draw()?;
            return Ok(());
        }

        let hovered = self.model.hit_test(x as f32, y as f32);
        if hovered != self.model.hovered {
            self.model.hovered = hovered;
            self.draw()?;
        }
        Ok(())
    }

    pub fn handle_lbutton_up(&mut self) {
        let was_dragging = self.is_dragging;
        if self.is_dragging {
            self.is_dragging = false;
            unsafe { ReleaseCapture().ok(); }
        }
        self.model.active_slider = None;

        match self.model.pressed.take() {
            Some(HoverTarget::PagePrev) => self.model.prev_page(),
            Some(HoverTarget::PageNext) => self.model.next_page(),
            Some(HoverTarget::FontMinus) => self.model.change_font_size(-1.0),
            Some(HoverTarget::FontPlus) => self.model.change_font_size(1.0),
            Some(HoverTarget::ChildFontMinus) => self.model.child_change_font_size(-1.0),
            Some(HoverTarget::ChildFontPlus) => self.model.child_change_font_size(1.0),
            Some(HoverTarget::Save) => self.save(),
            Some(HoverTarget::Reset) => self.model.reset(),
            _ => {}
        }
        let _ = self.draw();
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
            settings.windows.settings = Some(WindowPosition { x: rect.left, y: rect.top });
        }
        manager::save_quiet();
    }

    /// ホイールで数値・リストを操作する。`Shift` 押下時は変化量を 5 倍にする。
    ///
    /// `hovered` は `WM_MOUSEMOVE` 時のみ更新されるため、ここではカーソル座標から
    /// 直接ヒットテストする。カーソルを動かさずにホイール操作した場合も有効とする。
    pub fn handle_mouse_wheel(&mut self, delta: i16) {
        let mut pt = POINT::default();
        let mut rect = RECT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() || GetWindowRect(self.hwnd, &mut rect).is_err() {
                return;
            }
        }
        let x = (pt.x - rect.left) as f32;
        let y = (pt.y - rect.top) as f32;
        let target = self.model.hit_test(x, y);

        let is_shift = unsafe { (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 };
        let dir = if delta > 0 { -1 } else { 1 };
        let rows = if is_shift { 5 } else { 1 };

        match target {
            Some(HoverTarget::FontSizeSlider | HoverTarget::FontSizeValue) => {
                self.model.change_font_size(dir as f32 * rows as f32);
            }
            Some(HoverTarget::FontList) => {
                self.model.scroll_fonts(dir * rows);
            }
            Some(HoverTarget::ChildFontSizeSlider | HoverTarget::ChildFontSizeValue) => {
                self.model.child_change_font_size(dir as f32 * rows as f32);
            }
            Some(HoverTarget::ChildFontList) => {
                self.model.child_scroll_fonts(dir * rows);
            }
            Some(HoverTarget::ChildOpacitySlider) => {
                self.model.child_change_opacity(dir as f32 * 0.01 * rows as f32);
            }
            Some(HoverTarget::ChildIconSizeSlider) => {
                self.model.child_change_icon_size(dir as f32 * 4.0 * rows as f32);
            }
            _ => return,
        }
        let _ = self.draw();
    }

    /// ページに応じた値を `Ctrl` + `V` で貼り付ける。
    pub fn perform_paste(&mut self) {
        let Some(text_raw) = api::utils::get_clipboard_text() else { return };
        let text = text_raw.trim();

        if self.model.page == 0 {
            // メインページではフォント名とサイズを受け付ける。
            if let Some(name) = text.strip_prefix("font:") {
                let name = name.trim().to_string();
                if !name.is_empty() {
                    self.model.draft.font_family = name;
                    self.model.notice = None;
                    let _ = self.draw();
                }
                return;
            }
            if let Some(size) = text.strip_prefix("size:") {
                if let Ok(size) = size.trim().parse::<f32>() {
                    self.model.change_font_size(size - self.model.draft.font_size);
                    let _ = self.draw();
                }
            }
        } else {
            // 子ページではフォント・サイズ・色を受け付ける。
            if let Some(name) = text.strip_prefix("font:") {
                let name = name.trim().to_string();
                if !name.is_empty() {
                    if let Some(child) = self.model.current_child_mut() {
                        child.draft.font_family = name;
                    }
                    self.model.notice = None;
                    let _ = self.draw();
                }
                return;
            }
            if let Some(size) = text.strip_prefix("size:") {
                if let Ok(size) = size.trim().parse::<f32>() {
                    let current = self.model.current_child().map(|c| c.draft.font_size).unwrap_or(12.0);
                    self.model.child_change_font_size(size - current);
                    let _ = self.draw();
                }
                return;
            }
            self.paste_color_to_child();
        }
    }

    /// クリップボードの色を現在の子ウィンドウへ適用する。
    fn paste_color_to_child(&mut self) {
        let Some(text_raw) = api::utils::get_clipboard_text() else { return };
        let mut hex = text_raw.trim().to_string();
        if hex.eq_ignore_ascii_case("#random") {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            hex = format!("#{:02X}{:02X}{:02X}", rng.r#gen::<u8>(), rng.r#gen::<u8>(), rng.r#gen::<u8>());
        }
        self.model.child_set_bg_color(hex);
        let _ = self.draw();
    }

    /// draft をグローバル設定へ反映し、設定ファイルへ保存する。
    fn save(&mut self) {
        self.model.validate_draft();
        let mut rect = RECT::default();
        let position = if unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_ok() {
            Some(WindowPosition { x: rect.left, y: rect.top })
        } else {
            None
        };
        {
            // ここでロックを解放しないと `manager::save()` の read lock 取得時にデッドロックする。
            let mut settings = manager::get_settings_writer();
            settings.app = self.model.draft.clone();
            for entry in &self.model.children {
                if let Some(child) = settings.children.get_mut(&entry.id) {
                    *child = entry.draft.clone();
                }
            }
            if let Some(position) = position {
                settings.windows.settings = Some(position);
            }
        }
        manager::save();
        self.model.original = self.model.draft.clone();
        for entry in &mut self.model.children {
            entry.original = entry.draft.clone();
        }
        self.model.notice = Some("保存しました".to_string());
    }

    pub fn handle_rbutton_down(&mut self) -> Result<(), windows::core::Error> {
        Ok(())
    }

    pub fn handle_rbutton_up(&mut self) -> Result<(), windows::core::Error> {
        let ctrl_down = unsafe { (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };

        if ctrl_down {
            self.save_position();
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    windows::Win32::Foundation::HWND(0),
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
