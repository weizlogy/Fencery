use windows::Win32::{
    Foundation::HWND,
    Foundation::BOOL,
    Graphics::DirectComposition::{
        IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
    },
    Graphics::Dxgi::IDXGIDevice,
};

/// DXGI デバイスから DirectComposition デバイスを作成する。
pub fn create_device(dxgi_device: &IDXGIDevice) -> Result<IDCompositionDevice, windows::core::Error> {
    unsafe {
        windows::Win32::Graphics::DirectComposition::DCompositionCreateDevice(dxgi_device)
    }
}

/// ウィンドウに対してコンポジションターゲットを作成する。
pub fn create_target_for_hwnd(
    dcomp_device: &IDCompositionDevice,
    hwnd: HWND,
    topmost: bool,
) -> Result<IDCompositionTarget, windows::core::Error> {
    unsafe {
        dcomp_device.CreateTargetForHwnd(hwnd, BOOL::from(topmost))
    }
}

/// 描画レイヤーとなるビジュアルを作成する。
pub fn create_visual(
    dcomp_device: &IDCompositionDevice,
) -> Result<IDCompositionVisual, windows::core::Error> {
    unsafe {
        dcomp_device.CreateVisual()
    }
}
