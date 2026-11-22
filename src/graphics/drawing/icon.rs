use windows::Win32::Graphics::Direct2D::{ID2D1DeviceContext, ID2D1Bitmap};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

/// ビットマップをアイコンとして描画する。
pub fn draw_icon(
    context: &ID2D1DeviceContext,
    bitmap: &ID2D1Bitmap,
    rect: &D2D_RECT_F,
    opacity: f32,
) {
    unsafe {
        context.DrawBitmap(
            bitmap,
            Some(rect),
            opacity,
            windows::Win32::Graphics::Direct2D::D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            None, // ソース矩形を指定しない場合は全体を使用する。
        );
    }
}
