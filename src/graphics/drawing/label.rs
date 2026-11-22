use windows::Win32::Graphics::Direct2D::{ID2D1DeviceContext, ID2D1SolidColorBrush};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::DirectWrite::IDWriteTextFormat;

/// テキストを描画する。
pub fn draw_text(
    context: &ID2D1DeviceContext,
    text: &str,
    rect: &D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
    format: &IDWriteTextFormat,
) {
    // DirectWrite 向けに UTF-16 へ変換する。
    let wide_text: Vec<u16> = text.encode_utf16().collect();

    unsafe {
        context.DrawText(
            &wide_text,
            format,
            rect,
            brush,
            windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE,
            windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL,
        );
    }
}
