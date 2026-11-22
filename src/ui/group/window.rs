use crate::graphics::GraphicsEngine;
use crate::ui::group::interaction::{InteractionAction, InteractionHandler};
use crate::ui::group::model::GroupModel;
use crate::ui::group::renderer::GroupRenderer;
use crate::win32::api;
use crate::settings::{ChildSettings, manager};
use std::rc::Rc;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowLongPtrW, SetWindowPos, GWLP_USERDATA, HWND_BOTTOM, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE, WS_EX_ACCEPTFILES, SetTimer, KillTimer,
};

/// 実行直後のハイライトを解除するタイマーの ID。
const IDT_EXECUTE_FLASH: usize = 1;
/// グループウィンドウの状態と操作を統括する。
#[repr(C)]
pub struct GroupWindow {
    pub window_type: crate::ui::WindowType,
    pub hwnd: HWND,
    pub model: GroupModel,
    pub renderer: GroupRenderer,
    pub interaction: InteractionHandler,
}

impl GroupWindow {
    /// 設定からグループウィンドウを生成する。
    pub fn create(
        engine: Rc<GraphicsEngine>,
        id: String,
        title: String,
        child: &ChildSettings,
    ) -> Result<Box<Self>, windows::core::Error> {
        let instance = unsafe { GetModuleHandleW(None)? };

        let class_name_str = "FenceryGroupClass";
        let class_name = api::utils::to_wide(class_name_str);
        let window_name = api::utils::to_wide(&title);
        let class_pcwstr = PCWSTR::from_raw(class_name.as_ptr());
        let window_pcwstr = PCWSTR::from_raw(window_name.as_ptr());

        const WS_EX_NOREDIRECTIONBITMAP: windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE =
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0x00200000);

        let options = api::create_window::WindowOptions {
            width: child.width as i32,
            height: child.height as i32,
            ex_style: Some(
                WS_EX_LAYERED
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_NOREDIRECTIONBITMAP
                    | WS_EX_ACCEPTFILES,
            ),
            style: Some(WS_POPUP | WS_VISIBLE),
            ..Default::default()
        };

        let hwnd = api::create_window::create_window(
            instance.into(),
            class_pcwstr,
            window_pcwstr,
            options,
        )?;

        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                255,
                windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA,
            )?;
        }

        api::show_window::move_to_bottom(hwnd);

        let icons = child.icons.iter().map(|i| i.path.clone()).collect();
        // 子固有のフォント設定が未指定なら親の設定を継承する。
        let (font_size, font_family) = {
            let settings = manager::get_settings_reader();
            (
                if child.font_size > 0.0 { child.font_size } else { settings.app.font_size },
                if !child.font_family.is_empty() {
                    child.font_family.clone()
                } else {
                    settings.app.font_family.clone()
                },
            )
        };
        let model = GroupModel::new(
            id,
            title,
            child.bg_color.clone(),
            child.opacity,
            child.icon_size,
            font_size,
            font_family,
            icons,
        );
        let renderer = GroupRenderer::new(engine, hwnd, child.width, child.height)?;
        let interaction = InteractionHandler::new();

        let window = Box::new(Self {
            window_type: crate::ui::WindowType::Group,
            hwnd,
            model,
            renderer,
            interaction,
        });

        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*window as *const Self as isize);
        }

        Ok(window)
    }

    /// 保存済みの子設定をモデルへ反映する。
    pub fn reload_from_settings(&mut self) {
        let settings = manager::get_settings_reader();
        if let Some(child) = settings.children.get(&self.model.id) {
            self.model.bg_color_hex = child.bg_color.clone();
            self.model.opacity = child.opacity;
            self.model.icon_size = child.icon_size;
            self.model.font_size = if child.font_size > 0.0 {
                child.font_size
            } else {
                settings.app.font_size
            };
            self.model.font_family = if !child.font_family.is_empty() {
                child.font_family.clone()
            } else {
                settings.app.font_family.clone()
            };
        }
    }

    pub fn draw(&mut self) -> Result<(), windows::core::Error> {
        let mut rect = RECT::default();
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(self.hwnd, &mut rect)?; }
        let width = (rect.right - rect.left) as f32;
        let height = (rect.bottom - rect.top) as f32;

        let is_resizing = self.interaction.is_resizing();
        self.renderer.render(&self.model, width, height, is_resizing)
    }

    pub fn handle_resize(&mut self, width: u32, height: u32) -> Result<(), windows::core::Error> {
        self.renderer.resize(width, height)
    }

    pub fn handle_lbutton_down(&mut self) {
        let font_size = self.model.font_size;
        self.interaction.handle_lbutton_down(self.hwnd, self.model.icons.len(), self.model.icon_size, font_size);
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(self.hwnd); }
    }

    pub fn handle_lbutton_dblclk(&mut self) -> Result<(), windows::core::Error> {
        let font_size = self.model.font_size;
        let action = self.interaction.handle_lbutton_dblclk(self.hwnd, self.model.icons.len(), self.model.icon_size, font_size);
        self.perform_action(action)
    }

    pub fn handle_rbutton_down(&mut self) -> Result<(), windows::core::Error> {
        let font_size = self.model.font_size;
        let action = self.interaction.handle_rbutton_down(self.hwnd, self.model.icons.len(), self.model.icon_size, font_size);
        self.perform_action(action)
    }

    pub fn handle_rbutton_up(&mut self) -> Result<(), windows::core::Error> {
        let font_size = self.model.font_size;
        let action = self.interaction.handle_rbutton_up(self.hwnd, self.model.icons.len(), self.model.icon_size, font_size);
        self.perform_action(action)
    }

    pub fn handle_mouse_move(&mut self) -> Result<(), windows::core::Error> {
        let font_size = self.model.font_size;
        let action = self.interaction.handle_mouse_move(self.hwnd, self.model.icons.len(), self.model.icon_size, font_size);
        self.perform_action(action)
    }

    pub fn handle_mouse_wheel(&mut self, delta: i16) -> Result<(), windows::core::Error> {
        let action = self.interaction.handle_mouse_wheel(delta);
        self.perform_action(action)
    }

    pub fn handle_keydown(&mut self, virtual_key: u16) -> Result<(), windows::core::Error> {
        let action = self.interaction.handle_keydown(virtual_key);
        self.perform_action(action)
    }

    /// タイマー発火時に実行フラグを解除する。
    pub fn handle_timer(&mut self, timer_id: usize) -> Result<(), windows::core::Error> {
        if timer_id == IDT_EXECUTE_FLASH {
            self.model.executing_index = None;
            unsafe { KillTimer(self.hwnd, IDT_EXECUTE_FLASH).ok(); }
            self.draw()?;
        }
        Ok(())
    }

    pub fn perform_action(&mut self, action: InteractionAction) -> Result<(), windows::core::Error> {
        match action {
            InteractionAction::Move { dx, dy } => {
                let mut rect = RECT::default();
                unsafe {
                    GetWindowRect(self.hwnd, &mut rect)?;
                    let new_x = rect.left + dx;
                    let new_y = rect.top + dy;
                    SetWindowPos(self.hwnd, HWND_BOTTOM, new_x, new_y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE)?;

                    let mut settings = manager::get_settings_writer();
                    if let Some(child) = settings.children.get_mut(&self.model.id) {
                        child.x = new_x; child.y = new_y;
                        drop(settings);
                        manager::save();
                    }
                }
            }
            InteractionAction::Resize { dw, dh } => {
                let mut rect = RECT::default();
                unsafe {
                    GetWindowRect(self.hwnd, &mut rect)?;
                    let new_width = ((rect.right - rect.left) + dw).max(50);
                    let new_height = ((rect.bottom - rect.top) + dh).max(50);
                    SetWindowPos(self.hwnd, HWND_BOTTOM, 0, 0, new_width, new_height, SWP_NOMOVE | SWP_NOACTIVATE)?;

                    let mut settings = manager::get_settings_writer();
                    if let Some(child) = settings.children.get_mut(&self.model.id) {
                        child.width = new_width as u32; child.height = new_height as u32;
                        drop(settings);
                        manager::save();
                    }
                }
                self.draw()?;
            }
            InteractionAction::ChangeOpacity { delta } | InteractionAction::ChangeOpacityContinuous { delta } => {
                self.model.opacity = (self.model.opacity + delta).clamp(0.1, 1.0);
                let mut settings = manager::get_settings_writer();
                if let Some(child) = settings.children.get_mut(&self.model.id) {
                    child.opacity = self.model.opacity;
                    drop(settings);
                    manager::save();
                }
                self.draw()?;
            }
            InteractionAction::PasteColor => {
                if let Some(text_raw) = api::utils::get_clipboard_text() {
                    let text = text_raw.trim().to_lowercase();

                    // アイコンサイズ指定（例: size:64）を解析する。
                    if text.starts_with("size:") {
                        if let Ok(size) = text["size:".len()..].parse::<f32>() {
                            return self.perform_action(InteractionAction::ChangeIconSize { size });
                        }
                    }

                    // 背景色指定（#RRGGBB, #random）を解析する。
                    let mut hex = text_raw.trim().to_string();
                    if hex.to_lowercase() == "#random" {
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        hex = format!("#{:02X}{:02X}{:02X}", rng.r#gen::<u8>(), rng.r#gen::<u8>(), rng.r#gen::<u8>());
                    }
                    if (hex.len() == 7 || hex.len() == 9) && hex.starts_with('#') {
                        self.model.bg_color_hex = hex.clone();
                        let mut settings = manager::get_settings_writer();
                        if let Some(child) = settings.children.get_mut(&self.model.id) {
                            child.bg_color = hex;
                            drop(settings);
                            manager::save();
                        }
                        self.draw()?;
                    }
                }
            }
            InteractionAction::ChangeIconSize { size } => {
                self.model.icon_size = size.clamp(16.0, 256.0);
                let mut settings = manager::get_settings_writer();
                if let Some(child) = settings.children.get_mut(&self.model.id) {
                    child.icon_size = self.model.icon_size;
                    drop(settings);
                    manager::save();
                }
                self.draw()?;
            }
            InteractionAction::ExecuteIcon { index } => {
                // 先にパスを取得してモデルへの可変借用を避ける。
                let maybe_path = self.model.icons.get(index).map(|i| i.path.clone());

                if let Some(path) = maybe_path {
                    self.model.executing_index = Some(index);
                    self.draw()?;

                    unsafe { SetTimer(self.hwnd, IDT_EXECUTE_FLASH, 150, None); }

                    log::info!("Executing: {:?}", path);
                    api::shell::execute_path(&path)?;
                }
            }
            InteractionAction::OpenLocation { index } => {
                let icon_path = self.model.icons.get(index).map(|i| i.path.clone());
                if let Some(path) = icon_path {
                    log::info!("Opening location: {:?}", path);
                    api::shell::open_file_location(&path)?;
                }
            }
            InteractionAction::ReorderIcon { from, to } => {
                if from < self.model.icons.len() && to < self.model.icons.len() {
                    self.model.icons.swap(from, to);
                    let mut settings = manager::get_settings_writer();
                    if let Some(child) = settings.children.get_mut(&self.model.id) {
                        child.icons.swap(from, to);
                        drop(settings);
                        manager::save();
                    }
                    self.draw()?;
                }
            }
            InteractionAction::DeleteIcon { index } => {
                if index < self.model.icons.len() {
                    self.model.icons.remove(index);
                    let mut settings = manager::get_settings_writer();
                    if let Some(child) = settings.children.get_mut(&self.model.id) {
                        child.icons.remove(index);
                        drop(settings);
                        manager::save();
                    }
                    self.draw()?;
                }
            }
            InteractionAction::DeleteGroup => {
                let mut settings = manager::get_settings_writer();
                settings.children.remove(&self.model.id);
                drop(settings);
                manager::save();
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
            InteractionAction::HoverChanged { index } => {
                self.model.hovered_index = index;
                self.draw()?;
            }
            InteractionAction::None => {}
        }
        Ok(())
    }

    pub fn handle_drop_files(&mut self, paths: Vec<std::path::PathBuf>) -> Result<(), windows::core::Error> {
        for path in paths {
            let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string();
            self.model.icons.push(crate::ui::group::model::IconState { name, path: path.clone(), exists: true });
            let mut settings = manager::get_settings_writer();
            if let Some(child) = settings.children.get_mut(&self.model.id) {
                child.icons.push(crate::settings::models::PersistentIconInfo { path: path.clone() });
                drop(settings);
                manager::save();
            }
        }
        self.draw()
    }

    pub fn handle_lbutton_up(&mut self) {
        self.interaction.handle_lbutton_up();
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture().ok(); }
    }
}
