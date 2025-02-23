/*
    MartyPC
    https://github.com/dbalsom/martypc

    Copyright 2022-2024 Daniel Balsom

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the “Software”),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    ---------------------------------------------------------------------------

    display_backend_pixels::lib.rs

    Implements DisplayBackend for the Pixels backend
*/
use std::sync::{Arc, RwLock};
pub use wgpu_wrapper::{wgpu, Pixels};

use wgpu_wrapper::{builder::PixelsBuilder, wrapper::*};

pub use display_backend_trait::{
    BufferDimensions,
    DisplayBackend,
    DisplayBackendBuilder,
    TextureDimensions,
    //DisplayBackendError
};

use winit::window::Window;

use marty_egui_wgpu::context::GuiRenderContext;
use marty_scaler_wgpu::DisplayScaler;

use anyhow::Error;

pub struct WgpuBackend<'p> {
    pixels: Pixels<'p>,

    buffer_dim:  BufferDimensions,
    surface_dim: TextureDimensions,
}

impl<'p> WgpuBackend<'p> {
    pub fn new(w: u32, h: u32, window: &Window) -> Result<WgpuBackend, Error> {
        let window_size = window.inner_size();

        // Create a surface the size of the window's client area.
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);

        // Create the pixels instance.
        let pixels = PixelsBuilder::new(w, h, surface_texture)
            .request_adapter_options(wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .enable_vsync(false)
            .build()?;

        Ok(WgpuBackend {
            pixels,
            buffer_dim: (w, h, w).into(),
            surface_dim: (window_size.width, window_size.height).into(),
        })
    }
}

impl<'p> DisplayBackendBuilder for WgpuBackend<'p> {
    fn build(_buffer_size: BufferDimensions, _surface_size: TextureDimensions) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<'p, 'win> DisplayBackend<'win, 'p, GuiRenderContext> for WgpuBackend<'p>
where
    Self: 'p,
{
    type NativeDevice = wgpu::Device;
    type NativeBackend = ();
    type NativeTexture = wgpu::Texture;
    type NativeTextureFormat = wgpu::TextureFormat,
    type NativeBackendAdapterInfo = wgpu::AdapterInfo;
    type NativeScaler = Arc<
        RwLock<
            dyn DisplayScaler<
                wgpu::Device,
                wgpu::Queue,
                wgpu::Texture,
                NativeTextureView = wgpu::TextureView,
                NativeEncoder = wgpu::CommandEncoder,
                NativeRenderPass = wgpu::RenderPass<'static>,
            >,
        >,
    >;

    fn adapter_info(&self) -> Option<Self::NativeBackendAdapterInfo> {
        Some(self.pixels.adapter().get_info())
    }

    fn resize_surface_cpu_buffer(&mut self, new: BufferDimensions) -> Result<(), Error> {
        self.pixels.resize_buffer(new.w, new.h)?;
        self.buffer_dim = (new.w, new.h, new.w).into();
        Ok(())
    }

    fn resize_surface(&mut self, new: TextureDimensions) -> Result<(), Error> {
        self.pixels.resize_surface(new.w, new.h)?;
        self.surface_dim = (new.w, new.h).into();
        Ok(())
    }

    fn buf_dimensions(&self) -> BufferDimensions {
        self.buffer_dim
    }
    fn surface_dimensions(&self) -> TextureDimensions {
        self.surface_dim
    }

    fn buf(&self) -> &[u8] {
        self.pixels.frame()
    }
    fn buf_mut(&mut self) -> &mut [u8] {
        self.pixels.frame_mut()
    }

    fn get_backend_raw(&mut self) -> Option<&mut Self::NativeBackend> {
        Some(&mut self.pixels)
    }

    fn render(
        &mut self,
        scaler: Option<
            &mut Box<
                (dyn DisplayScaler<Pixels, NativeTextureView = wgpu::TextureView, NativeEncoder = wgpu::CommandEncoder>
                     + 'static),
            >,
        >,
        gui: Option<&mut GuiRenderContext>,
    ) -> Result<(), Error> {
        Ok(self.pixels.render_with(|encoder, render_target, context| {
            if let Some(scaler) = scaler {
                scaler.render(encoder, render_target);
            }

            if let Some(gui) = gui {
                //log::debug!("rendering gui!");
                gui.render(encoder, render_target, context);
            }

            Ok(())
        })?)
    }
}
