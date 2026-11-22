use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::Common::{D2D_POINT_2F};
use crate::graphics::drawing::resources::DrawingResources;

/// ヘルプに表示する操作説明。
const OPERATION_INSTRUCTIONS: [&str; 15] = [
    "## 操作説明",
    "### ■ Create Groups:",
    "  - Right-click: トレイアイコンを右クリックしてメニューを表示し New Group.",
    "  - Drag & Drop: ファイルをドラッグ＆ドロップしてグループに簡単に追加できます。",
    "### ■ Icons:",
    "  - Left-double-click: アプリケーションが起動またはファイルが開きます。",
    "  - Right-click: そのファイルがあるフォルダが開きます。",
    "  - Ctrl + Right-click: そのアイコンを削除します。",
    "### ■ Customization:",
    "  - Move: Ctrl + ドラッグ でグループを移動します。",
    "  - Resize: Shift + ドラッグ でグループのサイズを変更します。",
    "  - Color: Ctrl + V でカラーコード (#FF0000) や「#Random」を貼り付け。",
    "  - Transparency: Alt + ドラッグ で透明度を調整します。",
    "### ■ Delete Groups:",
    "  - Ctrl + Right-click: グループの何もない場所を右クリックして削除。",
];

/// 操作ガイドを描画する。
pub fn draw_help(
    context: &ID2D1DeviceContext,
    width: f32,
    _height: f32, // 他描画関数とのシグネチャ統一のため残す。
    text_color_hex: &str,
    resources: &mut DrawingResources,
) -> Result<(), windows::core::Error> {
    let settings = crate::settings::manager::get_settings_reader();
    let font_family = &settings.app.font_family;
    let base_font_size = settings.app.font_size * 1.2;
    let brush = resources.get_brush(context, text_color_hex)?;
    let format = resources.get_help_text_format(font_family, base_font_size)?;
    let dwrite_factory = resources.dwrite_factory.clone();
    drop(settings);

    let padding = 20.0;
    let mut current_y = padding;
    let max_text_width = width - padding * 2.0;

    for line in OPERATION_INSTRUCTIONS.iter() {
        let trimmed_line = line.trim();
        let mut x_offset = padding;

        // 簡易的な見出し・箇条書き記法を解析する。
        let display_text = if trimmed_line.starts_with("## ") {
            &trimmed_line[3..]
        } else if trimmed_line.starts_with("### ") {
            &trimmed_line[4..]
        } else if trimmed_line.starts_with("  - ") {
            x_offset = padding * 2.0;
            &trimmed_line[4..]
        } else {
            trimmed_line
        };

        if display_text.is_empty() {
            current_y += base_font_size;
            continue;
        }

        // TextLayout により折り返し後の高さを正確に測定する。
        let wide_text: Vec<u16> = display_text.encode_utf16().collect();
        unsafe {
            let layout = dwrite_factory.CreateTextLayout(
                &wide_text,
                &format,
                max_text_width - (x_offset - padding),
                1000.0, // 高さ制限の上限として十分な値。
            )?;

            let mut metrics = windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;

            // テキストを描画する。
            context.DrawTextLayout(
                D2D_POINT_2F { x: x_offset, y: current_y },
                &layout,
                &brush,
                windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE,
            );

            // 描画した高さだけ次の行へ進める。
            current_y += metrics.height + (base_font_size * 0.4);
        }
    }

    Ok(())
}
