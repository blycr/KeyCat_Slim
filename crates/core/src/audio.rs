//! Procedural cat purr synthesizer — zero bundled audio assets.
//!
//! Generates soft brown noise through a 420 Hz low-pass filter, modulated by a 25 Hz purr pulse
//! and the cat's breathing envelope. Runs identically on Windows (`waveOut`) and macOS (`AVAudioEngine`).

use std::f32::consts::TAU;

pub struct PurrSynth {
    sample_rate: f32,
    rng: u64,
    brown: f32,
    lp_state: f32,
    lp_alpha: f32,
    lfo_phase: f32,
    gain: f32,
    target_gain: f32,
    pub volume: f32,
}

impl PurrSynth {
    pub fn new(sample_rate: u32) -> Self {
        let sr = (sample_rate.max(8000)) as f32;
        let rc = 1.0 / (TAU * 420.0);
        let dt = 1.0 / sr;
        let lp_alpha = dt / (rc + dt);
        Self {
            sample_rate: sr,
            rng: 0x517c_c1b7_2722_0a95,
            brown: 0.0,
            lp_state: 0.0,
            lp_alpha,
            lfo_phase: 0.0,
            gain: 0.0,
            target_gain: 0.0,
            volume: 0.65,
        }
    }

    /// Updates target purr loudness from `enabled`, `level` (0..1), and `breath` phase (cycles).
    pub fn set_target(&mut self, enabled: bool, level: f32, breath: f32) {
        if !enabled || level <= 0.01 {
            self.target_gain = 0.0;
            return;
        }
        let exhale = 0.6 + 0.4 * (breath * TAU).sin().max(0.0);
        self.target_gain = (level.clamp(0.0, 1.0) * exhale * self.volume * 0.48).clamp(0.0, 0.7);
    }

    pub fn is_active(&self) -> bool {
        self.target_gain > 0.0005 || self.gain > 0.0005
    }

    #[inline]
    fn next_white(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        let u = ((x >> 40) as f32) / ((1u32 << 24) as f32);
        u * 2.0 - 1.0
    }

    #[inline]
    fn next_sample(&mut self) -> f32 {
        // Smooth gain ramp (tau ~ 70 ms) to avoid clicks when starting/stopping.
        let smooth = 1.0 / (0.07 * self.sample_rate);
        self.gain += (self.target_gain - self.gain) * smooth.min(1.0);
        if self.gain < 1e-5 && self.target_gain == 0.0 {
            self.gain = 0.0;
            return 0.0;
        }

        let w = self.next_white();
        self.brown = (self.brown + 0.02 * w) / 1.02;
        let raw = (self.brown * 3.5).clamp(-1.0, 1.0);
        self.lp_state += self.lp_alpha * (raw - self.lp_state);

        self.lfo_phase += 25.0 / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        let pulse = 0.55 + 0.45 * (self.lfo_phase * TAU).sin();
        (self.lp_state * pulse * self.gain).clamp(-0.95, 0.95)
    }

    pub fn fill_f32(&mut self, out: &mut [f32]) {
        if !self.is_active() {
            out.fill(0.0);
            return;
        }
        for s in out.iter_mut() {
            *s = self.next_sample();
        }
    }

    pub fn fill_i16(&mut self, out: &mut [i16]) {
        if !self.is_active() {
            out.fill(0);
            return;
        }
        for s in out.iter_mut() {
            let v = self.next_sample();
            *s = (v * 32767.0).clamp(-32767.0, 32767.0) as i16;
        }
    }
}
