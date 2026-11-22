use windows::Win32::Graphics::Imaging::{
    IWICImagingFactory, IWICBitmap, GUID_WICPixelFormat32bppPBGRA, CLSID_WICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom,
};
use windows::Win32::Graphics::Direct2D::{ID2D1DeviceContext, ID2D1Bitmap};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::WindowsAndMessaging::HICON;

/// WIC ファクトリを作成する。
pub fn create_factory() -> Result<IWICImagingFactory, windows::core::Error> {
    unsafe {
        CoCreateInstance(
            &CLSID_WICImagingFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )
    }
}

/// HICON を Direct2D ビットマップへ変換する。
pub fn create_bitmap_from_hicon(
    context: &ID2D1DeviceContext,
    wic_factory: &IWICImagingFactory,
    hicon: HICON,
) -> Result<ID2D1Bitmap, windows::core::Error> {
    unsafe {
        // HICON から WIC ビットマップを作成する。
        let wic_bitmap: IWICBitmap = wic_factory.CreateBitmapFromHICON(hicon)?;

        // HICON の形式は不定のため、32bppPBGRA へ変換してから D2D へ渡す。
        let converter = wic_factory.CreateFormatConverter()?;
        converter.Initialize(
            &wic_bitmap,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;

        // WIC ビットマップから Direct2D ビットマップを作成する。
        let d2d_bitmap = context.CreateBitmapFromWicBitmap(&converter, None)?;

        Ok(d2d_bitmap)
    }
}
