use std::rc::Rc;
use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct2D::{ID2D1DeviceContext, ID2D1Bitmap1},
        DirectComposition::{IDCompositionTarget, IDCompositionVisual},
        Dxgi::IDXGISwapChain1,
    },
};
use crate::graphics::{api, GraphicsEngine};

/// ウィンドウごとの描画領域を管理する。
///
/// バッファを予め大きめに確保し、リサイズ時の再確保を減らす。
pub struct Canvas {
    engine: Rc<GraphicsEngine>,
    swap_chain: IDXGISwapChain1,
    pub d2d_context: ID2D1DeviceContext,
    pub comp_target: IDCompositionTarget,
    pub comp_visual: IDCompositionVisual,
    buffer_width: u32,
    buffer_height: u32,
}

impl Canvas {
    pub fn new(
        engine: Rc<GraphicsEngine>,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> Result<Self, windows::core::Error> {
        // 初期サイズにも余裕を持たせる。
        let buffer_width = width + 200;
        let buffer_height = height + 200;

        let swap_chain = api::dxgi::create_swap_chain_for_composition(
            &engine.d3d_device,
            buffer_width,
            buffer_height,
        )?;

        let d2d_context = api::d2d1::create_device_context(&engine.d2d_device)?;
        let comp_target = api::dcomp::create_target_for_hwnd(&engine.dcomp_device, hwnd, true)?;
        let comp_visual = api::dcomp::create_visual(&engine.dcomp_device)?;

        unsafe {
            comp_visual.SetContent(&swap_chain)?;
            comp_target.SetRoot(&comp_visual)?;
            engine.dcomp_device.Commit()?;
        }

        let mut canvas = Self {
            engine,
            swap_chain,
            d2d_context,
            comp_target,
            comp_visual,
            buffer_width,
            buffer_height,
        };

        canvas.setup_render_target()?;
        Ok(canvas)
    }

    pub fn setup_render_target(&mut self) -> Result<(), windows::core::Error> {
        unsafe {
            let back_buffer = self.swap_chain.GetBuffer::<windows::Win32::Graphics::Dxgi::IDXGISurface>(0)?;
            let d2d_bitmap: ID2D1Bitmap1 = api::d2d1::create_bitmap_from_dxgi_surface(&self.d2d_context, &back_buffer)?;
            self.d2d_context.SetTarget(&d2d_bitmap);
            self.d2d_context.SetDpi(96.0, 96.0);
        }
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), windows::core::Error> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        // 既存バッファに収まる場合は再確保をスキップする。
        if width <= self.buffer_width && height <= self.buffer_height {
            return Ok(());
        }

        // 不足する場合は余裕を持たせて再確保する。
        let new_buffer_width = width + 300;
        let new_buffer_height = height + 300;

        unsafe {
            self.d2d_context.SetTarget(None);
            self.swap_chain.ResizeBuffers(
                0,
                new_buffer_width,
                new_buffer_height,
                windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN,
                0,
            )?;

            self.buffer_width = new_buffer_width;
            self.buffer_height = new_buffer_height;
            self.setup_render_target()?;
        }
        Ok(())
    }

    pub fn begin_draw(&self) {
        unsafe {
            self.d2d_context.BeginDraw();
            // バッファ全体を透明でクリアする。
            self.d2d_context.Clear(None);
        }
    }

    /// 描画を確定して画面へ反映する。
    ///
    /// `sync_interval` が 1 なら VSync に同期し、0 なら即座に反映する。
    pub fn end_draw(&self, sync_interval: u32) -> Result<(), windows::core::Error> {
        unsafe {
            // Direct2D の描画を完了する。
            self.d2d_context.EndDraw(None, None)?;

            // スワップチェーンを表示する。追従性を優先する場合は sync_interval を 0 とする。
            self.swap_chain.Present(sync_interval, 0).ok()?;

            // DirectComposition の変更を確定する。
            self.engine.dcomp_device.Commit()?;
        }
        Ok(())
    }
}
