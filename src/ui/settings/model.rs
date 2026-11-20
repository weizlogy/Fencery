use crate::settings::AppSettings;

pub const WINDOW_WIDTH: f32 = 450.0;
pub const WINDOW_HEIGHT: f32 = 600.0;

/// クリック判定用の矩形。描画とヒットテストで同じ値を使うよ。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

/// マウスが乗っている／押されている UI 要素だよ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverTarget {
    FontSizeSlider,
    FontSizeValue,
    FontMinus,
    FontPlus,
    FontList,
    Save,
    Reset,
}

impl HoverTarget {
    /// ホイールで増減できる数値項目かどうか。
    pub fn is_numeric(self) -> bool {
        matches!(self, Self::FontSizeSlider | Self::FontSizeValue)
    }
}

/// 現在編集中の数値項目。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericField {
    FontSize,
}

/// 固定レイアウト。ウィンドウはリサイズできないので定数で持つよ。
#[derive(Clone, Copy)]
pub struct SettingsLayout {
    pub font_size_slider: UiRect,
    pub font_minus: UiRect,
    pub font_plus: UiRect,
    pub font_size_value: UiRect,
    pub font_list: UiRect,
    pub save_button: UiRect,
    pub reset_button: UiRect,
    pub font_row_height: f32,
}

pub const LAYOUT: SettingsLayout = SettingsLayout {
    font_size_slider: UiRect::new(80.0, 106.0, 280.0, 18.0),
    font_minus: UiRect::new(368.0, 104.0, 24.0, 24.0),
    font_plus: UiRect::new(398.0, 104.0, 24.0, 24.0),
    font_size_value: UiRect::new(368.0, 78.0, 58.0, 22.0),
    font_list: UiRect::new(24.0, 204.0, 382.0, 200.0),
    save_button: UiRect::new(226.0, 510.0, 90.0, 28.0),
    reset_button: UiRect::new(326.0, 510.0, 90.0, 28.0),
    font_row_height: 25.0,
};

/// 設定画面のピュアなデータ層。DirectX には一切依存しないよ。
pub struct SettingsModel {
    /// 編集中の値。保存ボタンが押されるまでグローバル設定には反映しない。
    pub draft: AppSettings,
    /// 開いた時点の値。「元に戻す」と dirty 判定に使う。
    pub original: AppSettings,
    /// DirectWrite から列挙したインストール済みフォント。
    pub font_families: Vec<String>,
    /// リストの先頭行インデックス。
    pub list_scroll: usize,
    pub hovered: Option<HoverTarget>,
    pub pressed: Option<HoverTarget>,
    /// ドラッグ中のスライダーと、ドラッグ開始時の値・座標。
    pub active_slider: Option<(NumericField, f32, f32)>,
    /// 保存ボタン押下後の一時メッセージ。
    pub notice: Option<String>,
}

impl SettingsModel {
    pub fn new(app: &AppSettings, font_families: Vec<String>) -> Self {
        let mut model = Self {
            draft: app.clone(),
            original: app.clone(),
            font_families,
            list_scroll: 0,
            hovered: None,
            pressed: None,
            active_slider: None,
            notice: None,
        };
        model.scroll_to_current_font();
        model
    }

    /// draft が開いた時点から変更されているか。
    pub fn is_dirty(&self) -> bool {
        self.draft.font_size != self.original.font_size
            || self.draft.font_family != self.original.font_family
    }

    /// 現在選択されているフォントの list 内インデックス。
    pub fn selected_font_index(&self) -> Option<usize> {
        self.font_families
            .iter()
            .position(|name| name == &self.draft.font_family)
    }

    /// 一度に表示できる行数。
    pub fn list_visible_rows(&self) -> usize {
        (LAYOUT.font_list.h / LAYOUT.font_row_height) as usize
    }

    /// 現在のフォントが見える位置までスクロールする。
    pub fn scroll_to_current_font(&mut self) {
        if let Some(index) = self.selected_font_index() {
            let visible = self.list_visible_rows();
            if index < self.list_scroll {
                self.list_scroll = index;
            } else if index >= self.list_scroll + visible {
                self.list_scroll = index + 1 - visible;
            }
        }
    }

    /// 座標から UI 要素を特定するよ。小さいボタンから順に判定する。
    pub fn hit_test(x: f32, y: f32) -> Option<HoverTarget> {
        if LAYOUT.save_button.contains(x, y) {
            return Some(HoverTarget::Save);
        }
        if LAYOUT.reset_button.contains(x, y) {
            return Some(HoverTarget::Reset);
        }
        if LAYOUT.font_minus.contains(x, y) {
            return Some(HoverTarget::FontMinus);
        }
        if LAYOUT.font_plus.contains(x, y) {
            return Some(HoverTarget::FontPlus);
        }
        if LAYOUT.font_size_value.contains(x, y) {
            return Some(HoverTarget::FontSizeValue);
        }
        if LAYOUT.font_size_slider.contains(x, y) {
            return Some(HoverTarget::FontSizeSlider);
        }
        if LAYOUT.font_list.contains(x, y) {
            return Some(HoverTarget::FontList);
        }
        None
    }

    /// フォントリスト内の行インデックスを返す。
    pub fn font_row_at(&self, x: f32, y: f32) -> Option<usize> {
        if !LAYOUT.font_list.contains(x, y) {
            return None;
        }
        let row = ((y - LAYOUT.font_list.y) / LAYOUT.font_row_height) as usize;
        if row >= self.list_visible_rows() {
            return None;
        }
        let index = self.list_scroll + row;
        (index < self.font_families.len()).then_some(index)
    }

    pub fn change_font_size(&mut self, delta: f32) {
        self.draft.font_size = (self.draft.font_size + delta).clamp(8.0, 72.0);
        self.notice = None;
    }

    /// スライダーの x 座標からフォントサイズを設定する（1.0 刻み）。
    pub fn set_font_size_from_x(&mut self, x: f32) {
        let r = LAYOUT.font_size_slider;
        let t = ((x - r.x) / r.w).clamp(0.0, 1.0);
        self.draft.font_size = (8.0 + t * 64.0).round().clamp(8.0, 72.0);
        self.notice = None;
    }

    pub fn select_font(&mut self, index: usize) {
        if let Some(name) = self.font_families.get(index) {
            self.draft.font_family = name.clone();
            self.notice = None;
        }
    }

    /// リストを rows 行分スクロールする。
    pub fn scroll_fonts(&mut self, rows: i32) {
        let max_scroll = self.font_families.len().saturating_sub(self.list_visible_rows());
        let next = self.list_scroll as i32 + rows;
        self.list_scroll = next.clamp(0, max_scroll as i32) as usize;
    }

    pub fn reset(&mut self) {
        self.draft = self.original.clone();
        self.scroll_to_current_font();
        self.notice = Some("元に戻しました".to_string());
    }

    /// 保存前に Models 側のバリデーションを適用する。
    pub fn validate_draft(&mut self) {
        self.draft.validate();
        self.scroll_to_current_font();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> SettingsModel {
        let app = AppSettings {
            font_size: 12.0,
            font_family: "Meiryo".to_string(),
        };
        let families: Vec<String> = ["Arial", "Consolas", "Meiryo", "Yu Gothic"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        SettingsModel::new(&app, families)
    }

    #[test]
    fn hit_test_finds_buttons_and_list() {
        assert_eq!(
            SettingsModel::hit_test(380.0, 116.0),
            Some(HoverTarget::FontMinus)
        );
        assert_eq!(
            SettingsModel::hit_test(410.0, 116.0),
            Some(HoverTarget::FontPlus)
        );
        assert_eq!(SettingsModel::hit_test(270.0, 520.0), Some(HoverTarget::Save));
        assert_eq!(SettingsModel::hit_test(100.0, 220.0), Some(HoverTarget::FontList));
        assert_eq!(SettingsModel::hit_test(5.0, 5.0), None);
    }

    #[test]
    fn font_size_changes_are_clamped_and_mark_dirty() {
        let mut m = model();
        m.change_font_size(100.0);
        assert_eq!(m.draft.font_size, 72.0);
        m.change_font_size(-100.0);
        assert_eq!(m.draft.font_size, 8.0);
        assert!(m.is_dirty());
    }

    #[test]
    fn slider_maps_x_to_font_size() {
        let mut m = model();
        m.set_font_size_from_x(LAYOUT.font_size_slider.x);
        assert_eq!(m.draft.font_size, 8.0);
        m.set_font_size_from_x(LAYOUT.font_size_slider.x + LAYOUT.font_size_slider.w);
        assert_eq!(m.draft.font_size, 72.0);
    }

    #[test]
    fn selecting_and_resetting_font() {
        let mut m = model();
        assert_eq!(m.selected_font_index(), Some(2));
        m.select_font(1);
        assert_eq!(m.draft.font_family, "Consolas");
        assert!(m.is_dirty());
        m.reset();
        assert!(!m.is_dirty());
        assert_eq!(m.draft.font_family, "Meiryo");
    }

    #[test]
    fn scroll_is_bounded_by_item_count() {
        let mut m = model();
        m.scroll_fonts(-10);
        assert_eq!(m.list_scroll, 0);
        m.scroll_fonts(100);
        assert_eq!(m.list_scroll, 0); // 4 行 < 表示 8 行

        m.font_families = (0..20).map(|i| format!("F{}", i)).collect();
        m.scroll_fonts(100);
        assert_eq!(m.list_scroll, 12); // 20 - 8
    }
}