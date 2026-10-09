//! COIN: something round spinning down on a table (Euler's disk).
//!
//! As a spinning disk settles, its rolling contact point circles faster and
//! faster: the precession rate diverges as a power of the time left,
//! `Ω ∝ (t₀ − t)^-p`, and the disk stops abruptly at a finite time with a
//! final slap. Here the contact point raps the object's own modes (and the
//! table's) once per wobble, so you hear the classic "wub … wub . wub-wub-
//! wbwbwbrrrrr-CLACK" of a coin or a dropped pot lid. TIME is how long the
//! spin lasts; a harder hit spins it longer.
//!
//! X = WOBBLE (the exponent: up makes the singularity sharper, holding the
//! slow wobble longer and cramming the acceleration into the last moments),
//! Y = RIM (a smooth rim whirrs; a rough one rattles with extra bumps and
//! rolling grit), Z = OBJECT (coin → pot lid → plate → bowl).

use super::Ctx;
use crate::params::map;
use crate::util::{Mode, Noise, morph, sin_turns};

const MODES: usize = 6;
/// Per object: mode ratios.
const RATIOS: [[f32; MODES]; 4] = [
    // Coin: free thin disk.
    [1.0, 1.73, 2.33, 3.91, 4.11, 6.30],
    // Pot lid: a domed shell with a handle.
    [1.0, 1.58, 2.31, 2.83, 3.45, 4.62],
    // Ceramic plate.
    [1.0, 2.09, 3.37, 4.82, 6.03, 7.21],
    // Bowl: the wide-spaced modes of a singing bowl.
    [1.0, 2.71, 5.12, 8.21, 11.9, 16.3],
];
const GAINS: [[f32; MODES]; 4] = [
    [1.0, 0.7, 0.6, 0.45, 0.4, 0.3],
    [1.0, 0.9, 0.7, 0.6, 0.5, 0.4],
    [1.0, 0.6, 0.5, 0.35, 0.3, 0.2],
    [1.0, 0.6, 0.4, 0.25, 0.15, 0.1],
];
/// Ring time of the lowest mode, seconds, per object.
const T60: [f32; 4] = [0.35, 1.6, 0.7, 4.0];
/// A wooden table under it all.
const TABLE_HZ: [f32; 4] = [92.0, 157.0, 231.0, 318.0];

#[derive(Clone, Debug)]
pub struct Coin {
    fs: f32,
    modes: [Mode; MODES],
    gains: [f32; MODES],
    table: [Mode; 4],
    /// Seconds into the spin and its total length.
    t: f32,
    length: f32,
    spinning: bool,
    /// Wobble: phase in turns, start and end rates, singularity offset.
    phase: f32,
    w_start: f32,
    w_end: f32,
    eps: f32,
    p: f32,
    rim: f32,
    level: f32,
    /// Rolling-noise lowpass state.
    grit: f32,
    /// Pending contact pulse: samples left and its length.
    pulse_left: u32,
    pulse_len: u32,
    pulse_amp: f32,
    bumps: u32,
    bump_index: u32,
}

impl Coin {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            modes: [Mode::default(); MODES],
            gains: GAINS[0],
            table: [Mode::default(); 4],
            t: 0.0,
            length: 1.0,
            spinning: false,
            phase: 0.0,
            w_start: 6.0,
            w_end: 200.0,
            eps: 0.01,
            p: 1.0,
            rim: 0.3,
            level: 0.0,
            grit: 0.0,
            pulse_left: 0,
            pulse_len: 1,
            pulse_amp: 0.0,
            bumps: 1,
            bump_index: 0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        let ratios = morph(&RATIOS, c.z);
        self.gains = morph(&GAINS, c.z);
        let t60 = morph(&[[T60[0]], [T60[1]], [T60[2]], [T60[3]]], c.z)[0];
        for (k, m) in self.modes.iter_mut().enumerate() {
            m.set(c.hz * ratios[k], t60 / (1.0 + 0.5 * k as f32), c.fs);
        }
        for (m, &hz) in self.table.iter_mut().zip(&TABLE_HZ) {
            m.set(hz, 0.12, c.fs);
        }
        self.rim = c.y;
        self.bumps = 1 + (c.y * 2.99) as u32;
        self.p = map::wobble_exponent(c.x);
        // The wobble ends somewhere below the object's ring, within reason.
        self.w_end = (c.hz * 0.3).clamp(60.0, 420.0);
    }

    pub fn trigger(&mut self, c: &Ctx, _rng: &mut Noise) {
        self.t = 0.0;
        self.spinning = true;
        self.length = c.time_s.max(0.12) * (0.55 + 0.45 * c.velocity);
        self.w_start = 3.0 + 6.0 * c.velocity;
        self.level = 0.4 + 0.6 * c.velocity;
        self.phase = 0.0;
        // Ω(u) = w_start·(u + ε)^-p reaches w_end at u = 0.
        self.eps = (self.w_start / self.w_end).powf(1.0 / self.p);
        // The flick that starts the spin.
        self.pulse(0.6 * self.level, 0.002);
    }

    fn pulse(&mut self, amp: f32, seconds: f32) {
        self.pulse_len = ((seconds * self.fs) as u32).max(2);
        self.pulse_left = self.pulse_len;
        self.pulse_amp = amp;
    }

    /// Current wobble rate in Hz.
    pub fn wobble_hz(&self) -> f32 {
        let u = (1.0 - self.t / self.length).max(0.0);
        self.w_start * (u + self.eps).powf(-self.p)
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        let mut force = 0.0;
        if self.spinning {
            self.t += dt;
            if self.t >= self.length {
                // Flat on the table.
                self.spinning = false;
                self.pulse(1.4 * self.level, 0.0008);
            } else {
                let w = self.wobble_hz();
                let progress = (w / self.w_end).min(1.0);
                self.phase += w * dt;
                // A rough rim knocks several times per wobble.
                let per = 1.0 / self.bumps as f32;
                if self.phase >= per {
                    self.phase -= per;
                    self.bump_index = (self.bump_index + 1) % self.bumps;
                    let accent = if self.bump_index == 0 {
                        1.0
                    } else {
                        0.35 + 0.4 * self.rim
                    };
                    let jitter = 1.0 + 0.3 * self.rim * rng.sample();
                    let width = (0.35 / (w * self.bumps as f32)).clamp(0.0004, 0.004);
                    self.pulse(
                        self.level * accent * jitter * (0.25 + 0.75 * progress.sqrt()),
                        width,
                    );
                }
                // Rolling contact: grit that swells and fades with each wobble.
                self.grit += 0.25 * (rng.sample() - self.grit);
                let around = 0.5 + 0.5 * sin_turns(self.phase * self.bumps as f32);
                force += self.grit * self.rim * self.level * around * 0.08 * (0.3 + progress);
            }
        }
        if self.pulse_left > 0 {
            // Raised-cosine contact force.
            let x = 1.0 - self.pulse_left as f32 / self.pulse_len as f32;
            // Unit area per unit amplitude.
            force +=
                self.pulse_amp * (1.0 - (x * core::f32::consts::TAU).cos()) / self.pulse_len as f32;
            self.pulse_left -= 1;
        }
        let mut out = 0.0;
        for (m, &g) in self.modes.iter_mut().zip(&self.gains) {
            out += g * m.tick(force * g);
        }
        let mut table = 0.0;
        for m in &mut self.table {
            table += m.tick(force);
        }
        0.45 * out + 0.6 * table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wobble_accelerates_to_the_end_rate_then_stops() {
        let fs = 48_000.0;
        let mut coin = Coin::new(fs);
        let c = Ctx {
            time_s: 1.0,
            x: 0.5,
            ..Ctx::new(fs)
        };
        coin.control(&c);
        coin.trigger(&c, &mut Noise::new(1));
        let start = coin.wobble_hz();
        let mut rng = Noise::new(2);
        let mut last = start;
        let mut rates = Vec::new();
        while coin.spinning {
            coin.tick(&c, &mut rng);
            let w = coin.wobble_hz();
            assert!(w >= last - 1e-3);
            last = w;
            rates.push(w);
        }
        assert!(start < 10.0);
        assert!((last - coin.w_end).abs() / coin.w_end < 0.1, "{last}");
        // Then it rings and dies away.
        let tail: Vec<f32> = (0..fs as usize * 6)
            .map(|_| coin.tick(&c, &mut rng))
            .collect();
        assert!(tail[..4800].iter().any(|x| x.abs() > 1e-3));
        assert!(tail[tail.len() - 4800..].iter().all(|x| x.abs() < 1e-3));
    }
}
