//! Native D3D11 rendering with source-equivalent shading/MSAA/shadows and a
//! nonblocking three-frame readback ring. Only the rug's bounds are allocated.
use crate::{
    cloth::{Cloth, N, Vertex},
    image::Image,
    lighting::LightEnv,
};
use std::{
    ffi::c_void,
    mem::{size_of, size_of_val},
    ptr,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{E_FAIL, HWND, POINT, SIZE},
        Graphics::{Direct3D::Fxc::*, Direct3D::*, Direct3D11::*, Dxgi::Common::*, Gdi::*},
        UI::WindowsAndMessaging::*,
    },
    core::{BOOL, Error, PCSTR, Result, s},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl Rect {
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width as i32).min(other.x + other.width as i32);
        let bottom = (self.y + self.height as i32).min(other.y + other.height as i32);
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: (right - x) as u32,
            height: (bottom - y) as u32,
        })
    }
}
#[repr(C)]
struct Uniforms {
    view: [f32; 2],
    surface: [f32; 2],
    origin: [f32; 2],
    texel: [f32; 2],
    alpha: f32,
    point_scale: f32,
    padding: [f32; 2],
    light: [f32; 4],
    tint: [f32; 4],
    shadow: [f32; 4],
    fold_lighting: [f32; 4],
}
#[derive(Clone, Copy)]
struct FrameMark {
    bounds: Rect,
    sequence: u64,
}
struct Slot {
    vertices: ID3D11Buffer,
    staging: ID3D11Texture2D,
    query: ID3D11Query,
    pending: Option<FrameMark>,
}
struct Targets {
    width: u32,
    height: u32,
    shadow_width: u32,
    shadow_height: u32,
    point_scale: f32,
    msaa: ID3D11Texture2D,
    resolved: ID3D11Texture2D,
    rtv: ID3D11RenderTargetView,
    depth: ID3D11DepthStencilView,
    shadow_rtv: ID3D11RenderTargetView,
    shadow_srv: ID3D11ShaderResourceView,
    slots: Vec<Slot>,
    dib: Dib,
}
struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut c_void,
    width: u32,
    height: u32,
}
impl Dib {
    fn new(width: u32, height: u32) -> Result<Self> {
        // SAFETY: the DIB is a top-down BGRA allocation owned by this object; GDI only uses it on this thread.
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err(Error::from_thread());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pixels = ptr::null_mut();
            let bitmap =
                match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut pixels, None, 0) {
                    Ok(bitmap) => bitmap,
                    Err(error) => {
                        let _ = DeleteDC(dc);
                        return Err(error);
                    }
                };
            let previous = SelectObject(dc, bitmap.into());
            Ok(Self {
                dc,
                bitmap,
                previous,
                pixels,
                width,
                height,
            })
        }
    }
}
impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

pub struct Renderer {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    rug_vs: ID3D11VertexShader,
    rug_ps: ID3D11PixelShader,
    shadow_vs: ID3D11VertexShader,
    shadow_ps: ID3D11PixelShader,
    full_vs: ID3D11VertexShader,
    composite_ps: ID3D11PixelShader,
    layout: ID3D11InputLayout,
    indices: ID3D11Buffer,
    constants: ID3D11Buffer,
    texture: ID3D11ShaderResourceView,
    sampler: ID3D11SamplerState,
    raster: ID3D11RasterizerState,
    depth_on: ID3D11DepthStencilState,
    depth_off: ID3D11DepthStencilState,
    opaque: ID3D11BlendState,
    premultiplied: ID3D11BlendState,
    maximum: ID3D11BlendState,
    targets: Option<Targets>,
    cpu_vertices: Vec<Vertex>,
    index_count: u32,
    sequence: u64,
    latest: Option<FrameMark>,
    view: [f32; 2],
    pub skipped_frames: u64,
}
fn required<T>(value: Option<T>) -> Result<T> {
    value.ok_or_else(|| Error::new(E_FAIL, "Windows returned a null graphics resource"))
}

impl Renderer {
    pub fn with_texture(view: [f32; 2], image: &Image) -> Result<Self> {
        // SAFETY: descriptors and bytecode are initialized; COM outputs are owned; all D3D calls stay on the UI thread.
        unsafe {
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                Default::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
            let device = required(device)?;
            let context = required(context)?;
            if device.CheckMultisampleQualityLevels(DXGI_FORMAT_B8G8R8A8_UNORM, 4)? == 0 {
                return Err(Error::new(E_FAIL, "GPU does not support required 4x MSAA"));
            }
            let (rug_vs, rug_signature) = vertex_shader(&device, s!("rugVS"))?;
            let (shadow_vs, _) = vertex_shader(&device, s!("shadowVS"))?;
            let (full_vs, _) = vertex_shader(&device, s!("fullVS"))?;
            let rug_ps = pixel_shader(&device, s!("rugFS"))?;
            let shadow_ps = pixel_shader(&device, s!("shadowFS"))?;
            let composite_ps = pixel_shader(&device, s!("compositeFS"))?;
            let element = |name, index, format, offset| D3D11_INPUT_ELEMENT_DESC {
                SemanticName: name,
                SemanticIndex: index,
                Format: format,
                AlignedByteOffset: offset,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                ..Default::default()
            };
            let mut layout = None;
            device.CreateInputLayout(
                &[
                    element(s!("POSITION"), 0, DXGI_FORMAT_R32G32B32_FLOAT, 0),
                    element(s!("TEXCOORD"), 0, DXGI_FORMAT_R32G32_FLOAT, 12),
                    element(s!("NORMAL"), 0, DXGI_FORMAT_R32G32B32_FLOAT, 20),
                    element(s!("TEXCOORD"), 1, DXGI_FORMAT_R32_FLOAT, 32),
                ],
                &rug_signature,
                Some(&mut layout),
            )?;
            let index_data = Cloth::indices();
            let index_count = index_data.len() as u32;
            let indices = buffer(
                &device,
                size_of_val(index_data.as_slice()) as u32,
                D3D11_BIND_INDEX_BUFFER,
                index_data.as_ptr().cast(),
            )?;
            let constants = buffer(
                &device,
                size_of::<Uniforms>() as u32,
                D3D11_BIND_CONSTANT_BUFFER,
                ptr::null(),
            )?;
            let texture = upload_texture(&device, &context, image)?;
            let mut sampler = None;
            device.CreateSamplerState(
                &D3D11_SAMPLER_DESC {
                    Filter: D3D11_FILTER_ANISOTROPIC,
                    AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                    MaxAnisotropy: 4,
                    ComparisonFunc: D3D11_COMPARISON_NEVER,
                    MaxLOD: f32::MAX,
                    ..Default::default()
                },
                Some(&mut sampler),
            )?;
            let mut raster = None;
            device.CreateRasterizerState(
                &D3D11_RASTERIZER_DESC {
                    FillMode: D3D11_FILL_SOLID,
                    CullMode: D3D11_CULL_NONE,
                    FrontCounterClockwise: false.into(),
                    DepthClipEnable: true.into(),
                    ScissorEnable: true.into(),
                    MultisampleEnable: true.into(),
                    ..Default::default()
                },
                Some(&mut raster),
            )?;
            let depth_desc = D3D11_DEPTH_STENCIL_DESC {
                DepthEnable: true.into(),
                DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
                DepthFunc: D3D11_COMPARISON_LESS,
                ..Default::default()
            };
            let mut depth_on = None;
            let mut depth_off = None;
            device.CreateDepthStencilState(&depth_desc, Some(&mut depth_on))?;
            device.CreateDepthStencilState(
                &D3D11_DEPTH_STENCIL_DESC {
                    DepthEnable: false.into(),
                    ..depth_desc
                },
                Some(&mut depth_off),
            )?;
            let mut opaque = None;
            let mut premultiplied = None;
            let mut maximum = None;
            let mut blend = D3D11_BLEND_DESC::default();
            blend.RenderTarget[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8;
            device.CreateBlendState(&blend, Some(&mut opaque))?;
            blend.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            device.CreateBlendState(&blend, Some(&mut premultiplied))?;
            blend.RenderTarget[0].DestBlend = D3D11_BLEND_ONE;
            blend.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ONE;
            blend.RenderTarget[0].BlendOp = D3D11_BLEND_OP_MAX;
            blend.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_MAX;
            device.CreateBlendState(&blend, Some(&mut maximum))?;
            Ok(Self {
                device,
                context,
                rug_vs,
                rug_ps,
                shadow_vs,
                shadow_ps,
                full_vs,
                composite_ps,
                layout: required(layout)?,
                indices,
                constants,
                texture,
                sampler: required(sampler)?,
                raster: required(raster)?,
                depth_on: required(depth_on)?,
                depth_off: required(depth_off)?,
                opaque: required(opaque)?,
                premultiplied: required(premultiplied)?,
                maximum: required(maximum)?,
                targets: None,
                cpu_vertices: vec![Vertex::default(); N],
                index_count,
                sequence: 0,
                latest: None,
                view,
                skipped_frames: 0,
            })
        }
    }
    pub fn set_texture(&mut self, image: &Image) -> Result<()> {
        self.texture = upload_texture(&self.device, &self.context, image)?;
        Ok(())
    }
    pub fn resize_view(&mut self, view: [f32; 2]) {
        if self.view != view {
            self.targets = None;
            self.latest = None;
        }
        self.view = view;
    }
    pub fn pending(&self) -> bool {
        self.targets
            .as_ref()
            .is_some_and(|targets| targets.slots.iter().any(|slot| slot.pending.is_some()))
    }
    pub fn poll(&mut self) -> Result<bool> {
        let Some(targets) = &mut self.targets else {
            return Ok(false);
        };
        let mut order = [0usize, 1, 2];
        order.sort_by_key(|index| {
            targets.slots[*index]
                .pending
                .map_or(u64::MAX, |mark| mark.sequence)
        });
        let mut ready = None;
        // SAFETY: event queries synchronize staging copies. Map is nonblocking; memory is read only after GPU completion.
        unsafe {
            for index in order {
                let slot = &mut targets.slots[index];
                let Some(_) = slot.pending else {
                    continue;
                };
                let mut done = BOOL(0);
                self.context.GetData(
                    &slot.query,
                    Some((&mut done as *mut BOOL).cast()),
                    size_of::<BOOL>() as u32,
                    D3D11_ASYNC_GETDATA_DONOTFLUSH.0 as u32,
                )?;
                if !done.as_bool() {
                    break;
                }
                ready = Some(index);
            }
            // Older completed images would be overwritten immediately. Read only the newest
            // completed frame, then release all completed slots without copying stale pixels.
            if let Some(index) = ready {
                let slot = &mut targets.slots[index];
                let mark = slot.pending.unwrap();
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                match self.context.Map(
                    &slot.staging,
                    0,
                    D3D11_MAP_READ,
                    D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
                    Some(&mut mapped),
                ) {
                    Ok(()) => {}
                    Err(error) if error.code().0 == 0x887A000Au32 as i32 => return Ok(false),
                    Err(error) => return Err(error),
                }
                for row in 0..targets.height as usize {
                    ptr::copy_nonoverlapping(
                        (mapped.pData as *const u8).add(row * mapped.RowPitch as usize),
                        (targets.dib.pixels as *mut u8).add(row * targets.width as usize * 4),
                        targets.width as usize * 4,
                    );
                }
                self.context.Unmap(&slot.staging, 0);
                for slot in &mut targets.slots {
                    if slot
                        .pending
                        .is_some_and(|frame| frame.sequence <= mark.sequence)
                    {
                        slot.pending = None;
                    }
                }
                self.latest = Some(mark);
            }
        }
        Ok(ready.is_some())
    }
    pub fn submit(&mut self, cloth: &Cloth, alpha: f32, environment: LightEnv) -> Result<bool> {
        self.view = cloth.view;
        let mut bounds = render_bounds(cloth, environment);
        let width = self
            .targets
            .as_ref()
            .map_or(bounds.width, |target| target.width.max(bounds.width));
        let height = self
            .targets
            .as_ref()
            .map_or(bounds.height, |target| target.height.max(bounds.height));
        bounds.width = width;
        bounds.height = height;
        if self.targets.as_ref().is_none_or(|targets| {
            targets.width != width
                || targets.height != height
                || targets.point_scale != cloth.point_scale()
        }) {
            self.targets = Some(create_targets(
                &self.device,
                width,
                height,
                cloth.point_scale(),
            )?);
            self.latest = None;
        }
        let _ = self.poll()?;
        if self
            .targets
            .as_ref()
            .unwrap()
            .slots
            .iter()
            .all(|slot| slot.pending.is_some())
        {
            self.skipped_frames += 1;
            return Ok(false);
        }
        let targets = self.targets.as_mut().unwrap();
        let slot = targets
            .slots
            .iter_mut()
            .find(|slot| slot.pending.is_none())
            .unwrap();
        cloth.fill_vertices(&mut self.cpu_vertices);
        let uniforms = Uniforms {
            view: cloth.view,
            surface: [width as f32, height as f32],
            origin: [bounds.x as f32, bounds.y as f32],
            texel: [
                1. / targets.shadow_width as f32,
                1. / targets.shadow_height as f32,
            ],
            alpha,
            point_scale: cloth.point_scale(),
            padding: [
                if cloth.soft_light { 1. } else { 0. },
                // Keep linear depth ordering for every current fold, including heights above
                // the old 400-point clipping threshold. The margin prevents a zero depth tie.
                cloth
                    .position
                    .iter()
                    .map(|p| p[2].abs())
                    .fold(400. * cloth.point_scale(), f32::max)
                    * 2.
                    + cloth.point_scale(),
            ],
            light: environment.light,
            tint: environment.tint,
            shadow: [
                environment.shadow[0],
                environment.shadow[1],
                environment.shadow[2] * cloth.point_scale(),
                environment.shadow[3] * cloth.point_scale(),
            ],
            fold_lighting: [environment.fold_contrast, 0., 0., 0.],
        };
        // SAFETY: all resources belong to one device/context, are live across queued commands,
        // and each vertex/staging buffer is reused only after its event query has completed.
        unsafe {
            self.context.UpdateSubresource(
                &slot.vertices,
                0,
                None,
                self.cpu_vertices.as_ptr().cast(),
                0,
                0,
            );
            self.context.UpdateSubresource(
                &self.constants,
                0,
                None,
                (&uniforms as *const Uniforms).cast(),
                0,
                0,
            );
            self.context.RSSetState(&self.raster);
            self.context.IASetInputLayout(&self.layout);
            self.context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.context.IASetVertexBuffers(
                0,
                1,
                Some(&Some(slot.vertices.clone())),
                Some(&(size_of::<Vertex>() as u32)),
                Some(&0),
            );
            self.context
                .IASetIndexBuffer(&self.indices, DXGI_FORMAT_R16_UINT, 0);
            self.context
                .VSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            self.context
                .PSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            self.context
                .PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            // Fullscreen sets shadow opacity to zero. Skip both the shadow geometry and its
            // full-resolution 13-sample blur rather than rendering a completely invisible pass.
            if environment.tint[3] > 0. {
                self.context.PSSetShaderResources(0, Some(&[None]));
                self.context
                    .OMSetRenderTargets(Some(&[Some(targets.shadow_rtv.clone())]), None);
                self.context
                    .ClearRenderTargetView(&targets.shadow_rtv, &[0.; 4]);
                self.context.OMSetBlendState(&self.maximum, None, u32::MAX);
                self.context.OMSetDepthStencilState(&self.depth_off, 0);
                viewport(&self.context, targets.shadow_width, targets.shadow_height);
                self.context.VSSetShader(&self.shadow_vs, None);
                self.context.PSSetShader(&self.shadow_ps, None);
                self.context
                    .PSSetShaderResources(0, Some(&[Some(self.texture.clone())]));
                self.context.DrawIndexed(self.index_count, 0, 0);
            }
            // Soft shadow composite into a 4x multisampled, premultiplied-alpha target.
            self.context
                .OMSetRenderTargets(Some(&[Some(targets.rtv.clone())]), &targets.depth);
            self.context.ClearRenderTargetView(&targets.rtv, &[0.; 4]);
            self.context
                .ClearDepthStencilView(&targets.depth, D3D11_CLEAR_DEPTH.0, 1., 0);
            viewport(&self.context, width, height);
            if environment.tint[3] > 0. {
                self.context.OMSetDepthStencilState(&self.depth_off, 0);
                self.context
                    .OMSetBlendState(&self.premultiplied, None, u32::MAX);
                self.context.IASetInputLayout(None);
                self.context.VSSetShader(&self.full_vs, None);
                self.context.PSSetShader(&self.composite_ps, None);
                self.context
                    .PSSetShaderResources(0, Some(&[Some(targets.shadow_srv.clone())]));
                self.context.Draw(3, 0);
            }
            // Rug material, back face, AO, specular and anisotropic mipmapped texture.
            self.context.IASetInputLayout(&self.layout);
            self.context.OMSetBlendState(&self.opaque, None, u32::MAX);
            self.context.OMSetDepthStencilState(&self.depth_on, 0);
            self.context.VSSetShader(&self.rug_vs, None);
            self.context.PSSetShader(&self.rug_ps, None);
            self.context
                .PSSetShaderResources(0, Some(&[Some(self.texture.clone())]));
            self.context.DrawIndexed(self.index_count, 0, 0);
            self.context.OMSetRenderTargets(None, None);
            self.context.PSSetShaderResources(0, Some(&[None]));
            self.context.ResolveSubresource(
                &targets.resolved,
                0,
                &targets.msaa,
                0,
                DXGI_FORMAT_B8G8R8A8_UNORM,
            );
            self.context.CopyResource(&slot.staging, &targets.resolved);
            self.context.End(&slot.query);
            self.context.Flush();
        }
        self.sequence += 1;
        slot.pending = Some(FrameMark {
            bounds,
            sequence: self.sequence,
        });
        Ok(true)
    }
    pub fn render_sync(&mut self, cloth: &Cloth, alpha: f32, environment: LightEnv) -> Result<()> {
        if !self.submit(cloth, alpha, environment)? {
            return Err(Error::new(E_FAIL, "GPU did not accept diagnostic frame"));
        }
        let sequence = self.sequence;
        let start = Instant::now();
        while self.latest.is_none_or(|frame| frame.sequence < sequence) {
            self.poll()?;
            if start.elapsed() > Duration::from_secs(5) {
                return Err(Error::new(E_FAIL, "GPU render timed out"));
            }
            std::thread::yield_now();
        }
        Ok(())
    }
    pub fn present_to(&self, hwnd: HWND, display: Rect, visible: bool) -> Result<()> {
        let (Some(mark), Some(targets)) = (self.latest, &self.targets) else {
            return Ok(());
        };
        let Some(part) = mark.bounds.intersection(display).filter(|_| visible) else {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            return Ok(());
        };
        // SAFETY: hwnd is our per-display child. The source DIB remains selected and alive,
        // and the source rectangle is within its allocation.
        unsafe {
            // For a WS_CHILD overlay, the destination is relative to its parent.
            let destination = POINT {
                x: part.x,
                y: part.y,
            };
            // Commit geometry with the new pixels. Moving or raising the child separately
            // lets Explorer compose the old surface at the new bounds.
            UpdateLayeredWindow(
                hwnd,
                None,
                Some(&destination),
                Some(&SIZE {
                    cx: part.width as i32,
                    cy: part.height as i32,
                }),
                Some(targets.dib.dc),
                Some(&POINT {
                    x: part.x - mark.bounds.x,
                    y: part.y - mark.bounds.y,
                }),
                Default::default(),
                Some(&BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                    BlendFlags: 0,
                }),
                ULW_ALPHA,
            )?;
            if GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE.0 == 0 {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        Ok(())
    }
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let (Some(mark), Some(targets)) = (self.latest, &self.targets) else {
            return [0; 4];
        };
        let x = x as i32 - mark.bounds.x;
        let y = y as i32 - mark.bounds.y;
        if x < 0 || y < 0 || x >= targets.dib.width as i32 || y >= targets.dib.height as i32 {
            return [0; 4];
        }
        // SAFETY: checked coordinates lie inside the live DIB allocation.
        unsafe {
            *((targets.dib.pixels as *const [u8; 4])
                .add(y as usize * targets.dib.width as usize + x as usize))
        }
    }
    pub fn snapshot(&self) -> Image {
        let mut result = Image::new(self.view[0] as u32, self.view[1] as u32);
        let (Some(mark), Some(targets)) = (self.latest, &self.targets) else {
            return result;
        };
        for y in 0..targets.height {
            let destination_y = mark.bounds.y + y as i32;
            if destination_y < 0 || destination_y >= result.height as i32 {
                continue;
            }
            for x in 0..targets.width {
                let destination_x = mark.bounds.x + x as i32;
                if destination_x < 0 || destination_x >= result.width as i32 {
                    continue;
                }
                let pixel = self.pixel(destination_x as usize, destination_y as usize);
                let index =
                    ((destination_y as u32 * result.width + destination_x as u32) * 4) as usize;
                result.bgra[index + 3] = pixel[3];
                if pixel[3] > 0 {
                    for (channel, value) in pixel[..3].iter().enumerate() {
                        result.bgra[index + channel] =
                            (u32::from(*value) * 255 / u32::from(pixel[3])).min(255) as u8;
                    }
                }
            }
        }
        result
    }
    pub fn allocation_size(&self) -> Option<(u32, u32)> {
        self.targets.as_ref().map(|t| (t.width, t.height))
    }
    pub fn frame_sequence(&self) -> u64 {
        self.latest.map_or(0, |frame| frame.sequence)
    }
}
fn render_bounds(cloth: &Cloth, environment: LightEnv) -> Rect {
    let mut minimum = [f32::INFINITY; 2];
    let mut maximum = [f32::NEG_INFINITY; 2];
    for p in &cloth.position {
        for axis in 0..2 {
            let projected =
                p[axis] + (p[axis] - cloth.view[axis] * 0.5) * p[2] * cloth.perspective();
            let shadow = p[axis]
                + environment.shadow[axis] * p[2]
                + environment.shadow[axis + 2] * cloth.point_scale();
            minimum[axis] = minimum[axis].min(projected).min(shadow);
            maximum[axis] = maximum[axis].max(projected).max(shadow);
        }
    }
    let margin = 40. * cloth.point_scale();
    let x = (minimum[0] - margin)
        .floor()
        .clamp(0., cloth.view[0].max(1.) - 1.) as i32;
    let y = (minimum[1] - margin)
        .floor()
        .clamp(0., cloth.view[1].max(1.) - 1.) as i32;
    let right = (maximum[0] + margin)
        .ceil()
        .clamp(x as f32 + 1., cloth.view[0].max(x as f32 + 1.));
    let bottom = (maximum[1] + margin)
        .ceil()
        .clamp(y as f32 + 1., cloth.view[1].max(y as f32 + 1.));
    Rect {
        x,
        y,
        width: ((right - x as f32) as u32).max(1).div_ceil(64) * 64,
        height: ((bottom - y as f32) as u32).max(1).div_ceil(64) * 64,
    }
}
unsafe fn viewport(context: &ID3D11DeviceContext, width: u32, height: u32) {
    unsafe {
        context.RSSetViewports(Some(&[D3D11_VIEWPORT {
            Width: width as f32,
            Height: height as f32,
            MaxDepth: 1.,
            ..Default::default()
        }]));
        context.RSSetScissorRects(Some(&[windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        }]));
    }
}
fn create_targets(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    point_scale: f32,
) -> Result<Targets> {
    // SAFETY: each texture descriptor matches its views and the resources are owned by Targets.
    unsafe {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 4,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
            ..Default::default()
        };
        let msaa = texture2d(device, &desc)?;
        let mut rtv = None;
        device.CreateRenderTargetView(&msaa, None, Some(&mut rtv))?;
        let resolved = texture2d(
            device,
            &D3D11_TEXTURE2D_DESC {
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BindFlags: 0,
                ..desc
            },
        )?;
        let depth_texture = texture2d(
            device,
            &D3D11_TEXTURE2D_DESC {
                Format: DXGI_FORMAT_D32_FLOAT,
                BindFlags: D3D11_BIND_DEPTH_STENCIL.0 as u32,
                ..desc
            },
        )?;
        let mut depth = None;
        device.CreateDepthStencilView(&depth_texture, None, Some(&mut depth))?;
        let shadow_width = (width as f32 * 0.3 / point_scale) as u32;
        let shadow_height = (height as f32 * 0.3 / point_scale) as u32;
        let shadow = texture2d(
            device,
            &D3D11_TEXTURE2D_DESC {
                Width: shadow_width.max(8),
                Height: shadow_height.max(8),
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                ..desc
            },
        )?;
        let mut shadow_rtv = None;
        let mut shadow_srv = None;
        device.CreateRenderTargetView(&shadow, None, Some(&mut shadow_rtv))?;
        device.CreateShaderResourceView(&shadow, None, Some(&mut shadow_srv))?;
        let mut slots = Vec::with_capacity(3);
        for _ in 0..3 {
            let vertices = buffer(
                device,
                (N * size_of::<Vertex>()) as u32,
                D3D11_BIND_VERTEX_BUFFER,
                ptr::null(),
            )?;
            let staging = texture2d(
                device,
                &D3D11_TEXTURE2D_DESC {
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    ..desc
                },
            )?;
            let mut query = None;
            device.CreateQuery(
                &D3D11_QUERY_DESC {
                    Query: D3D11_QUERY_EVENT,
                    ..Default::default()
                },
                Some(&mut query),
            )?;
            slots.push(Slot {
                vertices,
                staging,
                query: required(query)?,
                pending: None,
            });
        }
        Ok(Targets {
            width,
            height,
            shadow_width: shadow_width.max(8),
            shadow_height: shadow_height.max(8),
            point_scale,
            msaa,
            resolved,
            rtv: required(rtv)?,
            depth: required(depth)?,
            shadow_rtv: required(shadow_rtv)?,
            shadow_srv: required(shadow_srv)?,
            slots,
            dib: Dib::new(width, height)?,
        })
    }
}
fn texture2d(device: &ID3D11Device, desc: &D3D11_TEXTURE2D_DESC) -> Result<ID3D11Texture2D> {
    let mut texture = None;
    unsafe {
        device.CreateTexture2D(desc, None, Some(&mut texture))?;
    }
    required(texture)
}
fn upload_texture(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    image: &Image,
) -> Result<ID3D11ShaderResourceView> {
    // SAFETY: decoded BGRA covers exactly width*height pixels. GPU owns the full mip chain after upload.
    unsafe {
        let texture = texture2d(
            device,
            &D3D11_TEXTURE2D_DESC {
                Width: image.width,
                Height: image.height,
                MipLevels: 0,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
                MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
                ..Default::default()
            },
        )?;
        context.UpdateSubresource(
            &texture,
            0,
            None,
            image.bgra.as_ptr().cast(),
            image.width * 4,
            0,
        );
        let mut view = None;
        device.CreateShaderResourceView(&texture, None, Some(&mut view))?;
        let view = required(view)?;
        context.GenerateMips(&view);
        Ok(view)
    }
}
fn buffer(
    device: &ID3D11Device,
    bytes: u32,
    bind: D3D11_BIND_FLAG,
    data: *const c_void,
) -> Result<ID3D11Buffer> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: bytes,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: bind.0 as u32,
        ..Default::default()
    };
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: data,
        ..Default::default()
    };
    let mut result = None;
    unsafe {
        device.CreateBuffer(
            &desc,
            if data.is_null() { None } else { Some(&initial) },
            Some(&mut result),
        )?;
    }
    required(result)
}
fn vertex_shader(device: &ID3D11Device, entry: PCSTR) -> Result<(ID3D11VertexShader, Vec<u8>)> {
    let code = compile(entry, s!("vs_5_0"))?;
    let mut shader = None;
    unsafe {
        device.CreateVertexShader(&code, None, Some(&mut shader))?;
    }
    Ok((required(shader)?, code))
}
fn pixel_shader(device: &ID3D11Device, entry: PCSTR) -> Result<ID3D11PixelShader> {
    let code = compile(entry, s!("ps_5_0"))?;
    let mut shader = None;
    unsafe {
        device.CreatePixelShader(&code, None, Some(&mut shader))?;
    }
    required(shader)
}
fn compile(entry: PCSTR, target: PCSTR) -> Result<Vec<u8>> {
    let source = include_str!("rug.hlsl");
    let mut code = None;
    let mut errors = None;
    unsafe {
        if let Err(error) = D3DCompile(
            source.as_ptr().cast(),
            source.len(),
            None,
            None,
            None,
            entry,
            target,
            D3DCOMPILE_OPTIMIZATION_LEVEL3,
            0,
            &mut code,
            Some(&mut errors),
        ) {
            let message = errors.map(|e| {
                String::from_utf8_lossy(std::slice::from_raw_parts(
                    e.GetBufferPointer().cast(),
                    e.GetBufferSize(),
                ))
                .into_owned()
            });
            return Err(Error::new(
                error.code(),
                message.unwrap_or_else(|| error.to_string()),
            ));
        }
        let code = required(code)?;
        Ok(
            std::slice::from_raw_parts(code.GetBufferPointer().cast(), code.GetBufferSize())
                .to_vec(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloth::{NX, NY};

    #[test]
    fn fold_fade_reduces_shading_contrast_without_changing_flat_color_or_coverage() {
        let view = [640., 480.];
        let mut texture = Image::new(2, 2);
        for pixel in texture.bgra.as_chunks_mut::<4>().0 {
            *pixel = [80, 130, 200, 255];
        }
        let mut cloth = Cloth::new(view);
        cloth.reset_fullscreen(view, 64.);
        let mut renderer = Renderer::with_texture(view, &texture).unwrap();
        let mut light = LightEnv::NEUTRAL;
        light.light = [0., -0.7, 0.7, 1.];
        light.tint[3] = 0.;
        let alpha = 0.65;
        let center_pixel = |renderer: &Renderer| renderer.pixel(320, 240);
        renderer.render_sync(&cloth, alpha, light).unwrap();
        let flat = center_pixel(&renderer);
        let mut reduced = light;
        reduced.fold_contrast = 0.3;
        renderer.render_sync(&cloth, alpha, reduced).unwrap();
        let flat_reduced = center_pixel(&renderer);
        assert!(
            flat.into_iter()
                .zip(flat_reduced)
                .all(|(a, b)| a.abs_diff(b) <= 1)
        );

        for p in &mut cloth.position {
            p[2] = 140. + 0.5 * (p[1] - 240.);
        }
        renderer.render_sync(&cloth, alpha, light).unwrap();
        let folded = center_pixel(&renderer);
        let coverage = renderer.snapshot().bgra;
        renderer.render_sync(&cloth, alpha, reduced).unwrap();
        let faded = center_pixel(&renderer);
        let faded_coverage = renderer.snapshot().bgra;
        assert!(
            flat[2].abs_diff(folded[2]) >= 10,
            "the chosen fold must be visibly shaded: {flat:?}, {folded:?}",
        );
        assert!(flat[2].abs_diff(faded[2]) < flat[2].abs_diff(folded[2]) / 2);
        assert!(
            coverage
                .iter()
                .skip(3)
                .step_by(4)
                .eq(faded_coverage.iter().skip(3).step_by(4)),
            "fold lighting must not change geometry or layer opacity",
        );
    }

    #[test]
    fn layered_present_moves_clipped_child_without_reordering() {
        use windows::Win32::Foundation::RECT;
        use windows::core::w;

        struct Parent(HWND);
        impl Drop for Parent {
            fn drop(&mut self) {
                // SAFETY: this thread created the parent and owns its children.
                let _ = unsafe { DestroyWindow(self.0) };
            }
        }

        let view = [640., 480.];
        let mut texture = Image::new(2, 2);
        texture.bgra.fill(255);
        let mut renderer = Renderer::with_texture(view, &texture).unwrap();
        let mut cloth = Cloth::new(view);
        cloth.reset_fullscreen(view, 64.);
        renderer.render_sync(&cloth, 1., LightEnv::NEUTRAL).unwrap();

        // SAFETY: the hidden parent and both children live on this test thread.
        unsafe {
            let parent = Parent(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    None,
                    WS_POPUP,
                    -10000,
                    600,
                    640,
                    480,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let child = |style| {
                CreateWindowExW(
                    style,
                    w!("STATIC"),
                    None,
                    WS_CHILD,
                    0,
                    0,
                    1,
                    1,
                    Some(parent.0),
                    None,
                    None,
                    None,
                )
                .unwrap()
            };
            let layered = child(WS_EX_LAYERED);
            let sibling = child(WINDOW_EX_STYLE::default());
            SetWindowPos(
                sibling,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .unwrap();

            let visible = || GetWindowLongPtrW(layered, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0;
            assert_eq!(GetWindow(layered, GW_HWNDPREV).ok(), Some(sibling));
            for (x, y, width, height) in [(40, 20, 240, 150), (60, 50, 160, 100)] {
                let display = Rect {
                    x,
                    y,
                    width,
                    height,
                };
                renderer.present_to(layered, display, true).unwrap();
                assert_eq!(GetWindow(layered, GW_HWNDPREV).ok(), Some(sibling));
                let mut actual = RECT::default();
                GetWindowRect(layered, &mut actual).unwrap();
                assert_eq!((actual.left, actual.top), (-10000 + x, 600 + y));
                assert_eq!(actual.right - actual.left, width as i32);
                assert_eq!(actual.bottom - actual.top, height as i32);
                assert!(visible());
            }
            let display = Rect {
                x: 60,
                y: 50,
                width: 160,
                height: 100,
            };
            renderer.present_to(layered, display, false).unwrap();
            assert!(!visible());
            renderer.present_to(layered, display, true).unwrap();
            assert!(visible());
            assert_eq!(GetWindow(layered, GW_HWNDPREV).ok(), Some(sibling));
        }
    }

    #[test]
    fn shadow_toggle_and_queued_frames_preserve_the_latest_image() {
        let mut image = Image::new(8, 8);
        for pixel in image.bgra.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[180, 80, 40, 255]);
        }
        let view = [640., 480.];
        let mut cloth = Cloth::new(view);
        cloth.reset_pose([320., 240.], 340., 70., 0.);
        let mut renderer = Renderer::with_texture(view, &image).unwrap();
        let shadow = LightEnv::NEUTRAL;
        renderer.render_sync(&cloth, 1., shadow).unwrap();
        let expected = renderer.snapshot().bgra;
        let mut no_shadow = shadow;
        no_shadow.tint[3] = 0.;
        renderer.render_sync(&cloth, 1., no_shadow).unwrap();
        let without_shadow = renderer.snapshot().bgra;
        assert!(
            without_shadow
                .iter()
                .skip(3)
                .step_by(4)
                .zip(expected.iter().skip(3).step_by(4))
                .any(|(a, b)| a < b),
            "disabling shadow must remove shadow alpha outside the rug"
        );
        renderer.render_sync(&cloth, 1., shadow).unwrap();
        let restored = renderer.snapshot().bgra;
        assert!(
            restored
                .iter()
                .zip(&expected)
                .enumerate()
                .all(|(i, (a, b))| if i % 4 == 3 {
                    a == b
                } else {
                    a.abs_diff(*b) <= 1
                }),
            "state after skipping shadow must be restored within one UNORM rounding unit"
        );
        for alpha in [0.9, 0.6, 0.26] {
            assert!(renderer.submit(&cloth, alpha, no_shadow).unwrap());
        }
        let sequence = renderer.sequence;
        let deadline = Instant::now() + Duration::from_secs(5);
        while renderer.frame_sequence() < sequence {
            assert!(Instant::now() < deadline);
            renderer.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!renderer.pending());
        let queued: Vec<_> = (0..480)
            .flat_map(|y| (0..640).map(move |x| (x, y)))
            .map(|(x, y)| renderer.pixel(x, y))
            .collect();
        renderer.render_sync(&cloth, 0.26, no_shadow).unwrap();
        for (i, actual) in queued.iter().enumerate() {
            let expected = renderer.pixel(i % 640, i / 640);
            assert_eq!(actual[3], expected[3], "latest frame alpha at {i}");
            // Compare premultiplied UNORM bytes: at alpha=0.26, one GPU rounding unit
            // becomes four units after snapshot's unpremultiplication.
            assert!(
                (0..3).all(|c| actual[c].abs_diff(expected[c]) <= 1),
                "latest frame color at {i}: {actual:?} vs {expected:?}"
            );
        }
    }

    #[test]
    fn raised_layers_keep_their_depth_order_on_the_gpu() {
        let mut texture = Image::new(64, 64);
        for (index, pixel) in texture.bgra.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel.copy_from_slice(if index / 64 < 32 {
                &[0, 0, 220, 255]
            } else {
                &[220, 0, 0, 255]
            });
        }
        let view = [640., 480.];
        let mut renderer = Renderer::with_texture(view, &texture).unwrap();
        let mut cloth = Cloth::new(view);
        cloth.reset_fullscreen(view, 64.);
        let mut light = LightEnv::NEUTRAL;
        light.tint[3] = 0.;
        for heights in [[450., 550.], [550., 450.]] {
            for j in 0..NY {
                let layer = usize::from(j >= NY / 2);
                let row = j - layer * (NY / 2);
                let z = heights[layer];
                let k = z * cloth.perspective();
                for i in 0..NX {
                    let screen = [100. + i as f32 * 6., 80. + row as f32 * 10.];
                    cloth.position[j * NX + i] = [
                        (screen[0] + view[0] * 0.5 * k) / (1. + k),
                        (screen[1] + view[1] * 0.5 * k) / (1. + k),
                        z,
                    ];
                }
            }
            renderer.render_sync(&cloth, 1., light).unwrap();
            let pixel = renderer.pixel(250, 160);
            let (near, far) = if heights[1] > heights[0] {
                (0, 2)
            } else {
                (2, 0)
            };
            assert!(
                pixel[near] > pixel[far].saturating_add(80),
                "higher layer must occlude lower layer: heights={heights:?}, BGRA={pixel:?}"
            );
        }
    }
}
