//! Audio/Sound.swift synth and landing detector, rendered on a dedicated native waveOut thread.
use std::{
    mem::size_of,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use windows::{Win32::Media::Audio::*, core::PSTR};

pub struct SoundSynth {
    pub slide_target: f32,
    pub thump: f32,
    pub volume: f32,
    pub sample_rate: f32,
    s1: f32,
    s2: f32,
    slide: f32,
    rng: u32,
    grain: f32,
    grain_target: f32,
    grain_count: i32,
    phase: f32,
    envelope: f32,
    amplitude: f32,
    click_lowpass: f32,
}
impl Default for SoundSynth {
    fn default() -> Self {
        Self {
            slide_target: 0.,
            thump: 0.,
            volume: 0.7,
            sample_rate: 44100.,
            s1: 0.,
            s2: 0.,
            slide: 0.,
            rng: 0x12345678,
            grain: 0.6,
            grain_target: 0.6,
            grain_count: 0,
            phase: 0.,
            envelope: 0.,
            amplitude: 0.,
            click_lowpass: 0.,
        }
    }
}
impl SoundSynth {
    fn noise(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng as i32) as f32 / 2_147_483_648.
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out {
            self.slide += (self.slide_target - self.slide) * 0.0006;
            if self.grain_count <= 0 {
                self.grain_target = 0.75 + 0.25 * self.noise().abs();
                self.grain_count = (self.sample_rate / (0.8 + 1.6 * self.noise().abs())) as i32;
            }
            self.grain_count -= 1;
            self.grain += (self.grain_target - self.grain) * 0.00035;
            let noise = self.noise();
            self.s1 += (noise - self.s1) * (0.05 + 0.32 * self.slide);
            self.s2 += (noise - self.s2) * 0.006;
            let rustle =
                (self.s1 - self.s2) * 1.05 * self.slide.powf(1.15) * (0.82 + 0.18 * self.grain);
            if self.thump > 0. {
                self.amplitude = self.thump;
                self.thump = 0.;
                self.envelope = 1.;
                self.phase = 0.;
            }
            let mut thump = 0.;
            if self.envelope > 0.0004 {
                let frequency = 38. + 52. * self.envelope * self.envelope;
                self.phase += 2. * std::f32::consts::PI * frequency / self.sample_rate;
                if self.phase > 2. * std::f32::consts::PI {
                    self.phase -= 2. * std::f32::consts::PI;
                }
                self.click_lowpass += (self.noise() - self.click_lowpass) * 0.07;
                let e6 = self.envelope.powi(6);
                thump = (self.phase.sin() * self.envelope * 0.9 + self.click_lowpass * e6 * 0.8)
                    * self.amplitude;
                self.envelope *= 0.99984;
            }
            *sample = ((rustle + thump) * self.volume).tanh();
        }
    }
}

pub struct ThumpDetector {
    window: u32,
    total: f32,
    last: f64,
    recent: Vec<f64>,
}
impl Default for ThumpDetector {
    fn default() -> Self {
        Self {
            window: 0,
            total: 0.,
            last: -10.,
            recent: Vec::new(),
        }
    }
}
impl ThumpDetector {
    pub fn cancel_pending(&mut self) {
        self.window = 0;
        self.total = 0.;
    }

    pub fn feed(&mut self, impact: f32, now: f64, calm: bool) -> Option<f32> {
        if impact > 150. && self.window == 0 && !calm {
            self.window = 6;
            self.total = 0.;
        }
        if self.window == 0 {
            return None;
        }
        self.total += impact;
        self.window -= 1;
        if self.window != 0 {
            return None;
        }
        let total = self.total;
        self.total = 0.;
        self.recent.retain(|time| now - time <= 4.);
        if now - self.last <= 0.6 || total <= 250. || self.recent.len() >= 3 {
            return None;
        }
        self.last = now;
        self.recent.push(now);
        Some((total / 3000.).min(1.).powf(0.6) * 0.9)
    }
}

enum Message {
    Wake,
    StopSoon,
    Slide(f32),
    Thump(f32),
    Enabled(bool),
    Shutdown,
}
pub struct RugSound {
    sender: mpsc::Sender<Message>,
    worker: Option<thread::JoinHandle<()>>,
    pub enabled: bool,
    errors: mpsc::Receiver<String>,
}
impl RugSound {
    pub fn new(enabled: bool) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let (error_sender, errors) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("kilim-audio".into())
            .spawn(move || audio_thread(receiver, error_sender, enabled))?;
        Ok(Self {
            sender,
            worker: Some(worker),
            enabled,
            errors,
        })
    }
    pub fn wake(&self) {
        if self.enabled {
            let _ = self.sender.send(Message::Wake);
        }
    }
    pub fn stop_soon(&self) {
        let _ = self.sender.send(Message::StopSoon);
    }
    pub fn slide(&self, level: f32) {
        if self.enabled {
            let _ = self.sender.send(Message::Slide(level.clamp(0., 1.)));
        }
    }
    pub fn thump(&self, amplitude: f32) {
        if self.enabled {
            let _ = self.sender.send(Message::Thump(amplitude.clamp(0., 1.)));
        }
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        let _ = self.sender.send(Message::Enabled(enabled));
    }
    pub fn error(&self) -> Option<String> {
        self.errors.try_recv().ok()
    }
}
impl Drop for RugSound {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[repr(C, align(8))]
struct Header(std::cell::UnsafeCell<WAVEHDR>);
struct Completion {
    addresses: [usize; 3],
    completed: std::sync::atomic::AtomicU8,
    active: std::sync::atomic::AtomicUsize,
    closed: std::sync::atomic::AtomicBool,
}
unsafe extern "system" fn wave_callback(
    _handle: HWAVEOUT,
    message: u32,
    instance: usize,
    parameter: usize,
    _unused: usize,
) {
    // The native callback only performs allocation-free atomic operations. No WinMM calls,
    // channel sends, locks or references to the packed WAVEHDR fields occur here.
    use std::sync::atomic::Ordering;
    unsafe {
        let state = &*(instance as *const Completion);
        state.active.fetch_add(1, Ordering::Acquire);
        if message == windows::Win32::Media::MM_WOM_DONE {
            if let Some(index) = state
                .addresses
                .iter()
                .position(|address| *address == parameter)
            {
                state.completed.fetch_or(1 << index, Ordering::Release);
            }
        } else if message == windows::Win32::Media::MM_WOM_CLOSE {
            state.closed.store(true, Ordering::Release);
        }
        state.active.fetch_sub(1, Ordering::Release);
    }
}
struct WaveDevice {
    handle: HWAVEOUT,
    headers: Box<[Header]>,
    buffers: Vec<Box<[f32]>>,
    callback: Option<Box<Completion>>,
    prepared: [bool; 3],
    available: u8,
    closed: bool,
}
impl WaveDevice {
    fn open() -> Result<Self, String> {
        let format = WAVEFORMATEX {
            wFormatTag: 3,
            nChannels: 1,
            nSamplesPerSec: 44100,
            nAvgBytesPerSec: 44100 * 4,
            nBlockAlign: 4,
            wBitsPerSample: 32,
            cbSize: 0,
        };
        let mut buffers: Vec<Box<[f32]>> =
            (0..3).map(|_| vec![0.; 512].into_boxed_slice()).collect();
        let headers: Box<[Header]> = (0..3)
            .map(|index| {
                Header(std::cell::UnsafeCell::new(WAVEHDR {
                    lpData: PSTR(buffers[index].as_mut_ptr().cast()),
                    dwBufferLength: (512 * size_of::<f32>()) as u32,
                    ..Default::default()
                }))
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let callback = Box::new(Completion {
            addresses: std::array::from_fn(|index| headers[index].0.get() as usize),
            completed: std::sync::atomic::AtomicU8::new(0),
            active: std::sync::atomic::AtomicUsize::new(0),
            closed: std::sync::atomic::AtomicBool::new(false),
        });
        let mut handle = HWAVEOUT::default();
        // SAFETY: boxed callback/header/sample addresses remain stable until native close and
        // all callbacks complete. CALLBACK_FUNCTION runs locally without app-thread references.
        let code = unsafe {
            waveOutOpen(
                Some(&mut handle),
                WAVE_MAPPER,
                &format,
                Some(wave_callback as *const () as usize),
                Some((&*callback as *const Completion) as usize),
                CALLBACK_FUNCTION,
            )
        };
        if code != 0 {
            return Err(format!("waveOutOpen failed ({code})"));
        }
        let mut device = Self {
            handle,
            headers,
            buffers,
            callback: Some(callback),
            prepared: [false; 3],
            available: 7,
            closed: false,
        };
        for index in 0..3 {
            let code = unsafe {
                waveOutPrepareHeader(
                    handle,
                    device.headers[index].0.get(),
                    size_of::<WAVEHDR>() as u32,
                )
            };
            if code != 0 {
                return Err(format!("waveOutPrepareHeader failed ({code})"));
            }
            device.prepared[index] = true;
        }
        Ok(device)
    }
    fn refill(&mut self, synth: &mut SoundSynth) -> Result<(), String> {
        use std::sync::atomic::Ordering;
        self.available |= self
            .callback
            .as_ref()
            .unwrap()
            .completed
            .swap(0, Ordering::Acquire);
        for index in 0..3 {
            if self.available & (1 << index) != 0 {
                // Acquire of WOM_DONE returns exclusive sample-buffer ownership to this thread.
                synth.render(&mut self.buffers[index]);
                let code = unsafe {
                    waveOutWrite(
                        self.handle,
                        self.headers[index].0.get(),
                        size_of::<WAVEHDR>() as u32,
                    )
                };
                if code != 0 {
                    return Err(format!("waveOutWrite failed ({code})"));
                }
                self.available &= !(1 << index);
            }
        }
        Ok(())
    }
    fn close(&mut self) -> Result<(), String> {
        use std::sync::atomic::Ordering;
        if self.closed {
            return Ok(());
        }
        // Reset returns all sample buffers before unprepare; packed headers are accessed
        // exclusively as raw pointers through UnsafeCell while WinMM owns their fields.
        let code = unsafe { waveOutReset(self.handle) };
        if code != 0 {
            return Err(format!("waveOutReset failed ({code})"));
        }
        for index in 0..3 {
            if self.prepared[index] {
                let code = unsafe {
                    waveOutUnprepareHeader(
                        self.handle,
                        self.headers[index].0.get(),
                        size_of::<WAVEHDR>() as u32,
                    )
                };
                if code != 0 {
                    return Err(format!("waveOutUnprepareHeader failed ({code})"));
                }
                self.prepared[index] = false;
            }
        }
        let code = unsafe { waveOutClose(self.handle) };
        if code != 0 {
            return Err(format!("waveOutClose failed ({code})"));
        }
        let start = Instant::now();
        let state = self.callback.as_ref().unwrap();
        while !state.closed.load(Ordering::Acquire) || state.active.load(Ordering::Acquire) != 0 {
            if start.elapsed() > Duration::from_millis(250) {
                return Err("Native audio close did not finish its callbacks".into());
            }
            thread::yield_now();
        }
        self.closed = true;
        Ok(())
    }
}
impl Drop for WaveDevice {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // Never free memory still potentially borrowed by a failed native driver. The audio
            // worker disables the failed output; these bounded allocations survive until process exit.
            crate::logging::write(format!(
                "audio shutdown: {error}; retaining native-owned buffers"
            ));
            eprintln!("Kilim audio shutdown: {error}");
            if let Some(callback) = self.callback.take() {
                std::mem::forget(callback);
            }
            std::mem::forget(std::mem::take(&mut self.buffers));
            std::mem::forget(std::mem::replace(
                &mut self.headers,
                Vec::new().into_boxed_slice(),
            ));
        }
    }
}
fn audio_thread(
    receiver: mpsc::Receiver<Message>,
    errors: mpsc::Sender<String>,
    mut enabled: bool,
) {
    let mut synth = SoundSynth::default();
    let mut device: Option<WaveDevice> = None;
    let mut wanted = false;
    let mut failed = false;
    let mut deadline: Option<Instant> = None;
    loop {
        let message = if device.is_some() {
            receiver.recv_timeout(Duration::from_millis(5)).ok()
        } else {
            receiver.recv().ok()
        };
        let Some(message) = message.or_else(|| {
            if device.is_some() {
                receiver.try_recv().ok()
            } else {
                Some(Message::Shutdown)
            }
        }) else {
            if deadline.is_some_and(|time| Instant::now() >= time) {
                device = None;
                deadline = None;
            }
            if let Some(output) = &mut device
                && let Err(error) = output.refill(&mut synth) {
                    let _ = errors.send(error);
                    device = None;
                    failed = true;
                }
            continue;
        };
        match message {
            Message::Shutdown => break,
            Message::Wake => {
                wanted = true;
                deadline = None;
            }
            Message::StopSoon => {
                wanted = false;
                synth.slide_target = 0.;
                if device.is_some() {
                    deadline = Some(Instant::now() + Duration::from_millis(1200));
                }
            }
            Message::Slide(level) => synth.slide_target = level,
            Message::Thump(amplitude) => {
                synth.thump = amplitude;
                wanted = true;
                deadline = None;
            }
            Message::Enabled(value) => {
                enabled = value;
                failed = false;
                if !enabled {
                    device = None;
                    synth.slide_target = 0.;
                    synth.thump = 0.;
                    deadline = None;
                }
            }
        }
        if enabled && wanted && !failed && device.is_none() {
            match WaveDevice::open() {
                Ok(output) => device = Some(output),
                Err(error) => {
                    let _ = errors.send(error);
                    failed = true;
                }
            }
        }
        if deadline.is_some_and(|time| Instant::now() >= time) {
            device = None;
            deadline = None;
        }
        if let Some(output) = &mut device
            && let Err(error) = output.refill(&mut synth) {
                let _ = errors.send(error);
                device = None;
                failed = true;
            }
    }
}

pub fn self_test() -> Result<String, String> {
    let mut synth = SoundSynth::default();
    let mut samples = [0.; 512];
    let mut peak: f32 = 0.;
    let mut energy = 0.;
    for block in 0..1200 {
        synth.slide_target = if block < 600 { block as f32 / 600. } else { 0. };
        if block == 600 {
            synth.thump = 0.9;
        }
        synth.render(&mut samples);
        for sample in samples {
            if !sample.is_finite() {
                return Err("NaN/Inf in sound synth".into());
            }
            peak = peak.max(sample.abs());
            energy += f64::from(sample * sample);
        }
    }
    if peak <= 0.01 || peak > 1. || energy <= 0. {
        return Err("Sound synth is silent or clips".into());
    }
    Ok(format!(
        "audio: frames={}, peak={peak:.4}, rms={:.4}, nan=false",
        1200 * 512,
        (energy / (1200. * 512.)).sqrt()
    ))
}
pub fn device_test() -> Result<(), String> {
    let mut device = WaveDevice::open()?;
    let mut synth = SoundSynth {
        volume: 0.,
        ..Default::default()
    };
    device.refill(&mut synth)?;
    std::thread::sleep(Duration::from_millis(50));
    device.close()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_slide_decays_quickly_and_can_resume() {
        let mut synth = SoundSynth {
            slide_target: 0.8,
            ..Default::default()
        };
        let mut samples = [0.; 4410];
        synth.render(&mut samples);
        synth.render(&mut samples);
        let energy = |samples: &[f32]| samples.iter().map(|x| x * x).sum::<f32>();
        let before = energy(&samples);
        synth.slide_target = 0.;
        synth.render(&mut samples);
        synth.render(&mut samples);
        assert!(energy(&samples) < before * 0.001);
        synth.slide_target = 0.8;
        synth.render(&mut samples);
        assert!(energy(&samples) > before * 0.25);
    }

    #[test]
    fn audio_is_finite_and_landing_detector_limits_repeats() {
        assert!(self_test().is_ok());
        let mut detector = ThumpDetector::default();
        for step in 0..120 {
            assert!(detector.feed(2000., step as f64 / 60., true).is_none());
        }
        let mut triggers = 0;
        for step in 0..180 {
            if detector
                .feed(2000., 3. + step as f64 / 60., false)
                .is_some()
            {
                triggers += 1;
            }
        }
        assert_eq!(triggers, 3);
    }
}
