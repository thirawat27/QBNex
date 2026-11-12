//! Audio output through cpal (replaces miniaudio's device layer).
//!
//! The cpal stream is created, played and dropped on a dedicated thread
//! because streams are not `Send` on every platform. The callback renders
//! stereo f32 through a caller-supplied function (the engine mixer, which
//! takes the engine lock) and maps it to the device's channel count and
//! sample format.
//!
//! Without a usable output device, [`Device::open_null`] runs the same mix
//! against the wall clock and discards it (a silent device), so sound timing
//! and the music queue behave as they do with speakers.

pub type RenderFn = std::sync::Arc<dyn Fn(&mut [f32]) + Send + Sync>;

#[cfg(target_arch = "wasm32")]
pub use web::{web_active, web_render, Device, WEB_RATE};

/// Browser build: no device thread. The page pulls stereo f32
/// from the mixer ([`web_render`]) and plays it through WebAudio.
#[cfg(target_arch = "wasm32")]
mod web {
    use super::RenderFn;
    use std::sync::Mutex;

    /// Mixer rate in the browser (WebAudio resamples to the device).
    pub const WEB_RATE: u32 = 44_100;

    static RENDER: Mutex<Option<RenderFn>> = Mutex::new(None);

    pub struct Device {
        pub rate: u32,
    }

    impl Device {
        pub fn open(render: RenderFn) -> Option<Device> {
            *RENDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(render);
            Some(Device { rate: WEB_RATE })
        }

        /// The page pulls the mix in any case, so the silent clock is the same device.
        pub fn open_null(render: RenderFn) -> Option<Device> {
            Device::open(render)
        }

        pub fn close(self) {
            *RENDER.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    /// A program has started sound (the mixer has something to render).
    pub fn web_active() -> bool {
        RENDER.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// Fills `out` (interleaved stereo) with the next frames of the mix;
    /// false (and silence) while no sound has been started.
    pub fn web_render(out: &mut [f32]) -> bool {
        let r = RENDER.lock().unwrap_or_else(|e| e.into_inner()).clone();
        match r {
            Some(f) => {
                f(out);
                true
            }
            None => {
                out.fill(0.0);
                false
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{Device, NULL_RATE};

#[cfg(not(target_arch = "wasm32"))]
mod native {
use super::RenderFn;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Mix rate of the silent device ([`Device::open_null`]).
pub const NULL_RATE: u32 = 44_100;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

pub struct Device {
    pub rate: u32,
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Device {
    /// Opens the default output device. None if there is no usable device.
    pub fn open(render: RenderFn) -> Option<Device> {
        let (ready_tx, ready_rx) = mpsc::channel::<Option<u32>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("qb-audio".into())
            .spawn(move || {
                let stream = match build(render) {
                    Some((s, rate)) => {
                        let _ = ready_tx.send(Some(rate));
                        s
                    }
                    None => {
                        let _ = ready_tx.send(None);
                        return;
                    }
                };
                let _ = stop_rx.recv();
                drop(stream);
            })
            .ok()?;
        match ready_rx.recv() {
            Ok(Some(rate)) => Some(Device { rate, stop: stop_tx, thread: Some(thread) }),
            _ => {
                let _ = thread.join();
                None
            }
        }
    }

    /// The silent device used when no output device can be opened: a thread
    /// pulls the mix at [`NULL_RATE`] frames per second of wall-clock time and
    /// discards it, so queued SOUND/PLAY notes, `PLAY(n)`, `ON PLAY(n)` and
    /// `_SNDPLAYING` advance exactly as with a real device. A lag of more than
    /// half a second (a suspended machine) is dropped, as a device would.
    pub fn open_null(render: RenderFn) -> Option<Device> {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let rate = NULL_RATE;
        let thread = std::thread::Builder::new()
            .name("qb-audio-null".into())
            .spawn(move || {
                let start = Instant::now();
                let mut done: u64 = 0;
                let mut buf: Vec<f32> = Vec::new();
                loop {
                    match stop_rx.recv_timeout(Duration::from_millis(10)) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        _ => return,
                    }
                    let due = (start.elapsed().as_secs_f64() * f64::from(rate)) as u64;
                    done = done.max(due.saturating_sub(u64::from(rate / 2)));
                    while done < due {
                        let n = (due - done).min(1024) as usize;
                        buf.resize(n * 2, 0.0);
                        render(&mut buf);
                        done += n as u64;
                    }
                }
            })
            .ok()?;
        Some(Device { rate, stop: stop_tx, thread: Some(thread) })
    }

    /// Stops the stream and waits for the audio thread (must not hold the engine lock).
    pub fn close(mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn build(render: RenderFn) -> Option<(cpal::Stream, u32)> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let supported = device.default_output_config().ok()?;
    let config: cpal::StreamConfig = supported.config();
    let rate = config.sample_rate;
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => stream::<f32>(&device, &config, render),
        cpal::SampleFormat::I16 => stream::<i16>(&device, &config, render),
        cpal::SampleFormat::U16 => stream::<u16>(&device, &config, render),
        cpal::SampleFormat::I32 => stream::<i32>(&device, &config, render),
        cpal::SampleFormat::U8 => stream::<u8>(&device, &config, render),
        cpal::SampleFormat::F64 => stream::<f64>(&device, &config, render),
        _ => None,
    }?;
    stream.play().ok()?;
    Some((stream, rate))
}

fn stream<T>(device: &cpal::Device, config: &cpal::StreamConfig, render: RenderFn) -> Option<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    if channels == 0 {
        // chunks_exact_mut(0) would panic in the callback.
        return None;
    }
    let mut stereo: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            *config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels.max(1);
                stereo.resize(frames * 2, 0.0);
                render(&mut stereo);
                for (f, out) in data.chunks_exact_mut(channels).enumerate() {
                    let (l, r) = (stereo[2 * f], stereo[2 * f + 1]);
                    if channels == 1 {
                        out[0] = T::from_sample((l + r) * 0.5);
                    } else {
                        out[0] = T::from_sample(l);
                        out[1] = T::from_sample(r);
                        for x in &mut out[2..] {
                            *x = T::from_sample(0.0f32);
                        }
                    }
                }
            },
            |_err| {},
            None,
        )
        .ok()
}
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// The silent device pulls frames at its rate of wall-clock time.
    #[test]
    fn null_device_pulls_frames_in_real_time() {
        let frames = Arc::new(AtomicU64::new(0));
        let f = frames.clone();
        let t = std::time::Instant::now();
        let dev = Device::open_null(Arc::new(move |buf: &mut [f32]| {
            f.fetch_add(buf.len() as u64 / 2, Ordering::Relaxed);
        }))
        .expect("null device");
        assert_eq!(dev.rate, NULL_RATE);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let n = frames.load(Ordering::Relaxed);
        let elapsed = t.elapsed().as_secs_f64();
        dev.close();
        let expect = elapsed * f64::from(NULL_RATE);
        // pulled up to the last 10 ms tick, never ahead of the clock
        assert!((n as f64) <= expect, "{n} frames in {elapsed} s");
        assert!((n as f64) >= 0.2 * f64::from(NULL_RATE), "{n} frames in {elapsed} s");
    }
}
