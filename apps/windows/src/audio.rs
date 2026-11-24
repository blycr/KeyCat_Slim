//! Lightweight procedural purr audio output for Windows using dynamically loaded `winmm.dll`.
//!
//! When sound is disabled (default), zero audio threads are spawned and `winmm.dll` is never loaded.
//! The worker thread and audio device open lazily only when sound is enabled and the cat purrs.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::mem;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    };
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use keycat_core::audio::PurrSynth;
    use windows::core::{s, w, PCSTR};
    use windows::Win32::Foundation::{FreeLibrary, HMODULE};
    use windows::Win32::Media::Audio::{
        CALLBACK_NULL, HWAVEOUT, WAVEFORMATEX, WAVEHDR, WAVE_FORMAT_PCM, WAVE_MAPPER, WHDR_DONE,
    };
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    const SAMPLE_RATE: u32 = 24_000;
    const CHUNK_SAMPLES: usize = 1024; // ~42 ms latency, ultra-low CPU
    const NUM_BUFS: usize = 3;

    type FnWaveOutOpen = unsafe extern "system" fn(
        *mut HWAVEOUT,
        u32,
        *const WAVEFORMATEX,
        usize,
        usize,
        u32,
    ) -> u32;
    type FnWaveOutHdr = unsafe extern "system" fn(HWAVEOUT, *mut WAVEHDR, u32) -> u32;
    type FnWaveOutCtl = unsafe extern "system" fn(HWAVEOUT) -> u32;

    struct Winmm {
        module: HMODULE,
        open: FnWaveOutOpen,
        prepare: FnWaveOutHdr,
        write: FnWaveOutHdr,
        unprepare: FnWaveOutHdr,
        reset: FnWaveOutCtl,
        close: FnWaveOutCtl,
    }

    impl Winmm {
        fn load() -> Option<Self> {
            unsafe {
                let module = LoadLibraryW(w!("winmm.dll")).ok()?;
                let resolve = |name: PCSTR| -> Option<*const c_void> {
                    GetProcAddress(module, name).map(|f| f as *const c_void)
                };
                let (Some(p_open), Some(p_prep), Some(p_write), Some(p_unprep), Some(p_reset), Some(p_close)) = (
                    resolve(s!("waveOutOpen")),
                    resolve(s!("waveOutPrepareHeader")),
                    resolve(s!("waveOutWrite")),
                    resolve(s!("waveOutUnprepareHeader")),
                    resolve(s!("waveOutReset")),
                    resolve(s!("waveOutClose")),
                ) else {
                    let _ = FreeLibrary(module);
                    return None;
                };
                Some(Self {
                    module,
                    open: mem::transmute::<*const c_void, FnWaveOutOpen>(p_open),
                    prepare: mem::transmute::<*const c_void, FnWaveOutHdr>(p_prep),
                    write: mem::transmute::<*const c_void, FnWaveOutHdr>(p_write),
                    unprepare: mem::transmute::<*const c_void, FnWaveOutHdr>(p_unprep),
                    reset: mem::transmute::<*const c_void, FnWaveOutCtl>(p_reset),
                    close: mem::transmute::<*const c_void, FnWaveOutCtl>(p_close),
                })
            }
        }
    }

    impl Drop for Winmm {
        fn drop(&mut self) {
            unsafe {
                let _ = FreeLibrary(self.module);
            }
        }
    }

    struct Shared {
        enabled: bool,
        level: f32,
        breath: f32,
    }

    struct Worker {
        state: Arc<(Mutex<Shared>, Condvar)>,
        quit: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl Drop for Worker {
        fn drop(&mut self) {
            self.quit.store(true, Ordering::Release);
            self.state.1.notify_one();
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }

    pub struct PurrAudio {
        worker: Option<Worker>,
    }

    impl PurrAudio {
        /// Zero-cost constructor: spawns no thread and loads no audio DLL until actually needed.
        pub fn new() -> Self {
            Self { worker: None }
        }

        pub fn sync(&mut self, enabled: bool, level: f32, breath: f32) {
            let active = enabled && level > 0.01;
            if !active && self.worker.is_none() {
                return;
            }
            if active && self.worker.is_none() {
                let state = Arc::new((
                    Mutex::new(Shared { enabled, level, breath }),
                    Condvar::new(),
                ));
                let quit = Arc::new(AtomicBool::new(false));
                let st = Arc::clone(&state);
                let q = Arc::clone(&quit);
                if let Ok(handle) = thread::Builder::new()
                    .name("keycat-purr".into())
                    .stack_size(128 * 1024)
                    .spawn(move || run_audio_loop(st, q))
                {
                    self.worker = Some(Worker { state, quit, handle: Some(handle) });
                }
                return;
            }
            if let Some(ref worker) = self.worker {
                if let Ok(mut g) = worker.state.0.lock() {
                    let wake = (!g.enabled || g.level <= 0.01) && active;
                    g.enabled = enabled;
                    g.level = level;
                    g.breath = breath;
                    if wake {
                        worker.state.1.notify_one();
                    }
                }
            }
            // If user explicitly turned sound off, tear down the background thread & unload winmm.dll.
            if !enabled {
                self.worker = None;
            }
        }
    }

    fn run_audio_loop(state: Arc<(Mutex<Shared>, Condvar)>, quit: Arc<AtomicBool>) {
        let Some(winmm) = Winmm::load() else { return };
        let mut synth = PurrSynth::new(SAMPLE_RATE);

        while !quit.load(Ordering::Acquire) {
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
                (winmm.open)(&mut hwo, WAVE_MAPPER, &fmt, 0, 0, CALLBACK_NULL.0)
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

            let hdr_sz = mem::size_of::<WAVEHDR>() as u32;
            unsafe {
                for i in 0..NUM_BUFS {
                    let _ = (winmm.prepare)(hwo, &mut hdrs[i], hdr_sz);
                    synth.fill_i16(&mut bufs[i]);
                    let _ = (winmm.write)(hwo, &mut hdrs[i], hdr_sz);
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
                            let _ = (winmm.write)(hwo, &mut hdrs[i], hdr_sz);
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
                let _ = (winmm.reset)(hwo);
                for h in &mut hdrs {
                    let _ = (winmm.unprepare)(hwo, h, hdr_sz);
                }
                let _ = (winmm.close)(hwo);
            }
        }
    }
}

#[cfg(windows)]
pub use imp::PurrAudio;
