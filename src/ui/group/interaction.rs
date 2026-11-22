use windows::Win32::Foundation::{POINT, RECT, HWND};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT, VK_MENU};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
use crate::graphics::layout;

/// ユーザー操作から変換された抽象的なアクション。
pub enum InteractionAction {
    None,
    Move { dx: i32, dy: i32 },   // 前回位置からの移動量。
    Resize { dw: i32, dh: i32 }, // 前回位置からのサイズ変化量。
    ChangeOpacity { delta: f32 }, // 不透明度の段階的な変化量。
    ChangeOpacityContinuous { delta: f32 }, // ドラッグによる連続的な変化量。
    ChangeIconSize { size: f32 }, // アイコンの論理サイズを直接指定する。
    PasteColor,                  // クリップボードからの貼り付け要求。
    ExecuteIcon { index: usize }, // アイコンの実行。
    DeleteIcon { index: usize },  // アイコンの削除。
    OpenLocation { index: usize }, // ファイルの場所を開く。
    ReorderIcon { from: usize, to: usize }, // アイコンの並び替え。
    DeleteGroup,                 // グループ自体の削除。
    HoverChanged { index: Option<usize> }, // ホバー対象の変更。
}

/// マウス・キー入力を抽象アクションへ変換する。
pub struct InteractionHandler {
    last_screen_pos: Option<POINT>,
    is_dragging: bool,
    is_resizing: bool,
    is_adjusting_opacity: bool,
    dragged_icon_index: Option<usize>, // ドラッグ中のアイコン位置。
    hovered_index: Option<usize>, // ホバー中のアイコン位置。
}

impl InteractionHandler {
    pub fn new() -> Self {
        Self {
            last_screen_pos: None,
            is_dragging: false,
            is_resizing: false,
            is_adjusting_opacity: false,
            dragged_icon_index: None,
            hovered_index: None,
        }
    }

    /// カーソル位置のアイコンを特定する。
    fn hit_test(hwnd: HWND, icon_count: usize, icon_size: f32, font_size: f32) -> Option<usize> {
        let mut pt = POINT::default();
        let mut rect = RECT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() || GetWindowRect(hwnd, &mut rect).is_err() {
                return None;
            }
        }

        let rel_x = (pt.x - rect.left) as f32;
        let rel_y = (pt.y - rect.top) as f32;
        let width = (rect.right - rect.left) as f32;

        let layouts = layout::calculate_grid_layout(width, icon_count, icon_size, font_size, 1.0);
        for (i, layout) in layouts.iter().enumerate() {
            if rel_x >= layout.hit_rect.left && rel_x <= layout.hit_rect.right &&
               rel_y >= layout.hit_rect.top && rel_y <= layout.hit_rect.bottom {
                return Some(i);
            }
        }

        None
    }

    /// 左ボタン押下時に操作状態を遷移させる。
    pub fn handle_lbutton_down(&mut self, hwnd: HWND, icon_count: usize, icon_size: f32, font_size: f32) {
        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }

        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        let is_shift = unsafe { (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 };
        let is_alt = unsafe { (GetKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 };

        if is_ctrl {
            self.is_dragging = true;
        } else if is_shift {
            self.is_resizing = true;
        } else if is_alt {
            self.is_adjusting_opacity = true;
        } else {
            // 修飾キーがない場合はアイコンの並び替えを開始する。
            self.dragged_icon_index = Self::hit_test(hwnd, icon_count, icon_size, font_size);
        }

        self.last_screen_pos = Some(pt);
    }

    /// ダブルクリック時にアイコンの実行を要求する。
    pub fn handle_lbutton_dblclk(&self, hwnd: HWND, icon_count: usize, icon_size: f32, font_size: f32) -> InteractionAction {
        if let Some(index) = Self::hit_test(hwnd, icon_count, icon_size, font_size) {
            return InteractionAction::ExecuteIcon { index };
        }
        InteractionAction::None
    }

    /// 右ボタン押下時は操作を確定させない。
    pub fn handle_rbutton_down(&self, _hwnd: HWND, _icon_count: usize, _icon_size: f32, _font_size: f32) -> InteractionAction {
        // 削除・場所を開く判定はボタン解放時に行う。
        InteractionAction::None
    }

    /// 右ボタン解放時に修飾キーに応じた操作を決定する。
    pub fn handle_rbutton_up(&self, hwnd: HWND, icon_count: usize, icon_size: f32, font_size: f32) -> InteractionAction {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let is_ctrl = unsafe { (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        let hit_index = Self::hit_test(hwnd, icon_count, icon_size, font_size);

        match (hit_index, is_ctrl) {
            (Some(index), true) => InteractionAction::DeleteIcon { index },
            (None, true) => InteractionAction::DeleteGroup,
            (Some(index), false) => InteractionAction::OpenLocation { index },
            _ => InteractionAction::None,
        }
    }

    /// マウス移動からホバー・ドラッグ操作を生成する。
    pub fn handle_mouse_move(&mut self, hwnd: HWND, icon_count: usize, icon_size: f32, font_size: f32) -> InteractionAction {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() {
                return InteractionAction::None;
            }
        }

        // ホバー状態を更新する。
        let new_hover = Self::hit_test(hwnd, icon_count, icon_size, font_size);
        if new_hover != self.hovered_index {
            self.hovered_index = new_hover;
            
            // ドラッグ中に別アイコンへ入った時点で並び替えを発行する。
            if let (Some(from), Some(to)) = (self.dragged_icon_index, new_hover) {
                if from != to {
                    self.dragged_icon_index = Some(to); // 追跡位置を現在のアイコンへ進める。
                    return InteractionAction::ReorderIcon { from, to };
                }
            }
            
            // 他の操作中はホバー変更のみを通知する。
            if !self.is_dragging && !self.is_resizing && !self.is_adjusting_opacity && self.dragged_icon_index.is_none() {
                return InteractionAction::HoverChanged { index: new_hover };
            }
        }

        // ドラッグ操作の移動量を計算する。
        if let Some(last_pos) = self.last_screen_pos {
            let dx = pt.x - last_pos.x;
            let dy = pt.y - last_pos.y;

            if dx == 0 && dy == 0 {
                return InteractionAction::None;
            }

            self.last_screen_pos = Some(pt);

            if self.is_dragging {
                return InteractionAction::Move { dx, dy };
            } else if self.is_resizing {
                return InteractionAction::Resize { dw: dx, dh: dy };
            } else if self.is_adjusting_opacity {
                return InteractionAction::ChangeOpacityContinuous { delta: dx as f32 * 0.005 };
            }
        }
        InteractionAction::None
    }

    pub fn handle_mouse_wheel(&self, delta: i16) -> InteractionAction {
        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        if is_ctrl {
            let step = 0.05;
            let delta_f = if delta > 0 { step } else { -step };
            return InteractionAction::ChangeOpacity { delta: delta_f };
        }
        InteractionAction::None
    }

    pub fn handle_keydown(&self, virtual_key: u16) -> InteractionAction {
        let is_ctrl = unsafe { (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 };
        if is_ctrl && virtual_key == 'V' as u16 {
            return InteractionAction::PasteColor;
        }
        InteractionAction::None
    }

    pub fn handle_lbutton_up(&mut self) {
        self.is_dragging = false;
        self.is_resizing = false;
        self.is_adjusting_opacity = false;
        self.dragged_icon_index = None;
        self.last_screen_pos = None;
    }

    pub fn is_dragging(&self) -> bool { self.is_dragging }
    pub fn is_resizing(&self) -> bool { self.is_resizing }
    pub fn is_adjusting_opacity(&self) -> bool { self.is_adjusting_opacity }
}
