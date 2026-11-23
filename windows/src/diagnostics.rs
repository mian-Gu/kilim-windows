//! Source-equivalent offline diagnostics. They never change the real user preferences/startup state.
use crate::{
    audio::{self, SoundSynth, ThumpDetector},
    cloth::{Cloth, N, NX, NY},
    image::Image,
    lighting::{LightEnv, LightMode},
    localization::{L10n, LANGUAGES, TEXT_COUNT, Text},
    random::Rng,
    renderer::Renderer,
    settings, texture,
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use windows::{
    Win32::Foundation::E_FAIL,
    core::{Error, Result},
};

pub fn run_if_requested(args: &[String]) -> Result<bool> {
    let data_directory = settings::directory_argument(args)
        .map_err(|error| failure(error.to_string()))?
        .unwrap_or_else(settings::data_directory);
    if args.iter().any(|arg| arg == "--fullscreen-bench") {
        fullscreen_benchmark(&data_directory)?;
        return Ok(true);
    }
    if args.iter().any(|arg| arg == "--audiotest") {
        audio_test()?;
        return Ok(true);
    }
    if args.iter().any(|arg| arg == "--audio-device-test") {
        audio::device_test().map_err(failure)?;
        println!("native audio output: opened, queued silence, closed");
        return Ok(true);
    }
    if args.iter().any(|arg| arg == "--langtest") {
        let errors = L10n::validate();
        if !errors.is_empty() {
            return Err(failure(errors.join(", ")));
        }
        println!(
            "all {} languages complete ({} strings each)",
            LANGUAGES.len(),
            TEXT_COUNT
        );
        for (code, _) in LANGUAGES {
            let language = L10n::new(code);
            println!(
                "{code} | {} | {}",
                language.text(Text::TossRug),
                language.theme("turk")
            );
        }
        return Ok(true);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--custom") {
        let input = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| failure("--custom requires an image path"))?;
        let photo = Image::decode_file(Path::new(input), 1400)?;
        let image = texture::custom(&photo);
        let path = output_argument(args)
            .unwrap_or_else(|| std::env::temp_dir().join("kilim-tex-custom.png"));
        image.save_png(&path)?;
        println!("custom: loaded=true, output={}", path.display());
        return Ok(true);
    }
    if args.iter().any(|arg| arg == "--texture") {
        let directory =
            output_argument(args).unwrap_or_else(|| std::env::temp_dir().join("kilim-textures"));
        std::fs::create_dir_all(&directory).map_err(|e| failure(e.to_string()))?;
        for theme in texture::PRESETS
            .iter()
            .map(|preset| preset.key)
            .chain(std::iter::once("custom"))
        {
            let start = Instant::now();
            let image = texture::generate(theme, &data_directory)?;
            let path = directory.join(format!("kilim-tex-{theme}.png"));
            image.save_png(&path)?;
            println!(
                "{theme}: {} ms {}",
                start.elapsed().as_millis(),
                path.display()
            );
        }
        return Ok(true);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--icon") {
        let theme = args.get(index + 1).map(String::as_str).unwrap_or("kashan");
        let path = args
            .get(index + 2)
            .filter(|value| !value.starts_with("--"))
            .map(PathBuf::from)
            .or_else(|| output_argument(args))
            .unwrap_or_else(|| std::env::temp_dir().join("kilim-icon.png"));
        let image = texture::generate(
            if texture::PRESETS.iter().any(|preset| preset.key == theme) || theme == "custom" {
                theme
            } else {
                "kashan"
            },
            &data_directory,
        )?;
        icon(&image).save_png(&path)?;
        println!("icon: {}", path.display());
        return Ok(true);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--selftest") {
        if args.iter().any(|arg| arg == "--bench") {
            benchmark(&data_directory)?;
            return Ok(true);
        }
        let directory = args
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("kilim-test"));
        self_test(
            &directory,
            args.iter().any(|arg| arg == "--debug"),
            &data_directory,
        )?;
        return Ok(true);
    }
    Ok(false)
}
fn output_argument(args: &[String]) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == "--output")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}
fn failure(message: impl AsRef<str>) -> Error {
    Error::new(E_FAIL, message.as_ref())
}
fn selected_texture(directory: &Path) -> Result<Image> {
    let preferences = settings::Settings::read(&directory.join("settings.ini"))
        .map_err(|error| failure(error.to_string()))?;
    let theme = if preferences.theme == "custom"
        || texture::PRESETS
            .iter()
            .any(|preset| preset.key == preferences.theme)
    {
        preferences.theme.as_str()
    } else {
        "kashan"
    };
    texture::generate(theme, directory)
}

fn audio_test() -> Result<()> {
    let mut synth = SoundSynth::default();
    for (label, slide, thump, seconds) in [
        ("silence", 0., 0., 0.5),
        ("slow slide", 0.2, 0., 1.),
        ("fast slide", 1., 0., 1.),
        ("soft thump", 0., 0.1, 0.6),
        ("hard thump", 0., 0.9, 0.6),
    ] {
        synth.slide_target = slide;
        if thump > 0. {
            synth.thump = thump;
        }
        let count = (44100. * seconds) as usize;
        let mut buffer = vec![0.; count];
        synth.render(&mut buffer);
        if buffer.iter().any(|value| !value.is_finite()) {
            return Err(failure("NaN/Inf in procedural sound"));
        }
        let peak = buffer.iter().map(|v| v.abs()).fold(0_f32, f32::max);
        let rms = (buffer.iter().map(|v| v * v).sum::<f32>() / count as f32).sqrt();
        if label.ends_with("slide") {
            let list: Vec<f32> = buffer[count / 4..]
                .as_chunks::<441>().0.iter()
                .map(|window| (window.iter().map(|v| v * v).sum::<f32>() / 441.).sqrt())
                .collect();
            let mean = list.iter().sum::<f32>() / list.len() as f32;
            let deviation = (list.iter().map(|value| (value - mean).powi(2)).sum::<f32>()
                / list.len() as f32)
                .sqrt();
            println!(
                "   loudness wobble (std/mean): {:.2}",
                deviation / mean.max(1e-6)
            );
        }
        println!("{label}: peak={peak:.3} rms={rms:.4} nan=false");
    }
    println!("{}", audio::self_test().map_err(failure)?);
    Ok(())
}
fn run(cloth: &mut Cloth, count: usize) {
    for _ in 0..count {
        cloth.step();
    }
}
pub fn self_test(directory: &Path, debug: bool, data_directory: &Path) -> Result<()> {
    std::fs::create_dir_all(directory).map_err(|e| failure(e.to_string()))?;
    let mut renderer = Renderer::with_texture([1440., 900.], &selected_texture(data_directory)?)?;
    let mut cloth = Cloth::new([1440., 900.]);
    let mut random = Rng::new(5);
    let mut icons = Vec::new();
    for index in 0..14 {
        icons.push([
            1150. + random.next() as f32 * 150.,
            150. + index as f32 * 38. + random.next() as f32 * 40.,
        ]);
    }
    for _ in 0..10 {
        icons.push([
            980. + random.next() as f32 * 120.,
            300. + random.next() as f32 * 220.,
        ]);
    }
    cloth.bumps = icons.clone();
    cloth.reset_pose([520., 460.], 560., 130., -0.12);
    if debug {
        bounding_box("init", &cloth);
        for step in 0..6 {
            cloth.step();
            bounding_box(&format!("step{step}"), &cloth);
        }
        return Ok(());
    }
    let day = LightEnv::NEUTRAL;
    run(&mut cloth, 5);
    snapshot(
        directory,
        "0_falling",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    run(&mut cloth, 240);
    snapshot(
        directory,
        "1_landed",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    if !cloth.begin_grab([760., 470.]) {
        return Err(failure("Drag scenario could not grab the rug"));
    }
    for step in 0..120 {
        let t = step as f32 / 119.;
        cloth.move_grab([760. + 340. * t, 470. + 40. * (t * 3.).sin()]);
        cloth.step();
    }
    snapshot(
        directory,
        "2_dragged",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    cloth.end_grab();
    run(&mut cloth, 480);
    snapshot(
        directory,
        "3_covering_icons",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    for (name, mode) in [
        ("day", LightMode::Day),
        ("evening", LightMode::Evening),
        ("night", LightMode::Night),
    ] {
        snapshot(
            directory,
            &format!("5_light_{name}"),
            &mut renderer,
            &cloth,
            1.,
            mode.environment(),
            &icons,
        )?;
    }
    snapshot(
        directory,
        "6_peek",
        &mut renderer,
        &cloth,
        0.26,
        LightMode::Day.environment(),
        &icons,
    )?;
    cloth.reset_pose([520., 460.], 560., 40., -0.1);
    run(&mut cloth, 200);
    snapshot(
        directory,
        "4a_flat_again",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    let edge = (NY / 2) * NX + NX - 5;
    let vertices = cloth.vertices();
    let p = project(vertices[edge].position, cloth.view);
    if !cloth.begin_grab(p) {
        return Err(failure("Fold scenario could not grab the rug"));
    }
    for step in 0..110 {
        let t = step as f32 / 109.;
        cloth.move_grab([p[0] - 400. * t, p[1] + 70. * t]);
        cloth.step();
    }
    snapshot(
        directory,
        "4_folding",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    cloth.end_grab();
    run(&mut cloth, 300);
    snapshot(
        directory,
        "5_folded_settled",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    cloth.reset_pose([1120., 360.], 620., 120., 0.08);
    run(&mut cloth, 400);
    snapshot(
        directory,
        "6_over_icons",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    cloth.reset_pose([560., 440.], 620., 30., -0.08);
    run(&mut cloth, 200);
    cloth.toggle_roll(true);
    for (count, name) in [
        (40, "7_roll_a"),
        (50, "7_roll_b"),
        (60, "7_roll_c"),
        (120, "7_roll_done"),
    ] {
        run(&mut cloth, count);
        snapshot(directory, name, &mut renderer, &cloth, 1., day, &icons)?;
    }
    cloth.toggle_roll(true);
    run(&mut cloth, 50);
    snapshot(
        directory,
        "7_unroll_mid",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    run(&mut cloth, 160);
    snapshot(
        directory,
        "7_unrolled",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    cloth.reset_pose([640., 450.], 480., 30., -0.08);
    run(&mut cloth, 200);
    cloth.scale(2.);
    run(&mut cloth, 120);
    snapshot(
        directory,
        "8_size_xl",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    cloth.scale(0.35);
    run(&mut cloth, 120);
    snapshot(
        directory,
        "8_size_s",
        &mut renderer,
        &cloth,
        1.,
        day,
        &icons,
    )?;
    fullscreen_drag_test(directory, data_directory)?;
    println!(
        "selftest complete: source and fullscreen drag scenes exported to {}",
        directory.display()
    );
    Ok(())
}

fn fullscreen_benchmark(data_directory: &Path) -> Result<()> {
    let texture = selected_texture(data_directory)?;
    for view in [[2560., 1600.], [4480., 1600.]] {
        let mut renderer = Renderer::with_texture(view, &texture)?;
        let mut cloth = Cloth::new(view);
        cloth.set_point_scale(1.25);
        cloth.reset_fullscreen(view, 80.);
        let mut environment = LightEnv::NEUTRAL;
        environment.tint[3] = 0.;
        renderer.render_sync(&cloth, 1., environment)?;
        let start = [view[0] * 0.5, view[1] * 0.5];
        if !cloth.begin_grab(start) {
            return Err(failure("Fullscreen benchmark must hit the sheet"));
        }
        let mut physics = Vec::new();
        let mut render = Vec::new();
        for step in 0..120 {
            let t = step as f32 / 119.;
            cloth.move_grab([start[0] - view[0] * 0.32 * t, start[1] - view[1] * 0.3 * t]);
            let clock = Instant::now();
            cloth.step();
            let simulation = clock.elapsed();
            let clock = Instant::now();
            renderer.render_sync(&cloth, 1., environment)?;
            let drawing = clock.elapsed();
            if simulation.as_secs_f64() + drawing.as_secs_f64() > 0.5 {
                return Err(failure(
                    "Fullscreen benchmark stopped: frame exceeded 500 ms",
                ));
            }
            physics.push(simulation.as_secs_f64() * 1000.);
            render.push(drawing.as_secs_f64() * 1000.);
            // Leave time for other applications; this benchmark does not run the desktop loop.
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
        for (name, mut samples) in [("physics", physics), ("render/readback", render)] {
            samples.sort_by(f64::total_cmp);
            println!(
                "fullscreen {view:?} {name}: mean={:.3} ms p95={:.3} ms max={:.3} ms",
                samples.iter().sum::<f64>() / samples.len() as f64,
                samples[113],
                samples[119],
            );
        }
    }
    Ok(())
}

fn fullscreen_drag_test(directory: &Path, data_directory: &Path) -> Result<()> {
    let texture = selected_texture(data_directory)?;
    for (name, view, start, travel, steps, scale) in [
        (
            "left",
            [1920., 1080.],
            [960., 540.],
            [-520., -300.],
            140,
            1.,
        ),
        (
            "wide",
            [3440., 1440.],
            [1720., 720.],
            [-1000., 520.],
            140,
            1.,
        ),
        (
            "corner",
            [2560., 1600.],
            [2480., 1440.],
            [-2280., -1200.],
            161,
            1.25,
        ),
    ] {
        let mut renderer = Renderer::with_texture(view, &texture)?;
        let mut cloth = Cloth::new(view);
        cloth.set_point_scale(scale);
        cloth.reset_fullscreen(view, 64. * scale);
        let mut environment = LightEnv::NEUTRAL;
        environment.tint[3] = 0.;
        if !cloth.begin_grab(start) {
            return Err(failure("Fullscreen drag must hit the sheet"));
        }
        let mut elapsed = std::time::Duration::ZERO;
        for step in 0..steps {
            let t = step as f32 / (steps - 1) as f32;
            cloth.move_grab([start[0] + travel[0] * t, start[1] + travel[1] * t]);
            let started = std::time::Instant::now();
            cloth.step();
            elapsed += started.elapsed();
            if [steps / 4 - 1, steps / 2 - 1, steps * 3 / 4 - 1, steps - 1].contains(&step) {
                snapshot(
                    directory,
                    &format!("9_fullscreen_{name}_{step}"),
                    &mut renderer,
                    &cloth,
                    1.,
                    environment,
                    &[],
                )?;
            }
        }
        println!(
            "fullscreen {name}: physics mean {:.3} ms/step",
            elapsed.as_secs_f64() * 1000. / steps as f64
        );
        cloth.end_grab();
        run(&mut cloth, 300);
        snapshot(
            directory,
            &format!("9_fullscreen_{name}_released"),
            &mut renderer,
            &cloth,
            1.,
            environment,
            &[],
        )?;
    }
    Ok(())
}

fn project(p: [f32; 3], view: [f32; 2]) -> [f32; 2] {
    [
        p[0] + (p[0] - view[0] * 0.5) * p[2] * 0.0009,
        p[1] + (p[1] - view[1] * 0.5) * p[2] * 0.0009,
    ]
}
fn snapshot(
    directory: &Path,
    name: &str,
    renderer: &mut Renderer,
    cloth: &Cloth,
    alpha: f32,
    environment: LightEnv,
    icons: &[[f32; 2]],
) -> Result<()> {
    if cloth
        .position
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(failure(format!("Nonfinite cloth in {name}")));
    }
    renderer.render_sync(cloth, alpha, environment)?;
    let rug = renderer.snapshot();
    let mut image = Image::new(rug.width, rug.height);
    for y in 0..image.height {
        for x in 0..image.width {
            let t = (f64::from(x) / f64::from(image.width)
                + f64::from(y) / f64::from(image.height))
                * 0.5;
            let index = ((y * image.width + x) * 4) as usize;
            image.bgra[index..index + 4].copy_from_slice(&[
                ((0.55 + (0.62 - 0.55) * t) * 255.) as u8,
                (0.30 * 255.) as u8,
                ((0.16 + (0.45 - 0.16) * t) * 255.) as u8,
                255,
            ]);
        }
    }
    for icon in icons {
        for y in (icon[1] as i32 - 28).max(0)..(icon[1] as i32 + 28).min(image.height as i32) {
            for x in (icon[0] as i32 - 22).max(0)..(icon[0] as i32 + 22).min(image.width as i32) {
                let index = ((y as u32 * image.width + x as u32) * 4) as usize;
                image.bgra[index..index + 4].copy_from_slice(&[247, 242, 242, 255]);
            }
        }
        for y in (icon[1] as i32 + 30).max(0)..(icon[1] as i32 + 42).min(image.height as i32) {
            for x in (icon[0] as i32 - 28).max(0)..(icon[0] as i32 + 28).min(image.width as i32) {
                let index = ((y as u32 * image.width + x as u32) * 4) as usize;
                image.bgra[index..index + 4].copy_from_slice(&[230, 102, 26, 255]);
            }
        }
    }
    for (index, pixel) in rug.bgra.as_chunks::<4>().0.iter().enumerate() {
        let a = f64::from(pixel[3]) / 255.;
        for (channel, value) in pixel[..3].iter().enumerate() {
            image.bgra[index * 4 + channel] = (f64::from(*value) * a
                + f64::from(image.bgra[index * 4 + channel]) * (1. - a))
                .round() as u8;
        }
    }
    image.save_png(&directory.join(format!("{name}.png")))?;
    println!(
        "wrote {name}.png (settled={} maxSpeed={:.3})",
        cloth.settled, cloth.max_speed
    );
    Ok(())
}
fn bounding_box(label: &str, cloth: &Cloth) {
    let minimum: [f32; 3] = std::array::from_fn(|axis| {
        cloth
            .position
            .iter()
            .map(|p| p[axis])
            .fold(f32::INFINITY, f32::min)
    });
    let maximum: [f32; 3] = std::array::from_fn(|axis| {
        cloth
            .position
            .iter()
            .map(|p| p[axis])
            .fold(f32::NEG_INFINITY, f32::max)
    });
    println!(
        "{label}: min={minimum:?} max={maximum:?} particles={N} constraints={}",
        cloth.constraint_count()
    );
}
fn benchmark(data_directory: &Path) -> Result<()> {
    let mut cloth = Cloth::new([2560., 1440.]);
    let mut random = Rng::new(9);
    cloth.bumps = (0..75)
        .map(|_| [random.next() as f32 * 2560., random.next() as f32 * 1440.])
        .collect();
    cloth.reset_pose([1000., 700.], 760., 130., -0.1);
    let mut detector = ThumpDetector::default();
    let mut first = 0;
    for step in 0..1200 {
        cloth.step();
        if detector
            .feed(cloth.impact_sum, step as f64 / 60., cloth.settled > 20)
            .is_some()
        {
            first += 1;
        }
    }
    println!("drop + rest (20 s): {first} thumps");
    let mut idle = 0;
    for step in 1200..3600 {
        cloth.step();
        if detector
            .feed(cloth.impact_sum, step as f64 / 60., cloth.settled > 20)
            .is_some()
        {
            idle += 1;
        }
    }
    println!("idle (40 s): {idle} thumps");
    if idle > 0 {
        return Err(failure("Idle cloth repeatedly triggers landing sounds"));
    }
    let p = project(cloth.position[NX * 15 + 22], cloth.view);
    let mut drag_thumps = Vec::new();
    if cloth.begin_grab(p) {
        for step in 0..240 {
            cloth.move_grab([
                p[0] + step as f32 * 3.,
                p[1] + (step as f32 / 20.).sin() * 80.,
            ]);
            cloth.step();
            if let Some(amplitude) = detector.feed(
                cloth.impact_sum,
                (3600 + step) as f64 / 60.,
                cloth.settled > 20,
            ) {
                drag_thumps.push(((3600 + step) as f64 / 60. - 60., amplitude));
            }
        }
        cloth.end_grab();
        let mut maximum: f32 = 0.;
        for step in 0..600 {
            cloth.step();
            maximum = maximum.max(cloth.impact_sum);
            if let Some(amplitude) = detector.feed(
                cloth.impact_sum,
                (3840 + step) as f64 / 60.,
                cloth.settled > 20,
            ) {
                drag_thumps.push(((3840 + step) as f64 / 60. - 60., amplitude));
            }
        }
        println!("max impactSum after release: {maximum:.3}");
    }
    println!("drag over bumps + release: thumps at {drag_thumps:?}");
    cloth.reset_pose([1000., 700.], 760., 130., -0.1);
    run(&mut cloth, 600);
    let start = Instant::now();
    run(&mut cloth, 300);
    println!(
        "sim step: {:.3} ms (75 bumps)",
        start.elapsed().as_secs_f64() * 1000. / 300.
    );
    cloth.bumps.clear();
    let start = Instant::now();
    run(&mut cloth, 300);
    println!(
        "sim step: {:.3} ms (no bumps)",
        start.elapsed().as_secs_f64() * 1000. / 300.
    );
    cloth.iterations = 0;
    let start = Instant::now();
    run(&mut cloth, 300);
    println!(
        "without solver: {:.3} ms",
        start.elapsed().as_secs_f64() * 1000. / 300.
    );
    cloth.iterations = 14;
    cloth.self_strength = 0.;
    cloth.self_thickness = 0.001;
    let start = Instant::now();
    run(&mut cloth, 300);
    println!(
        "without selfcollision: {:.3} ms",
        start.elapsed().as_secs_f64() * 1000. / 300.
    );
    cloth.self_strength = 0.5;
    cloth.self_thickness = 8.;
    let mut renderer = Renderer::with_texture([2560., 1440.], &selected_texture(data_directory)?)?;
    let environment = LightMode::Day.environment();
    renderer.render_sync(&cloth, 1., environment)?;
    let start = Instant::now();
    for _ in 0..20 {
        renderer.render_sync(&cloth, 1., environment)?;
        std::hint::black_box(renderer.snapshot());
    }
    println!(
        "render + full snapshot 2560x1440: {:.3} ms; crop={:?}",
        start.elapsed().as_secs_f64() * 1000. / 20.,
        renderer.allocation_size()
    );
    let start = Instant::now();
    for _ in 0..200 {
        std::hint::black_box(cloth.hit_node([1000., 700.]));
    }
    println!(
        "hitNode: {:.4} ms",
        start.elapsed().as_secs_f64() * 1000. / 200.
    );
    Ok(())
}

pub fn icon(texture: &Image) -> Image {
    let mut result = Image::new(1024, 1024);
    let side = f64::from(texture.width.min(texture.height));
    let crop_x = (f64::from(texture.width) - side) * 0.5;
    let crop_y = (f64::from(texture.height) - side) * 0.5;
    let distance = |x: f64, y: f64| {
        let qx = (x - 512.).abs() - (412. - 185.);
        let qy = (y - 512.).abs() - (412. - 185.);
        qx.max(0.).hypot(qy.max(0.)) + qx.max(qy).min(0.) - 185.
    };
    for y in 0..1024 {
        for x in 0..1024 {
            let px = f64::from(x) + 0.5;
            let py = f64::from(y) + 0.5;
            let d = distance(px, py);
            let shadow_distance = distance(px, py - 14.);
            let shadow = 0.38 * (-shadow_distance.max(0.).powi(2) / (2. * 14. * 14.)).exp();
            let coverage = (0.5 - d).clamp(0., 1.);
            let index = ((y * 1024 + x) * 4) as usize;
            let alpha = coverage + shadow * (1. - coverage);
            if alpha == 0. {
                continue;
            }
            let mut rgb = [0.1; 3];
            if coverage > 0. {
                let sample = texture.sample(
                    crop_x + (px - 100.) / 824. * side,
                    crop_y + (py - 100.) / 824. * side,
                );
                rgb = [
                    sample[2] * sample[3] + 0.1 * (1. - sample[3]),
                    sample[1] * sample[3] + 0.1 * (1. - sample[3]),
                    sample[0] * sample[3] + 0.1 * (1. - sample[3]),
                ];
                let t = ((px - 100.) + (py - 100.)) / (2. * 824.);
                if t < 0.5 {
                    let a = 0.28 * (1. - 2. * t).clamp(0., 1.);
                    rgb = rgb.map(|v| v * (1. - a) + a);
                } else {
                    let a = 0.30 * (2. * t - 1.).clamp(0., 1.);
                    rgb = rgb.map(|v| v * (1. - a));
                }
                let edge = (2. - d.abs()).clamp(0., 1.) * 0.35;
                rgb = rgb.map(|v| v * (1. - edge));
            }
            for (channel, value) in [rgb[2], rgb[1], rgb[0]].into_iter().enumerate() {
                result.bgra[index + channel] =
                    (value * coverage / alpha * 255.).round().clamp(0., 255.) as u8;
            }
            result.bgra[index + 3] = (alpha * 255.).round() as u8;
        }
    }
    result
}
