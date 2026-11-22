use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_SDK_VERSION,
};

/// D3D11 デバイスとデバイスコンテキストを作成する。
///
/// Direct2D との相互運用のため BGRA サポートを有効化する。
pub fn create_device() -> Result<(ID3D11Device, ID3D11DeviceContext), windows::core::Error> {
    unsafe {
        let mut device = None;
        let mut context = None;
        let mut feature_level = D3D_FEATURE_LEVEL_11_0;

        // 対応を試行する機能レベル。
        let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];

        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, // Direct2D との連携に必須。
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut feature_level),
            Some(&mut context),
        )?;

        Ok((device.unwrap(), context.unwrap()))
    }
}
