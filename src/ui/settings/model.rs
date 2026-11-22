use crate::settings::{AppSettings, ChildSettings, Settings};

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
    PagePrev,
    PageNext,
    FontSizeSlider,
    FontSizeValue,
    FontMinus,
    FontPlus,
    FontList,
    ChildFontSizeSlider,
    ChildFontSizeValue,
    ChildFontMinus,
    ChildFontPlus,
    ChildFontList,
    ChildBgColor,
    ChildOpacitySlider,
    ChildIconSizeSlider,
    Save,
    Reset,
}

/// 現在編集中の数値項目。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericField {
    FontSize,
    ChildFontSize,
    ChildOpacity,
    ChildIconSize,
}

/// 固定レイアウト。ウィンドウはリサイズできないので定数で持つよ。
/// フォント周りの座標はメイン・子で共通、下の追項目だけ子ページ専用だよ。
#[derive(Clone, Copy)]
pub struct SettingsLayout {
    pub nav_prev: UiRect,
    pub nav_next: UiRect,
    pub font_size_slider: UiRect,
    pub font_minus: UiRect,
    pub font_plus: UiRect,
    pub font_size_value: UiRect,
    pub font_list: UiRect,
    pub child_bg_color: UiRect,
    pub child_opacity_slider: UiRect,
    pub child_icon_size_slider: UiRect,
    pub save_button: UiRect,
    pub reset_button: UiRect,
    pub font_row_height: f32,
}

pub const LAYOUT: SettingsLayout = SettingsLayout {
    nav_prev: UiRect::new(24.0, 10.0, 22.0, 24.0),
    nav_next: UiRect::new(106.0, 10.0, 22.0, 24.0),
    font_size_slider: UiRect::new(80.0, 106.0, 280.0, 18.0),
    font_minus: UiRect::new(368.0, 104.0, 24.0, 24.0),
    font_plus: UiRect::new(398.0, 104.0, 24.0, 24.0),
    font_size_value: UiRect::new(368.0, 78.0, 58.0, 22.0),
    font_list: UiRect::new(24.0, 204.0, 382.0, 200.0),
    child_bg_color: UiRect::new(110.0, 410.0, 150.0, 26.0),
    child_opacity_slider: UiRect::new(110.0, 442.0, 216.0, 18.0),
    child_icon_size_slider: UiRect::new(110.0, 472.0, 216.0, 18.0),
    save_button: UiRect::new(226.0, 510.0, 90.0, 28.0),
    reset_button: UiRect::new(326.0, 510.0, 90.0, 28.0),
    font_row_height: 25.0,
};

/// 子ウィンドウ 1 つ分の編集状態。
pub struct ChildDraft {
    pub id: String,
    pub draft: ChildSettings,
    pub original: ChildSettings,
}

/// 設定画面のピュアなデータ層。DirectX には一切依存しないよ。
pub struct SettingsModel {
    /// 編集中の全体設定。保存ボタンが押されるまでグローバル設定には反映しない。
    pub draft: AppSettings,
    /// 開いた時点の値。「元に戻す」と dirty 判定に使う。
    pub original: AppSettings,
    /// 子ウィンドウの編集状態。ID 順にソートしてある。
    pub children: Vec<ChildDraft>,
    /// 0 = メイン、1.. = children[page - 1]。
    pub page: usize,
    /// DirectWrite から列挙したインストール済みフォント。
    pub font_families: Vec<String>,
    /// メインページのフォントリストの先頭行。
    pub list_scroll: usize,
    /// 子ページのフォントリストの先頭行。
    pub child_font_list_scroll: usize,
    pub hovered: Option<HoverTarget>,
    pub pressed: Option<HoverTarget>,
    /// ドラッグ中のスライダーと、ドラッグ開始時の値・座標。
    pub active_slider: Option<(NumericField, f32, f32)>,
    /// 保存ボタン押下後の一時メッセージ。
    pub notice: Option<String>,
}

impl SettingsModel {
    pub fn new(settings: &Settings, font_families: Vec<String>) -> Self {
        let mut children: Vec<ChildDraft> = settings
            .children
            .iter()
            .map(|(id, child)| ChildDraft {
                id: id.clone(),
                draft: child.clone(),
                original: child.clone(),
            })
            .collect();
        // HashMap の順番は不定なので、ID 順に並べてページを安定させるよ。
        children.sort_by(|a, b| a.id.cmp(&b.id));

        let mut model = Self {
            draft: settings.app.clone(),
            original: settings.app.clone(),
            children,
            page: 0,
            font_families,
            list_scroll: 0,
            child_font_list_scroll: 0,
            hovered: None,
            pressed: None,
            active_slider: None,
            notice: None,
        };
        model.scroll_to_current_font();
        model.scroll_to_current_child_font();
        model
    }

    /// メイン + 子ウィンドウのページ数。
    pub fn page_count(&self) -> usize {
        1 + self.children.len()
    }

    pub fn current_child(&self) -> Option<&ChildDraft> {
        self.page.checked_sub(1).and_then(|i| self.children.get(i))
    }

    pub fn current_child_mut(&mut self) -> Option<&mut ChildDraft> {
        let index = self.page.checked_sub(1)?;
        self.children.get_mut(index)
    }

    /// ページを切り替えたら、ドラッグやホバーの状態をリセットするよ。
    pub fn set_page(&mut self, page: usize) {
        self.page = page.min(self.page_count() - 1);
        self.hovered = None;
        self.pressed = None;
        self.active_slider = None;
        self.notice = None;
        self.scroll_to_current_font();
        self.scroll_to_current_child_font();
    }

    pub fn next_page(&mut self) {
        if self.page + 1 < self.page_count() {
            self.set_page(self.page + 1);
        }
    }

    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.set_page(self.page - 1);
        }
    }

    /// draft が開いた時点から変更されているか。
    pub fn is_dirty(&self) -> bool {
        self.draft.font_size != self.original.font_size
            || self.draft.font_family != self.original.font_family
            || self.children.iter().any(|c| c.draft != c.original)
    }

    /// フォントリストに一度に表示できる行数。
    pub fn list_visible_rows(&self) -> usize {
        (LAYOUT.font_list.h / LAYOUT.font_row_height) as usize
    }

    // ── メインページ ────────────────────────────────

    pub fn selected_font_index(&self) -> Option<usize> {
        self.font_families
            .iter()
            .position(|name| name == &self.draft.font_family)
    }

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

    pub fn font_row_at(&self, x: f32, y: f32) -> Option<usize> {
        row_at(&LAYOUT.font_list, self.list_scroll, x, y, self.list_visible_rows(), self.font_families.len())
    }

    pub fn change_font_size(&mut self, delta: f32) {
        self.draft.font_size = (self.draft.font_size + delta).clamp(8.0, 72.0);
        self.notice = None;
    }

    pub fn set_font_size_from_x(&mut self, x: f32) {
        self.draft.font_size = font_size_from_x(x, &LAYOUT.font_size_slider);
        self.notice = None;
    }

    pub fn select_font(&mut self, index: usize) {
        if let Some(name) = self.font_families.get(index) {
            self.draft.font_family = name.clone();
            self.notice = None;
        }
    }

    pub fn scroll_fonts(&mut self, rows: i32) {
        self.list_scroll = scroll_bounded(
            self.list_scroll,
            rows,
            self.font_families.len(),
            self.list_visible_rows(),
        );
    }

    // ── 子ページ ────────────────────────────────────

    pub fn child_selected_font_index(&self) -> Option<usize> {
        let family = &self.current_child()?.draft.font_family;
        self.font_families.iter().position(|name| name == family)
    }

    pub fn scroll_to_current_child_font(&mut self) {
        if let Some(index) = self.child_selected_font_index() {
            let visible = self.list_visible_rows();
            if index < self.child_font_list_scroll {
                self.child_font_list_scroll = index;
            } else if index >= self.child_font_list_scroll + visible {
                self.child_font_list_scroll = index + 1 - visible;
            }
        }
    }

    pub fn child_font_row_at(&self, x: f32, y: f32) -> Option<usize> {
        row_at(
            &LAYOUT.font_list,
            self.child_font_list_scroll,
            x,
            y,
            self.list_visible_rows(),
            self.font_families.len(),
        )
    }

    pub fn child_change_font_size(&mut self, delta: f32) {
        if let Some(child) = self.current_child_mut() {
            child.draft.font_size = (child.draft.font_size + delta).clamp(8.0, 72.0);
        }
        self.notice = None;
    }

    pub fn child_set_font_size_from_x(&mut self, x: f32) {
        let size = font_size_from_x(x, &LAYOUT.font_size_slider);
        if let Some(child) = self.current_child_mut() {
            child.draft.font_size = size;
        }
        self.notice = None;
    }

    pub fn child_select_font(&mut self, index: usize) {
        let Some(name) = self.font_families.get(index).cloned() else { return };
        if let Some(child) = self.current_child_mut() {
            child.draft.font_family = name;
        }
        self.notice = None;
    }

    pub fn child_scroll_fonts(&mut self, rows: i32) {
        self.child_font_list_scroll = scroll_bounded(
            self.child_font_list_scroll,
            rows,
            self.font_families.len(),
            self.list_visible_rows(),
        );
    }

    pub fn child_change_opacity(&mut self, delta: f32) {
        if let Some(child) = self.current_child_mut() {
            child.draft.opacity = (child.draft.opacity + delta).clamp(0.1, 1.0);
        }
        self.notice = None;
    }

    pub fn child_set_opacity_from_x(&mut self, x: f32) {
        let r = LAYOUT.child_opacity_slider;
        let t = ((x - r.x) / r.w).clamp(0.0, 1.0);
        if let Some(child) = self.current_child_mut() {
            child.draft.opacity = (0.1 + t * 0.9).clamp(0.1, 1.0);
        }
        self.notice = None;
    }

    pub fn child_change_icon_size(&mut self, delta: f32) {
        if let Some(child) = self.current_child_mut() {
            child.draft.icon_size = (child.draft.icon_size + delta).clamp(16.0, 256.0);
        }
        self.notice = None;
    }

    pub fn child_set_icon_size_from_x(&mut self, x: f32) {
        let r = LAYOUT.child_icon_size_slider;
        let t = ((x - r.x) / r.w).clamp(0.0, 1.0);
        if let Some(child) = self.current_child_mut() {
            child.draft.icon_size = (16.0 + t * 240.0).round().clamp(16.0, 256.0);
        }
        self.notice = None;
    }

    pub fn child_set_bg_color(&mut self, hex: String) {
        if (hex.len() == 7 || hex.len() == 9) && hex.starts_with('#') {
            if let Some(child) = self.current_child_mut() {
                child.draft.bg_color = hex;
            }
            self.notice = None;
        }
    }

    /// 座標から UI 要素を特定するよ。小さいボタンから順に判定する。
    pub fn hit_test(&self, x: f32, y: f32) -> Option<HoverTarget> {
        // ページナビとフッターはどのページでも共通。
        if LAYOUT.nav_prev.contains(x, y) {
            return Some(HoverTarget::PagePrev);
        }
        if LAYOUT.nav_next.contains(x, y) {
            return Some(HoverTarget::PageNext);
        }
        if LAYOUT.save_button.contains(x, y) {
            return Some(HoverTarget::Save);
        }
        if LAYOUT.reset_button.contains(x, y) {
            return Some(HoverTarget::Reset);
        }

        let is_child = self.page > 0;
        if LAYOUT.font_minus.contains(x, y) {
            return Some(if is_child { HoverTarget::ChildFontMinus } else { HoverTarget::FontMinus });
        }
        if LAYOUT.font_plus.contains(x, y) {
            return Some(if is_child { HoverTarget::ChildFontPlus } else { HoverTarget::FontPlus });
        }
        if LAYOUT.font_size_value.contains(x, y) {
            return Some(if is_child { HoverTarget::ChildFontSizeValue } else { HoverTarget::FontSizeValue });
        }
        if LAYOUT.font_size_slider.contains(x, y) {
            return Some(if is_child { HoverTarget::ChildFontSizeSlider } else { HoverTarget::FontSizeSlider });
        }
        if LAYOUT.font_list.contains(x, y) {
            return Some(if is_child { HoverTarget::ChildFontList } else { HoverTarget::FontList });
        }
        if is_child {
            if LAYOUT.child_bg_color.contains(x, y) {
                return Some(HoverTarget::ChildBgColor);
            }
            if LAYOUT.child_opacity_slider.contains(x, y) {
                return Some(HoverTarget::ChildOpacitySlider);
            }
            if LAYOUT.child_icon_size_slider.contains(x, y) {
                return Some(HoverTarget::ChildIconSizeSlider);
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.draft = self.original.clone();
        for child in &mut self.children {
            child.draft = child.original.clone();
        }
        self.scroll_to_current_font();
        self.scroll_to_current_child_font();
        self.notice = Some("元に戻しました".to_string());
    }

    /// 保存前に Models 側のバリデーションを適用する。
    pub fn validate_draft(&mut self) {
        self.draft.validate();
        for child in &mut self.children {
            child.draft.validate(&self.draft);
        }
        self.scroll_to_current_font();
        self.scroll_to_current_child_font();
    }
}

// ── 小さなヘルパー ────────────────────────────────

fn font_size_from_x(x: f32, r: &UiRect) -> f32 {
    let t = ((x - r.x) / r.w).clamp(0.0, 1.0);
    (8.0 + t * 64.0).round().clamp(8.0, 72.0)
}

fn row_at(
    rect: &UiRect,
    scroll: usize,
    x: f32,
    y: f32,
    visible: usize,
    count: usize,
) -> Option<usize> {
    if !rect.contains(x, y) {
        return None;
    }
    let row = ((y - rect.y) / LAYOUT.font_row_height) as usize;
    if row >= visible {
        return None;
    }
    let index = scroll + row;
    (index < count).then_some(index)
}

fn scroll_bounded(current: usize, rows: i32, count: usize, visible: usize) -> usize {
    let max = count.saturating_sub(visible);
    (current as i32 + rows).clamp(0, max as i32) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> SettingsModel {
        let mut settings = Settings {
            app: AppSettings {
                font_size: 12.0,
                font_family: "Meiryo".to_string(),
            },
            ..Default::default()
        };
        let mut child = ChildSettings::default();
        child.bg_color = "#112233".to_string();
        child.font_size = 14.0;
        child.font_family = "Consolas".to_string();
        settings.children.insert("child-1".to_string(), child);
        settings.children.insert("child-2".to_string(), ChildSettings::default());

        let families: Vec<String> = ["Arial", "Consolas", "Meiryo", "Yu Gothic"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        SettingsModel::new(&settings, families)
    }

    #[test]
    fn hit_test_finds_buttons_and_list() {
        let m = model();
        assert_eq!(m.hit_test(35.0, 22.0), Some(HoverTarget::PagePrev));
        assert_eq!(m.hit_test(116.0, 22.0), Some(HoverTarget::PageNext));
        assert_eq!(m.hit_test(380.0, 116.0), Some(HoverTarget::FontMinus));
        assert_eq!(m.hit_test(270.0, 520.0), Some(HoverTarget::Save));
        assert_eq!(m.hit_test(100.0, 220.0), Some(HoverTarget::FontList));
        assert_eq!(m.hit_test(5.0, 5.0), None);
    }

    #[test]
    fn main_font_edits_are_clamped_and_resettable() {
        let mut m = model();
        m.change_font_size(100.0);
        assert_eq!(m.draft.font_size, 72.0);
        m.select_font(1);
        assert_eq!(m.draft.font_family, "Consolas");
        assert!(m.is_dirty());
        m.reset();
        assert!(!m.is_dirty());
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
    fn paging_navigates_all_children() {
        let mut m = model();
        assert_eq!(m.page_count(), 3);
        m.next_page();
        assert_eq!(m.page, 1);
        assert_eq!(m.current_child().unwrap().id, "child-1");
        m.next_page();
        m.next_page();
        assert_eq!(m.page, 2);
        m.prev_page();
        assert_eq!(m.page, 1);
    }

    #[test]
    fn child_has_independent_font_and_extras() {
        let mut m = model();
        m.next_page();

        assert_eq!(m.current_child().unwrap().draft.font_family, "Consolas");
        assert_eq!(m.child_selected_font_index(), Some(1));

        m.child_change_font_size(2.0);
        m.child_set_bg_color("#ABCDEF".to_string());
        m.child_change_opacity(0.2);
        m.child_change_icon_size(8.0);

        let child = &m.current_child().unwrap().draft;
        assert_eq!(child.font_size, 16.0);
        assert_eq!(child.bg_color, "#ABCDEF");
        assert!(child.opacity > 0.1);
        assert_eq!(child.icon_size, 56.0);
        assert!(m.is_dirty());

        assert_eq!(m.draft.font_family, "Meiryo"); // メインは変更されない

        m.reset();
        assert!(!m.is_dirty());
        assert_eq!(m.current_child().unwrap().draft.font_family, "Consolas");
    }

    #[test]
    fn child_hit_test_switches_with_page() {
        let m = model();
        assert_eq!(m.hit_test(150.0, 85.0), None); // 子の色はメインでは判定しない
        let mut m = m;
        m.next_page();
        assert_eq!(m.hit_test(380.0, 116.0), Some(HoverTarget::ChildFontMinus));
        assert_eq!(m.hit_test(150.0, 420.0), Some(HoverTarget::ChildBgColor));
        assert_eq!(m.hit_test(150.0, 450.0), Some(HoverTarget::ChildOpacitySlider));
        assert_eq!(m.hit_test(150.0, 480.0), Some(HoverTarget::ChildIconSizeSlider));
    }

    #[test]
    fn scroll_is_bounded_by_item_count() {
        let mut m = model();
        m.scroll_fonts(-10);
        assert_eq!(m.list_scroll, 0);
        m.font_families = (0..20).map(|i| format!("F{}", i)).collect();
        m.scroll_fonts(100);
        assert_eq!(m.list_scroll, 12);
    }
}