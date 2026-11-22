use std::path::PathBuf;

/// グループウィンドウの状態を保持する。
///
/// 描画 API に依存しない純粋なデータ層。
pub struct GroupModel {
    pub id: String,
    pub title: String,
    pub bg_color_hex: String,
    pub opacity: f32, // 不透明度（0.0〜1.0）
    pub icon_size: f32,
    /// 子ウィンドウ固有のフォント設定。未指定時は親の設定が入る。
    pub font_size: f32,
    pub font_family: String,
    pub icons: Vec<IconState>,
    pub hovered_index: Option<usize>,
    pub executing_index: Option<usize>, // 実行直後のハイライト表示に使用する。
}

#[derive(Clone)]
pub struct IconState {
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
}

impl GroupModel {
    pub fn new(
        id: String,
        title: String,
        bg_color_hex: String,
        opacity: f32,
        icon_size: f32,
        font_size: f32,
        font_family: String,
        initial_icons: Vec<PathBuf>,
    ) -> Self {
        let icons = initial_icons
            .into_iter()
            .map(|path| {
                let name = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let exists = path.exists();
                IconState { name, path, exists }
            })
            .collect();

        Self {
            id,
            title,
            bg_color_hex,
            opacity,
            icon_size,
            font_size,
            font_family,
            icons,
            hovered_index: None,
            executing_index: None,
        }
    }
}
