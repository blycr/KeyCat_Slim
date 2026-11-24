//! Lightweight procedural purr audio output for Windows using Win32 `waveOut`.
//!
//! Spawns a single quiet background thread that only opens the audio device while purring is active,
//! keeping CPU and memory usage at zero whenever sound is disabled or the cat is not purring.

#[cfg(windows)]
mod imp {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    };
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use keycat_core::audio::PurrSynth;
    use windows::Win32::Media::Audio::{
        waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutReset, waveOutUnprepareHeader,
        waveOutWrite, CALLBACK_NULL, HWAVEOUT, WAVEFORMATEX, WAVEHDR, WAVE_FORMAT_PCM,
        WAVE_MAPPER, WHDR_DONE,
    };

    const SAMPLE_RATE: u32 = 24_000;
    const CHUNK_SAMPLES: usize = 1024; // ~42 ms latency, ultra-low CPU
    const NUM_BUFS: usize = 3;

    struct Shared {
        enabled: bool,
        level: f32,
        breath: f32,
    }

    pub struct PurrAudio {
        state: Arc<(Mutex<Shared>, Condvar)>,
        quit: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl PurrAudio {
        pub fn new() -> Self {
            let state = Arc::new((
                Mutex::new(Shared { enabled: false, level: 0.0, breath: 0.0 }),
                Condvar::new(),
            ));
            let quit = Arc::new(AtomicBool::new(false));
            let st = Arc::clone(&state);
            let q = Arc::clone(&quit);
            let handle = thread::Builder::new()
                .name("keycat-purr".into())
                .spawn(move || run_audio_loop(st, q))
                .ok();
            Self { state, quit, handle }
        }

        pub fn sync(&self, enabled: bool, level: f32, breath: f32) {
            if let Ok(mut g) = self.state.0.lock() {
                let wake = (!g.enabled || g.level <= 0.01) && enabled && level > 0.01;
                g.enabled = enabled;
                g.level = level;
                g.breath = breath;
                if wake {
                    self.state.1.notify_one();
                }
            }
        }
    }

    impl Drop for PurrAudio {
        fn drop(&mut self) {
            self.quit.store(true, Ordering::Release);
            self.state.1.notify_one();
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }

    fn run_audio_loop(state: Arc<(Mutex<Shared>, Condvar)>, quit: Arc<AtomicBool>) {
        let mut synth = PurrSynth::new(SAMPLE_RATE);
        while !quit.load(Ordering::Acquire) {
            // Sleep quietly until purring is requested.
            {
                let Ok(mut g) = state.0.lock() else { return };
                while !quit.load(Ordering::Acquire) && (!g.enabled || g.level <= 0.01) {
                    let Ok(next) = state.1.wait(g) else { return };
                    g = next;
                }
                if quit.load(Ordering::Acquire) {
                    return;
                }
                synth.set_target(g.enabled, g.level, g.breath);
            }

            // Open waveOut device only while actively purring.
            let fmt = WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_PCM as u16,
                nChannels: 1,
                nSamplesPerSec: SAMPLE_RATE,
                nAvgBytesPerSec: SAMPLE_RATE * 2,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            let mut hwo = HWAVEOUT::default();
            let open_res = unsafe {
                waveOutOpen(
                    Some(&mut hwo),
                    WAVE_MAPPER,
                    &fmt,
                    None,
                    None,
                    CALLBACK_NULL,
                )
            };
            if open_res != 0 {
                thread::sleep(Duration::from_millis(200));
                continue;
            }

            let mut bufs: Vec<Vec<i16>> = (0..NUM_BUFS).map(|_| vec![0i16; CHUNK_SAMPLES]).collect();
            let mut hdrs: Vec<WAVEHDR> = bufs
                .iter_mut()
                .map(|b| WAVEHDR {
                    lpData: windows::core::PSTR(b.as_mut_ptr() as *mut u8),
                    dwBufferLength: (CHUNK_SAMPLES * 2) as u32,
                    ..Default::default()
                })
                .collect();

            let hdr_sz = std::mem::size_of::<WAVEHDR>() as u32;
            unsafe {
                for i in 0..NUM_BUFS {
                    let _ = waveOutPrepareHeader(hwo, &mut hdrs[i], hdr_sz);
                    synth.fill_i16(&mut bufs[i]);
                    let _ = waveOutWrite(hwo, &mut hdrs[i], hdr_sz);
                }
            }

            let mut silent_chunks = 0u32;
            while !quit.load(Ordering::Acquire) {
                if let Ok(g) = state.0.lock() {
                    synth.set_target(g.enabled, g.level, g.breath);
                }
                let mut refilled = false;
                for i in 0..NUM_BUFS {
                    if (hdrs[i].dwFlags & WHDR_DONE) != 0 {
                        synth.fill_i16(&mut bufs[i]);
                        hdrs[i].dwFlags &= !WHDR_DONE;
                        unsafe {
                            let _ = waveOutWrite(hwo, &mut hdrs[i], hdr_sz);
                        }
                        refilled = true;
                        if synth.is_active() {
                            silent_chunks = 0;
                        } else {
                            silent_chunks += 1;
                        }
                    }
                }
                if silent_chunks >= (NUM_BUFS as u32) + 2 {
                    break;
                }
                if !refilled {
                    thread::sleep(Duration::from_millis(12));
                }
            }

            unsafe {
                let _ = waveOutReset(hwo);
                for h in &mut hdrs {
                    let _ = waveOutUnprepareHeader(hwo, h, hdr_sz);
                }
                let _ = waveOutClose(hwo);
            }
        }
    }
}

#[cfg(windows)]
pub use imp::PurrAudio;
