//! Native WIC image decoding/encoding, EXIF orientation and bounded image import.
use std::{path::Path, ptr};
use windows::{
    Win32::{
        Foundation::{E_FAIL, GENERIC_READ, GENERIC_WRITE},
        Graphics::Imaging::*,
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize,
                StructuredStorage::{PROPVARIANT, PropVariantClear},
            },
            Variant::{VT_UI2, VT_UI4},
        },
    },
    core::{Error, HSTRING, Interface, Result, w},
};

pub struct ComScope(bool);
impl ComScope {
    pub fn new() -> Result<Self> {
        // SAFETY: balances only successful initialization on the calling thread. An existing
        // STA is also suitable for these WIC calls and is left under its owner's control.
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result.0 == 0x80010106u32 as i32 {
            return Ok(Self(false));
        }
        result.ok()?;
        Ok(Self(true))
    }
}
impl Drop for ComScope {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}
impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            bgra: vec![0; (width * height * 4) as usize],
        }
    }
    pub fn decode_file(path: &Path, max_edge: u32) -> Result<Self> {
        let _com = ComScope::new()?;
        // WIC streams the file. The compressed input is not copied wholesale into RAM before resizing.
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let decoder = factory.CreateDecoderFromFilename(
                &HSTRING::from(path.as_os_str()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnLoad,
            )?;
            Self::from_decoder(&factory, &decoder, max_edge)
        }
    }
    pub fn decode(bytes: &[u8], max_edge: u32) -> Result<Self> {
        let _com = ComScope::new()?;
        // SAFETY: embedded asset bytes outlive the decoder and all COM objects are released in this scope.
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = factory.CreateStream()?;
            stream.InitializeFromMemory(bytes)?;
            let decoder = factory.CreateDecoderFromStream(
                &stream,
                ptr::null(),
                WICDecodeMetadataCacheOnLoad,
            )?;
            Self::from_decoder(&factory, &decoder, max_edge)
        }
    }
    unsafe fn from_decoder(
        factory: &IWICImagingFactory,
        decoder: &IWICBitmapDecoder,
        max_edge: u32,
    ) -> Result<Self> {
        // SAFETY: WIC owns all transforms. Allocation happens only after orientation and size bounds are known.
        unsafe {
            let frame = decoder.GetFrame(0)?;
            let orientation = orientation(&frame);
            let mut source: IWICBitmapSource = frame.cast()?;
            let mut count = 0;
            let _ = frame.GetColorContexts(&mut [], &mut count);
            if count > 0 {
                if count > 16 {
                    return Err(Error::new(E_FAIL, "Image contains too many color profiles"));
                }
                let mut contexts = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    contexts.push(Some(factory.CreateColorContext()?));
                }
                frame.GetColorContexts(&mut contexts, &mut count)?;
                let contexts: Vec<_> = contexts.into_iter().flatten().collect();
                let source_context = contexts
                    .iter()
                    .find(|context| {
                        context
                            .GetType()
                            .is_ok_and(|kind| kind == WICColorContextProfile)
                    })
                    .or_else(|| {
                        contexts.iter().find(|context| {
                            context
                                .GetType()
                                .is_ok_and(|kind| kind == WICColorContextExifColorSpace)
                        })
                    });
                if let Some(context) = source_context {
                    let already_srgb = context.GetType()? == WICColorContextExifColorSpace
                        && context.GetExifColorSpace()? == 1;
                    if !already_srgb {
                        let destination = factory.CreateColorContext()?;
                        destination.InitializeFromExifColorSpace(1)?;
                        let transform = factory.CreateColorTransformer()?;
                        transform.Initialize(
                            &source,
                            context,
                            &destination,
                            &GUID_WICPixelFormat32bppBGRA,
                        )?;
                        source = transform.cast()?;
                    }
                }
            }
            let transform = match orientation {
                2 => WICBitmapTransformFlipHorizontal,
                3 => WICBitmapTransformRotate180,
                4 => WICBitmapTransformFlipVertical,
                5 => WICBitmapTransformOptions(
                    WICBitmapTransformRotate270.0 | WICBitmapTransformFlipHorizontal.0,
                ),
                6 => WICBitmapTransformRotate90,
                7 => WICBitmapTransformOptions(
                    WICBitmapTransformRotate90.0 | WICBitmapTransformFlipHorizontal.0,
                ),
                8 => WICBitmapTransformRotate270,
                _ => WICBitmapTransformRotate0,
            };
            if transform != WICBitmapTransformRotate0 {
                let rotator = factory.CreateBitmapFlipRotator()?;
                rotator.Initialize(&source, transform)?;
                source = rotator.cast()?;
            }
            let (mut width, mut height) = (0, 0);
            source.GetSize(&mut width, &mut height)?;
            if width == 0 || height == 0 || max_edge == 0 {
                return Err(Error::new(E_FAIL, "Invalid image dimensions"));
            }
            let scale = (max_edge as f64 / f64::from(width.max(height))).min(1.);
            if scale < 1. {
                width = (f64::from(width) * scale).round().max(1.) as u32;
                height = (f64::from(height) * scale).round().max(1.) as u32;
                let scaler = factory.CreateBitmapScaler()?;
                scaler.Initialize(&source, width, height, WICBitmapInterpolationModeFant)?;
                source = scaler.cast()?;
            }
            if u64::from(width) * u64::from(height) > 64_000_000 {
                return Err(Error::new(E_FAIL, "Decoded image is too large"));
            }
            let converter = factory.CreateFormatConverter()?;
            converter.Initialize(
                &source,
                &GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.,
                WICBitmapPaletteTypeCustom,
            )?;
            let mut result = Self::new(width, height);
            converter.CopyPixels(ptr::null(), width * 4, &mut result.bgra)?;
            Ok(result)
        }
    }
    pub fn save_png(&self, path: &Path) -> Result<()> {
        let _com = ComScope::new()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        }
        // SAFETY: all inputs remain alive until frame/encoder commits. This writes only the caller's file.
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = factory.CreateStream()?;
            stream.InitializeFromFilename(&HSTRING::from(path.as_os_str()), GENERIC_WRITE.0)?;
            let encoder = factory.CreateEncoder(&GUID_ContainerFormatPng, ptr::null())?;
            encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;
            let mut frame = None;
            let mut options = None;
            encoder.CreateNewFrame(&mut frame, &mut options)?;
            let frame =
                frame.ok_or_else(|| Error::new(E_FAIL, "WIC did not create an encoder frame"))?;
            frame.Initialize(options.as_ref())?;
            frame.SetSize(self.width, self.height)?;
            frame.SetResolution(96., 96.)?;
            let mut format = GUID_WICPixelFormat32bppBGRA;
            frame.SetPixelFormat(&mut format)?;
            if format != GUID_WICPixelFormat32bppBGRA {
                return Err(Error::new(E_FAIL, "WIC PNG encoder changed BGRA format"));
            }
            frame.WritePixels(self.height, self.width * 4, &self.bgra)?;
            frame.Commit()?;
            encoder.Commit()?;
        }
        Ok(())
    }
    pub fn sample(&self, x: f64, y: f64) -> [f64; 4] {
        let x = x.clamp(0., f64::from(self.width - 1));
        let y = y.clamp(0., f64::from(self.height - 1));
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let a = x - f64::from(x0);
        let b = y - f64::from(y0);
        std::array::from_fn(|channel| {
            let pixel = |px, py| {
                f64::from(self.bgra[((py * self.width + px) * 4) as usize + channel]) / 255.
            };
            (pixel(x0, y0) * (1. - a) + pixel(x1, y0) * a) * (1. - b)
                + (pixel(x0, y1) * (1. - a) + pixel(x1, y1) * a) * b
        })
    }
    pub fn resize_fill(&self, width: u32, height: u32) -> Self {
        let mut result = Self::new(width, height);
        let scale = (f64::from(width) / f64::from(self.width))
            .max(f64::from(height) / f64::from(self.height));
        let offset_x = (f64::from(self.width) * scale - f64::from(width)) * 0.5;
        let offset_y = (f64::from(self.height) * scale - f64::from(height)) * 0.5;
        for y in 0..height {
            for x in 0..width {
                let sample = self.sample(
                    (f64::from(x) + 0.5 + offset_x) / scale - 0.5,
                    (f64::from(y) + 0.5 + offset_y) / scale - 0.5,
                );
                let index = ((y * width + x) * 4) as usize;
                for (channel, value) in sample.iter().enumerate() {
                    result.bgra[index + channel] = (value * 255.).round() as u8;
                }
            }
        }
        result
    }
}
unsafe fn orientation(frame: &IWICBitmapFrameDecode) -> u32 {
    // SAFETY: inspect only the numeric metadata union member matching its variant tag, then free it.
    unsafe {
        let Ok(reader) = frame.GetMetadataQueryReader() else {
            return 1;
        };
        for name in [w!("/app1/ifd/{ushort=274}"), w!("/ifd/{ushort=274}")] {
            let mut value = PROPVARIANT::default();
            if reader.GetMetadataByName(name, &mut value).is_ok() {
                let data = &value.Anonymous.Anonymous;
                let orientation = if data.vt == VT_UI2 {
                    u32::from(data.Anonymous.uiVal)
                } else if data.vt == VT_UI4 {
                    data.Anonymous.ulVal
                } else {
                    1
                };
                let _ = PropVariantClear(&mut value);
                return orientation;
            }
        }
        1
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exif_all_eight_directions_and_embedded_srgb_profile() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/test");
        let original = Image::decode_file(&directory.join("orientation-1.jpg"), 1400).unwrap();
        let (w, h) = (original.width, original.height);
        for orientation in 1..=8 {
            let image = Image::decode_file(
                &directory.join(format!("orientation-{orientation}.jpg")),
                1400,
            )
            .unwrap();
            assert_eq!(
                (image.width, image.height),
                if orientation >= 5 { (h, w) } else { (w, h) }
            );
            for y in 0..image.height {
                for x in 0..image.width {
                    let (sx, sy) = match orientation {
                        1 => (x, y),
                        2 => (w - 1 - x, y),
                        3 => (w - 1 - x, h - 1 - y),
                        4 => (x, h - 1 - y),
                        5 => (y, x),
                        6 => (y, h - 1 - x),
                        7 => (w - 1 - y, h - 1 - x),
                        8 => (w - 1 - y, x),
                        _ => unreachable!(),
                    };
                    let source = ((sy * w + sx) * 4) as usize;
                    let target = ((y * image.width + x) * 4) as usize;
                    assert_eq!(
                        &image.bgra[target..target + 4],
                        &original.bgra[source..source + 4],
                        "EXIF {orientation} at {x},{y}"
                    );
                }
            }
        }
        let tagged = Image::decode_file(&directory.join("srgb-profile.jpg"), 1400).unwrap();
        assert_eq!((tagged.width, tagged.height), (w, h));
        assert!(
            tagged
                .bgra
                .iter()
                .zip(original.bgra.iter())
                .all(|(a, b)| a.abs_diff(*b) <= 2)
        );
    }
    #[test]
    fn png_round_trip_preserves_alpha_and_invalid_data_is_rejected() {
        let mut image = Image::new(3, 2);
        image.bgra.copy_from_slice(&[
            0, 0, 255, 255, 0, 255, 0, 128, 255, 0, 0, 0, 9, 8, 7, 255, 6, 5, 4, 100, 3, 2, 1, 255,
        ]);
        let path = std::env::temp_dir().join(format!("kilim-wic-test-{}.png", std::process::id()));
        image.save_png(&path).unwrap();
        let decoded = Image::decode_file(&path, 1400).unwrap();
        assert_eq!(decoded.width, 3);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.bgra, image.bgra);
        assert!(Image::decode(b"not an image", 1400).is_err());
        let bounded = Image::decode_file(&path, 1).unwrap();
        assert_eq!(bounded.width, 1);
        assert_eq!(bounded.height, 1);
    }
}
