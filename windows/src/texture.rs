//! Faithful deterministic presets: original geometry/wear/fringe at build time,
//! original per-pixel weave restored in Rust. Custom photos are generated natively.
use crate::image::Image;
use crate::random::Rng;
use std::{
    f64::consts::PI,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};
use windows::Win32::Foundation::E_FAIL;
use windows::core::{Error, Result};

pub const WIDTH: u32 = 1536;
pub const HEIGHT: u32 = 1032;
const SCALE: f64 = 1.2;
const CUSTOM_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
pub struct Preset {
    pub key: &'static str,
    pub base: &'static [u8],
    pub fringe: &'static [u8],
    pub seed: u64,
}
include!("../assets/patterns.rs");

pub fn generate(theme: &str, directory: &Path) -> Result<Image> {
    if theme == "custom" {
        return Ok(match saved_photo(directory).as_ref() {
            Some(photo) => custom(photo),
            None => custom_internal(None),
        });
    }
    let preset = PRESETS
        .iter()
        .find(|preset| preset.key == theme)
        .ok_or_else(|| Error::new(E_FAIL, "Unknown rug pattern"))?;
    let mut image = Image::decode(preset.base, WIDTH)?;
    if image.width != WIDTH || image.height != HEIGHT {
        return Err(Error::new(E_FAIL, "Invalid built-in pattern dimensions"));
    }
    weave(&mut image, &mut Rng(preset.seed));
    let fringe = Image::decode(preset.fringe, WIDTH)?;
    if fringe.width != WIDTH || fringe.height != HEIGHT {
        return Err(Error::new(E_FAIL, "Invalid fringe dimensions"));
    }
    let mut raster = Raster(image);
    for (index, pixel) in fringe.bgra.as_chunks::<4>().0.iter().enumerate() {
        if pixel[3] != 0 {
            raster.blend(
                index,
                [
                    f64::from(pixel[2]) / 255.,
                    f64::from(pixel[1]) / 255.,
                    f64::from(pixel[0]) / 255.,
                    f64::from(pixel[3]) / 255.,
                ],
                1.,
            );
        }
    }
    Ok(raster.0)
}
fn import_custom(path: &Path, directory: &Path) -> Result<(Image, Image)> {
    let photo = Image::decode_file(path, 1400)?;
    let image = custom(&photo);
    std::fs::create_dir_all(directory).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
    let staging = directory.join("custom.pending.png");
    photo.save_png(&staging)?;
    // SAFETY: only replaces this app's saved custom photo after a complete successful encode.
    unsafe {
        windows::Win32::Storage::FileSystem::MoveFileExW(
            &windows::core::HSTRING::from(staging.as_os_str()),
            &windows::core::HSTRING::from(directory.join("custom.png").as_os_str()),
            windows::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                | windows::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
        )?;
    }
    Ok((image, photo))
}
pub fn saved_photo(directory: &Path) -> Option<Image> {
    let path = directory.join("custom.png");
    if !path.is_file() {
        return None;
    }
    match Image::decode_file(&path, 1400) {
        Ok(photo) => Some(photo),
        Err(error) => {
            crate::logging::write(format!("saved photo unavailable: {error}"));
            None
        }
    }
}
pub fn placeholder() -> Image {
    custom_internal(None)
}
fn weave(image: &mut Image, rng: &mut Rng) {
    weave_columns(image, rng, (64. * SCALE) as u32..(1216. * SCALE) as u32);
}
fn weave_columns(image: &mut Image, rng: &mut Rng, columns: std::ops::Range<u32>) {
    for y in 0..HEIGHT {
        for x in columns.clone() {
            let mut noise = (rng.next() - 0.5) * 18.;
            noise += if y & 1 == 0 { 4. } else { -4. };
            noise += if ((x >> 1) + (y >> 1)) & 1 == 0 {
                3.
            } else {
                -3.
            };
            let delta = noise as i32;
            let index = ((y * WIDTH + x) * 4) as usize;
            for channel in 0..3 {
                image.bgra[index + channel] =
                    (i32::from(image.bgra[index + channel]) + delta).clamp(0, 255) as u8;
            }
        }
    }
}

type Color = [f64; 4];
fn color(rgb: [f64; 3]) -> Color {
    [
        rgb[0].clamp(0., 1.),
        rgb[1].clamp(0., 1.),
        rgb[2].clamp(0., 1.),
        1.,
    ]
}
fn mix(a: [f64; 3], b: [f64; 3], amount: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * amount)
}
fn multiply(a: [f64; 3], scalar: f64) -> [f64; 3] {
    a.map(|x| x * scalar)
}
struct Palette {
    red: Color,
    red2: Color,
    red3: Color,
    navy: Color,
    navy2: Color,
    cream: Color,
    gold: Color,
    ink: Color,
}
impl Palette {
    fn original() -> Self {
        let hex = |value: u32| {
            color([
                f64::from((value >> 16) & 255) / 255.,
                f64::from((value >> 8) & 255) / 255.,
                f64::from(value & 255) / 255.,
            ])
        };
        Self {
            red: hex(0x9A2317),
            red2: hex(0x6B120C),
            red3: hex(0xC4472C),
            navy: hex(0x131A3B),
            navy2: hex(0x25325F),
            cream: hex(0xEBDAB5),
            gold: hex(0xD3A440),
            ink: hex(0x0D0A1B),
        }
    }
    fn from_photo(photo: &Image) -> Self {
        let mut average = [0.; 3];
        let mut vivid = [0.6, 0.2, 0.15];
        let mut vivid_score = -1.;
        let mut dark = [0.1, 0.1, 0.15];
        let mut dark_luma = 2.;
        let mut light = [0.9, 0.85, 0.7];
        let mut light_luma = -1.;
        for y in 0..20 {
            for x in 0..20 {
                let pixel = photo.sample(
                    (f64::from(x) + 0.5) * f64::from(photo.width) / 20. - 0.5,
                    (f64::from(y) + 0.5) * f64::from(photo.height) / 20. - 0.5,
                );
                let rgb = [
                    pixel[2] * pixel[3],
                    pixel[1] * pixel[3],
                    pixel[0] * pixel[3],
                ];
                for i in 0..3 {
                    average[i] += rgb[i];
                }
                let maximum = rgb[0].max(rgb[1]).max(rgb[2]);
                let minimum = rgb[0].min(rgb[1]).min(rgb[2]);
                let saturation = (maximum - minimum) * (0.4 + maximum);
                let luma = 0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2];
                if saturation > vivid_score {
                    vivid_score = saturation;
                    vivid = rgb;
                }
                if luma < dark_luma {
                    dark_luma = luma;
                    dark = rgb;
                }
                if luma > light_luma {
                    light_luma = luma;
                    light = rgb;
                }
            }
        }
        average = average.map(|x| x / 400.);
        let white = [1.; 3];
        Self {
            red: color(multiply(vivid, 0.95)),
            red2: color(multiply(vivid, 0.55)),
            red3: color(mix(vivid, white, 0.3)),
            navy: color(multiply(mix(dark, average, 0.25), 0.7)),
            navy2: color(multiply(mix(dark, average, 0.5), 0.9)),
            cream: color(multiply(mix(light, white, 0.25), 0.97)),
            gold: color(mix(vivid, light, 0.55)),
            ink: color(multiply(dark, 0.3)),
        }
    }
}
/// User photo printed across the whole rug: the picture itself is the weave, so no border ring,
/// no decoration and no fringe are drawn. The result is a plain opaque rectangle.
pub fn custom(photo: &Image) -> Image {
    let mut image = photo.resize_fill(WIDTH, HEIGHT);
    for pixel in image.bgra.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    weave_columns(&mut image, &mut Rng(CUSTOM_SEED), 0..WIDTH);
    image
}
fn custom_internal(photo: Option<&Image>) -> Image {
    let p = photo
        .map(Palette::from_photo)
        .unwrap_or_else(Palette::original);
    let mut raster = Raster(Image::new(WIDTH, HEIGHT));
    raster.rect(64., 0., 1152., 860., p.navy);
    for (inset, fill) in [
        (8., p.cream),
        (11., p.ink),
        (13., p.red),
        (33., p.cream),
        (36., p.ink),
        (37., p.navy),
        (139., p.ink),
        (141., p.cream),
        (147., p.ink),
        (149., p.red2),
        (153., p.ink),
        (155., p.cream),
        (157., p.ink),
    ] {
        raster.rect(
            64. + inset,
            inset,
            1152. - 2. * inset,
            860. - 2. * inset,
            fill,
        );
    }
    for (fixed, angle, horizontal) in [
        (88., 0., true),
        (772., PI, true),
        (152., -PI / 2., false),
        (1128., PI / 2., false),
    ] {
        let (lo, hi) = if horizontal {
            (176., 1104.)
        } else {
            (112., 748.)
        };
        let n = ((hi - lo) / 34_f64).round() as usize;
        for i in 0..=n {
            let t = lo + (hi - lo) * i as f64 / n as f64;
            let center = if horizontal { [t, fixed] } else { [fixed, t] };
            for (radius, vertical, fill, stroke) in [
                (
                    12.,
                    17.,
                    if i % 2 == 0 { p.cream } else { p.red3 },
                    Some((p.ink, 1.)),
                ),
                (5., 8., if i % 2 == 0 { p.red } else { p.navy2 }, None),
            ] {
                let points = [[0., -vertical], [radius, 0.], [0., vertical], [-radius, 0.]]
                    .map(|point| transform(point, center, angle));
                raster.polygon(&points, fill, stroke);
            }
        }
    }
    for center in [[152., 88.], [1128., 88.], [152., 772.], [1128., 772.]] {
        for (r1, r2, rotation, fill, width) in [
            (44., 24., PI / 8., p.cream, 1.6),
            (24., 12., 0., p.red, 1.2),
        ] {
            let points: Vec<_> = (0..16)
                .map(|i| {
                    let radius = if i % 2 == 0 { r1 } else { r2 };
                    let angle = rotation - PI / 2. + i as f64 * PI / 8.;
                    [
                        center[0] + radius * angle.cos(),
                        center[1] + radius * angle.sin(),
                    ]
                })
                .collect();
            raster.polygon(&points, fill, Some((p.ink, width)));
        }
        raster.circle(center, 5., p.gold, Some((p.ink, 1.)));
    }
    let cell = 5.5;
    let cols = (834_f64 / cell) as u32;
    let rows = (542_f64 / cell) as u32;
    let field = if let Some(photo) = photo {
        photo.resize_fill(cols, rows)
    } else {
        let mut field = Image::new(cols, rows);
        for row in 0..rows {
            for col in 0..cols {
                let index = ((row * cols + col) * 4) as usize;
                field.bgra[index..index + 4].copy_from_slice(&[
                    150,
                    (80 + 80 * row / rows) as u8,
                    (60 + 150 * col / cols) as u8,
                    255,
                ]);
            }
        }
        field
    };
    raster.rect(223., 159., 834., 542., p.ink);
    for row in 0..rows {
        for column in 0..cols {
            let index = ((row * cols + column) * 4) as usize;
            let alpha = f64::from(field.bgra[index + 3]) / 255.;
            let rgb = [
                f64::from(field.bgra[index + 2]) * alpha / 255.,
                f64::from(field.bgra[index + 1]) * alpha / 255.,
                f64::from(field.bgra[index]) * alpha / 255.,
            ];
            let luma = 0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2];
            let rgb =
                rgb.map(|value| (((luma + (value - luma) * 1.18).clamp(0., 1.) * 5.).round()) / 5.);
            raster.rect(
                223. + f64::from(column) * cell,
                159. + f64::from(row) * cell,
                cell - 0.7,
                cell - 0.7,
                color(rgb),
            );
        }
    }
    let mut rng = Rng(42);
    for _ in 0..30 {
        let center = [64. + rng.next() * 1152., rng.next() * 860.];
        let radius = 50. + rng.next() * 150.;
        let dark = rng.next() < 0.55;
        raster.radial_wear(
            center,
            radius,
            if dark {
                [0.1, 0.03, 0., 0.10]
            } else {
                [1., 0.85, 0.6, 0.07]
            },
        );
    }
    weave(&mut raster.0, &mut rng);
    let mut y = 3.;
    while y < 860. {
        for side in [-1., 1.] {
            let base = if side < 0. { 67. } else { 1213. };
            let length = 36. + rng.next() * 24.;
            let sway = (rng.next() - 0.5) * 14.;
            let light = 0.86 + rng.next() * 0.14;
            let control = [
                [base, y],
                [base + side * length * 0.35, y + sway * 0.1],
                [base + side * length * 0.7, y + sway * 0.7],
                [base + side * length, y + sway],
            ];
            let points: Vec<_> = (0..=24)
                .map(|i| {
                    let t = i as f64 / 24.;
                    let u = 1. - t;
                    std::array::from_fn(|axis| {
                        u.powi(3) * control[0][axis]
                            + 3. * u * u * t * control[1][axis]
                            + 3. * u * t * t * control[2][axis]
                            + t.powi(3) * control[3][axis]
                    })
                })
                .collect();
            raster.stroke(
                &points,
                3.1,
                [0.93 * light, 0.86 * light, 0.71 * light, 1.],
                false,
            );
        }
        y += 5.6;
    }
    raster.0
}
fn transform(p: [f64; 2], center: [f64; 2], angle: f64) -> [f64; 2] {
    [
        center[0] + p[0] * angle.cos() - p[1] * angle.sin(),
        center[1] + p[0] * angle.sin() + p[1] * angle.cos(),
    ]
}
struct Raster(Image);
impl Raster {
    fn blend(&mut self, index: usize, color: Color, coverage: f64) {
        let offset = index * 4;
        let alpha = color[3] * coverage;
        if alpha <= 0. {
            return;
        }
        let old_alpha = f64::from(self.0.bgra[offset + 3]) / 255.;
        let result_alpha = alpha + old_alpha * (1. - alpha);
        for (channel, value) in [color[2], color[1], color[0]].into_iter().enumerate() {
            let old = f64::from(self.0.bgra[offset + channel]) / 255.;
            self.0.bgra[offset + channel] =
                (((value * alpha + old * old_alpha * (1. - alpha)) / result_alpha) * 255.)
                    .round()
                    .clamp(0., 255.) as u8;
        }
        self.0.bgra[offset + 3] = (result_alpha * 255.).round() as u8;
    }
    fn bounds(&self, points: &[[f64; 2]], padding: f64) -> (u32, u32, u32, u32) {
        let minimum = std::array::from_fn::<_, 2, _>(|axis| {
            points
                .iter()
                .map(|p| p[axis] * SCALE)
                .fold(f64::INFINITY, f64::min)
        });
        let maximum = std::array::from_fn::<_, 2, _>(|axis| {
            points
                .iter()
                .map(|p| p[axis] * SCALE)
                .fold(f64::NEG_INFINITY, f64::max)
        });
        (
            (minimum[0] - padding).floor().max(0.) as u32,
            (minimum[1] - padding).floor().max(0.) as u32,
            (maximum[0] + padding).ceil().clamp(0., f64::from(WIDTH)) as u32,
            (maximum[1] + padding).ceil().clamp(0., f64::from(HEIGHT)) as u32,
        )
    }
    fn rect(&mut self, x: f64, y: f64, width: f64, height: f64, color: Color) {
        let left = x * SCALE;
        let top = y * SCALE;
        let right = (x + width) * SCALE;
        let bottom = (y + height) * SCALE;
        let (x0, y0, x1, y1) = self.bounds(&[[x, y], [x + width, y + height]], 0.);
        for py in y0..y1 {
            for px in x0..x1 {
                let coverage = (right.min(f64::from(px) + 1.) - left.max(f64::from(px)))
                    .clamp(0., 1.)
                    * (bottom.min(f64::from(py) + 1.) - top.max(f64::from(py))).clamp(0., 1.);
                self.blend((py * WIDTH + px) as usize, color, coverage);
            }
        }
    }
    fn polygon(&mut self, points: &[[f64; 2]], fill: Color, stroke: Option<(Color, f64)>) {
        let (x0, y0, x1, y1) = self.bounds(points, 1.);
        for y in y0..y1 {
            for x in x0..x1 {
                let mut coverage = 0.;
                for oy in [0.25, 0.75] {
                    for ox in [0.25, 0.75] {
                        let px = (f64::from(x) + ox) / SCALE;
                        let py = (f64::from(y) + oy) / SCALE;
                        let mut inside = false;
                        let mut previous = points.len() - 1;
                        for (i, p) in points.iter().enumerate() {
                            let q = points[previous];
                            if (p[1] > py) != (q[1] > py)
                                && px < (q[0] - p[0]) * (py - p[1]) / (q[1] - p[1]) + p[0]
                            {
                                inside = !inside;
                            }
                            previous = i;
                        }
                        if inside {
                            coverage += 0.25;
                        }
                    }
                }
                self.blend((y * WIDTH + x) as usize, fill, coverage);
            }
        }
        if let Some((color, width)) = stroke {
            self.stroke(points, width, color, true);
        }
    }
    fn stroke(&mut self, points: &[[f64; 2]], width: f64, color: Color, closed: bool) {
        let radius = width * SCALE * 0.5;
        let (x0, y0, x1, y1) = self.bounds(points, radius + 1.);
        for y in y0..y1 {
            for x in x0..x1 {
                let pixel = [(f64::from(x) + 0.5) / SCALE, (f64::from(y) + 0.5) / SCALE];
                let mut distance = f64::INFINITY;
                let segments = if closed {
                    points.len()
                } else {
                    points.len() - 1
                };
                for i in 0..segments {
                    let a = points[i];
                    let b = points[(i + 1) % points.len()];
                    let delta = [b[0] - a[0], b[1] - a[1]];
                    let length = delta[0] * delta[0] + delta[1] * delta[1];
                    let t = if length > 0. {
                        ((pixel[0] - a[0]) * delta[0] + (pixel[1] - a[1]) * delta[1]) / length
                    } else {
                        0.
                    };
                    let t = t.clamp(0., 1.);
                    distance = distance.min(
                        (pixel[0] - a[0] - t * delta[0]).hypot(pixel[1] - a[1] - t * delta[1])
                            * SCALE,
                    );
                }
                self.blend(
                    (y * WIDTH + x) as usize,
                    color,
                    (radius + 0.5 - distance).clamp(0., 1.),
                );
            }
        }
    }
    fn circle(&mut self, center: [f64; 2], radius: f64, fill: Color, stroke: Option<(Color, f64)>) {
        let points: Vec<_> = (0..48)
            .map(|i| {
                let a = i as f64 * PI / 24.;
                [center[0] + a.cos() * radius, center[1] + a.sin() * radius]
            })
            .collect();
        self.polygon(&points, fill, stroke);
    }
    fn radial_wear(&mut self, center: [f64; 2], radius: f64, color: Color) {
        let (x0, y0, x1, y1) = self.bounds(
            &[
                [center[0] - radius, center[1] - radius],
                [center[0] + radius, center[1] + radius],
            ],
            0.,
        );
        for y in y0..y1 {
            for x in x0..x1 {
                if f64::from(x) < 64. * SCALE || f64::from(x) >= 1216. * SCALE {
                    continue;
                }
                let distance = ((f64::from(x) + 0.5) / SCALE - center[0])
                    .hypot((f64::from(y) + 0.5) / SCALE - center[1]);
                if distance < radius {
                    self.blend((y * WIDTH + x) as usize, color, 1. - distance / radius);
                }
            }
        }
    }
}

pub struct Completed {
    pub id: u64,
    pub theme: String,
    pub result: Result<Image>,
    pub has_custom: bool,
}
enum Request {
    Generate(u64, String, Option<PathBuf>),
    Stop,
}
pub struct Worker {
    sender: mpsc::Sender<Request>,
    receiver: mpsc::Receiver<Completed>,
    worker: Option<thread::JoinHandle<()>>,
    sequence: u64,
}
impl Worker {
    pub fn new(directory: PathBuf, mut photo: Option<Image>) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kilim-textures".into())
            .spawn(move || {
                while let Ok(request) = rx.recv() {
                    let Request::Generate(id, theme, path) = request else {
                        break;
                    };
                    let result = if let Some(path) = path {
                        match import_custom(&path, &directory) {
                            Ok((image, new_photo)) => {
                                photo = Some(new_photo);
                                Ok(image)
                            }
                            Err(error) => Err(error),
                        }
                    } else if theme == "custom" {
                        Ok(match photo.as_ref() {
                            Some(photo) => custom(photo),
                            None => custom_internal(None),
                        })
                    } else {
                        generate(&theme, &directory)
                    };
                    if done_tx
                        .send(Completed {
                            id,
                            theme,
                            result,
                            has_custom: photo.is_some(),
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: tx,
            receiver: done_rx,
            worker: Some(worker),
            sequence: 0,
        })
    }
    pub fn request(&mut self, theme: &str, path: Option<PathBuf>) -> u64 {
        self.sequence += 1;
        let _ = self
            .sender
            .send(Request::Generate(self.sequence, theme.into(), path));
        self.sequence
    }
    pub fn poll(&self) -> Option<Completed> {
        self.receiver.try_recv().ok()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Stop);
        self.worker.take(); // A slow file/codec must not hold the UI shutdown hostage.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_presets_reconstruct_full_size_alpha_textures() {
        assert_eq!(PRESETS.len(), 15);
        for preset in PRESETS {
            let image = generate(preset.key, Path::new(".")).unwrap();
            assert_eq!((image.width, image.height), (WIDTH, HEIGHT));
            assert_eq!(image.bgra[3], 0);
            assert!(image.bgra.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 255));
        }
    }
    #[test]
    fn custom_photo_prints_edge_to_edge_without_the_border_ring() {
        let mut photo = Image::new(40, 20);
        for pixel in photo.bgra.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[0, 200, 0, 255]);
        }
        let printed = custom(&photo);
        let bordered = custom_internal(Some(&photo));
        let expected = photo.resize_fill(WIDTH, HEIGHT);
        let mut sampled = 0;
        let mut repainted = 0;
        for y in [140u32, HEIGHT / 2, HEIGHT - 140] {
            for x in [140u32, WIDTH / 2, WIDTH - 140] {
                let index = ((y * WIDTH + x) * 4) as usize;
                if printed.bgra[index + 3] == 0 {
                    continue;
                }
                sampled += 1;
                for channel in 0..3 {
                    let difference = i32::from(printed.bgra[index + channel])
                        - i32::from(expected.bgra[index + channel]);
                    assert!(
                        difference.abs() <= 40,
                        "({x},{y}) does not carry the photo: {difference}"
                    );
                }
                let delta = (0..3)
                    .map(|channel| {
                        (i32::from(printed.bgra[index + channel])
                            - i32::from(bordered.bgra[index + channel]))
                        .abs()
                    })
                    .max()
                    .unwrap_or(0);
                if delta > 25 {
                    repainted += 1;
                }
            }
        }
        assert!(sampled >= 7, "silhouette lost too much area: {sampled}");
        assert!(
            repainted >= 4,
            "the previous layout should have differed at these spots: {repainted}"
        );
    }
    #[test]
    fn custom_picture_prints_opaque_edge_to_edge() {
        let mut photo = Image::new(20, 10);
        for pixel in photo.bgra.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[30, 60, 220, 255]);
        }
        let image = custom(&photo);
        assert_eq!(image.bgra[3], 255, "no transparent fringe on the corners");
        let middle = ((HEIGHT / 2 * WIDTH + WIDTH / 2) * 4) as usize;
        assert_eq!(image.bgra[middle + 3], 255);
        assert!(image.bgra[middle + 2] > image.bgra[middle]);
    }
}
