use windows::Win32::Graphics::Direct2D::Common::{D2D_POINT_2F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_NONE, ID2D1DeviceContext, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_TEXT_METRICS, IDWriteFactory1, IDWriteTextFormat,
};

use crate::graphics::drawing::resources::DrawingResources;
use crate::ui::settings::model::{HoverTarget, SettingsModel, UiRect, LAYOUT};

fn to_rect(r: UiRect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.x,
        top: r.y,
        right: r.x + r.w,
        bottom: r.y + r.h,
    }
}

/// 1 行のテキストを描画して、その高さを返すよ。
fn draw_line(
    factory: &IDWriteFactory1,
    context: &ID2D1DeviceContext,
    format: &IDWriteTextFormat,
    brush: &ID2D1SolidColorBrush,
    text: &str,
    x: f32,
    y: f32,
    max_width: f32,
) -> Result<f32, windows::core::Error> {
    if text.is_empty() {
        return Ok(0.0);
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let layout = factory.CreateTextLayout(&wide, format, max_width, 1000.0)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        layout.GetMetrics(&mut metrics)?;
        context.DrawTextLayout(
            D2D_POINT_2F { x, y },
            &layout,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        Ok(metrics.height)
    }
}

fn fill_rect(
    context: &ID2D1DeviceContext,
    r: UiRect,
    brush: &ID2D1SolidColorBrush,
) {
    unsafe { context.FillRectangle(&to_rect(r), brush) };
}

fn stroke_rect(
    context: &ID2D1DeviceContext,
    r: UiRect,
    brush: &ID2D1SolidColorBrush,
    width: f32,
) {
    unsafe { context.DrawRectangle(&to_rect(r), brush, width, None) };
}

/// 設定画面の UI を描画するよ。背景は呼び出し側が先に描画しておく。
pub fn draw_settings(
    context: &ID2D1DeviceContext,
    width: f32,
    _height: f32,
    model: &SettingsModel,
    resources: &mut DrawingResources,
    bg_color_hex: &str,
    text_color_hex: &str,
) -> Result<(), windows::core::Error> {
    // ブラシとフォーマットを先に全部作って、あとは resources を触らないよ。
    // (DrawingResources の text format キャッシュは 1 つしかないため)
    let bg_brush = resources.get_brush(context, bg_color_hex)?;
    let bg_color = unsafe { bg_brush.GetColor() };
    let is_dark = crate::graphics::layout::is_dark_color(bg_color.r, bg_color.g, bg_color.b);
    let muted_hex = if is_dark { "#FFFFFF99" } else { "#00000099" };
    let panel_hex = if is_dark { "#FFFFFF18" } else { "#00000018" };
    let border_hex = if is_dark { "#FFFFFF44" } else { "#00000044" };

    let text_brush = resources.get_brush(context, text_color_hex)?;
    let muted_brush = resources.get_brush(context, muted_hex)?;
    let panel_brush = resources.get_brush(context, panel_hex)?;
    let border_brush = resources.get_brush(context, border_hex)?;
    let accent_brush = resources.get_brush(context, "#4A90E2")?;
    let selected_text_brush = resources.get_brush(context, "#FFFFFF")?;

    let ui_format = resources.get_help_text_format("Meiryo", 13.0)?;
    let title_format = resources.get_help_text_format("Meiryo", 18.0)?;
    let dwrite_factory = resources.dwrite_factory.clone();

    // プレビューだけ draft のフォント・サイズを使うよ。
    let family_wide = crate::win32::api::utils::to_wide(&model.draft.font_family);
    let preview_format: IDWriteTextFormat = unsafe {
        dwrite_factory.CreateTextFormat(
            windows::core::PCWSTR::from_raw(family_wide.as_ptr()),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            model.draft.font_size,
            windows::core::w!("ja-jp"),
        )?
    };
    unsafe {
        preview_format.SetTextAlignment(
            windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        preview_format.SetParagraphAlignment(
            windows::Win32::Graphics::DirectWrite::DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
        )?;
    }

    let left = 24.0;
    let content_width = width - left * 2.0;

    // タイトルと区切り線
    draw_line(
        &dwrite_factory,
        context,
        &title_format,
        &accent_brush,
        "Fencery Settings",
        left,
        12.0,
        content_width,
    )?;
    stroke_rect(
        context,
        UiRect::new(left, 40.0, content_width, 1.0),
        &border_brush,
        1.0,
    );

    // ── フォント ────────────────────────────────
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        &accent_brush,
        "フォント",
        left,
        48.0,
        content_width,
    )?;
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        &text_brush,
        "サイズ",
        left,
        78.0,
        60.0,
    )?;
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        &text_brush,
        &format!("{:.1} pt", model.draft.font_size),
        LAYOUT.font_size_value.x,
        78.0,
        LAYOUT.font_size_value.w,
    )?;

    // スライダー (トラック + 塗り + つまみ)
    let track = UiRect::new(
        LAYOUT.font_size_slider.x,
        LAYOUT.font_size_slider.y + 7.0,
        LAYOUT.font_size_slider.w,
        4.0,
    );
    fill_rect(context, track, &panel_brush);
    let font_ratio = (model.draft.font_size - 8.0) / 64.0;
    fill_rect(
        context,
        UiRect::new(track.x, track.y, track.w * font_ratio, track.h),
        &accent_brush,
    );
    fill_rect(
        context,
        UiRect::new(
            track.x + track.w * font_ratio - 3.0,
            LAYOUT.font_size_slider.y - 1.0,
            6.0,
            20.0,
        ),
        &accent_brush,
    );

    // ± ボタン
    for (r, label, target) in [
        (LAYOUT.font_minus, "−", HoverTarget::FontMinus),
        (LAYOUT.font_plus, "+", HoverTarget::FontPlus),
    ] {
        let hovered = model.hovered == Some(target);
        fill_rect(context, r, if hovered { &accent_brush } else { &panel_brush });
        stroke_rect(context, r, &border_brush, 1.0);
        draw_line(
            &dwrite_factory,
            context,
            &ui_format,
            if hovered { &selected_text_brush } else { &text_brush },
            label,
            r.x + 7.0,
            r.y + 1.0,
            r.w,
        )?;
    }

    // プレビュー枠
    let preview_rect = UiRect::new(left, 134.0, content_width, 34.0);
    fill_rect(context, preview_rect, &panel_brush);
    stroke_rect(context, preview_rect, &border_brush, 1.0);
    draw_line(
        &dwrite_factory,
        context,
        &preview_format,
        &text_brush,
        "Fencery",
        left + 8.0,
        137.0,
        content_width - 16.0,
    )?;

    // フォント一覧
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        &text_brush,
        "ファミリー",
        left,
        180.0,
        content_width,
    )?;
    fill_rect(context, LAYOUT.font_list, &panel_brush);
    stroke_rect(context, LAYOUT.font_list, &border_brush, 1.0);

    let visible = model.list_visible_rows();
    let selected = model.selected_font_index();
    for row in 0..visible {
        let index = model.list_scroll + row;
        let Some(name) = model.font_families.get(index) else { break };
        let row_rect = UiRect::new(
            LAYOUT.font_list.x + 1.0,
            LAYOUT.font_list.y + 1.0 + row as f32 * LAYOUT.font_row_height,
            LAYOUT.font_list.w - 2.0,
            LAYOUT.font_row_height - 1.0,
        );
        let is_selected = selected == Some(index);
        if is_selected {
            fill_rect(context, row_rect, &accent_brush);
        }
        let label = if is_selected {
            format!("{}  ✓", name)
        } else {
            name.clone()
        };
        draw_line(
            &dwrite_factory,
            context,
            &ui_format,
            if is_selected { &selected_text_brush } else { &text_brush },
            &label,
            row_rect.x + 6.0,
            row_rect.y + 2.0,
            row_rect.w - 12.0,
        )?;
    }

    // 操作ヒント (2 行に分けてボタンと重ならないようにするよ)
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        &muted_brush,
        "Ctrl+ドラッグ: 移動 / Ctrl+右クリック: 閉じる",
        left,
        444.0,
        content_width,
    )?;
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        &muted_brush,
        "ホイール: 調整 / Shift: 粗調整 / Ctrl+V: 貼り付け",
        left,
        464.0,
        content_width,
    )?;

    // 保存・元に戻す
    let save_hovered = model.hovered == Some(HoverTarget::Save);
    fill_rect(
        context,
        LAYOUT.save_button,
        if save_hovered { &accent_brush } else { &panel_brush },
    );
    stroke_rect(context, LAYOUT.save_button, &border_brush, 1.0);
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        if save_hovered { &selected_text_brush } else { &text_brush },
        "保存",
        LAYOUT.save_button.x + 30.0,
        LAYOUT.save_button.y + 2.0,
        LAYOUT.save_button.w,
    )?;

    let reset_hovered = model.hovered == Some(HoverTarget::Reset);
    fill_rect(
        context,
        LAYOUT.reset_button,
        if reset_hovered { &accent_brush } else { &panel_brush },
    );
    stroke_rect(context, LAYOUT.reset_button, &border_brush, 1.0);
    draw_line(
        &dwrite_factory,
        context,
        &ui_format,
        if reset_hovered { &selected_text_brush } else { &text_brush },
        "元に戻す",
        LAYOUT.reset_button.x + 16.0,
        LAYOUT.reset_button.y + 2.0,
        LAYOUT.reset_button.w,
    )?;

    if let Some(notice) = &model.notice {
        draw_line(
            &dwrite_factory,
            context,
            &ui_format,
            &accent_brush,
            notice,
            left,
            550.0,
            content_width,
        )?;
    }

    Ok(())
}
