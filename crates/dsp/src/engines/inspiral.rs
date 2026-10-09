//! INSPIRAL: two black holes spiral together.
//!
//! The hit sets two black holes orbiting, TIME before they merge. As the
//! orbit radiates away energy it shrinks and quickens, and the gravitational
//! wave (at twice the orbital frequency) sweeps upward as `f ∝ τ^-3/8` (τ the
//! time left), its amplitude growing as `f^2/3`: the "chirp" heard by LIGO,
//! here ending at PITCH. At the merger the signal peaks and the new black hole
//! rings down as a damped sinusoid (its quasi-normal mode), with a small step
//! of gravitational-wave memory left behind.
//!
//! Unlike a drum, the event *builds* towards the hit's climax, which arrives
//! TIME after you strike.
//!
//! X = MASS (ratio 1 … 12: unequal masses add the odd harmonics),
//! Y = ECCENTRIC (an elongated orbit sings in bursts at each close pass,
//! accelerating "whoop … whoop . whoop-whoop-WHOOM", circularising as it
//! shrinks, `e ∝ f^-19/18`), Z = SPIN (precession wobble, and a longer,
//! higher ringdown).

use super::Ctx;
use crate::params::map;
use crate::util::{Noise, sin_turns};

/// Merger time constant, in cycles of the final frequency.
const MERGE_CYCLES: f32 = 2.5;

#[derive(Clone, Debug)]
pub struct Inspiral {
    fs: f32,
    /// Seconds left to coalescence (negative after the merger).
    tau: f32,
    active: bool,
    merged: bool,
    hz: f32,
    q: f32,
    e0: f32,
    chi: f32,
    /// Orbital phase (radians) and the precession phase (turns).
    phi: f64,
    precess: f32,
    f_start: f32,
    tau_merge: f32,
    level: f32,
    ring_amp: f32,
    ring_mul: f32,
    ring_hz: f32,
    /// Waveform amplitude when the merger began (for a smooth handover).
    last: f32,
    memory: f32,
    memory_target: f32,
}

impl Inspiral {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            tau: 0.0,
            active: false,
            merged: true,
            hz: 300.0,
            q: 1.0,
            e0: 0.0,
            chi: 0.0,
            phi: 0.0,
            precess: 0.0,
            f_start: 20.0,
            tau_merge: 0.01,
            level: 1.0,
            ring_amp: 0.0,
            ring_mul: 0.0,
            ring_hz: 300.0,
            last: 0.0,
            memory: 0.0,
            memory_target: 0.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.hz = c.hz;
        self.q = map::mass_ratio(c.x);
        self.e0 = map::eccentricity(c.y);
        self.chi = map::spin(c.z);
    }

    /// Gravitational-wave frequency with `tau` seconds to go.
    pub fn gw_hz(&self, tau: f32) -> f32 {
        self.hz * (tau.max(self.tau_merge) / self.tau_merge).powf(-0.375)
    }

    pub fn trigger(&mut self, c: &Ctx, _rng: &mut Noise) {
        self.tau_merge = MERGE_CYCLES / self.hz;
        self.tau = c.time_s.max(4.0 * self.tau_merge);
        self.f_start = self.gw_hz(self.tau);
        self.active = true;
        self.merged = false;
        self.level = 0.3 + 0.7 * c.velocity;
        self.memory_target = 0.0;
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, _rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        let mut out = 0.0;
        if self.active && !self.merged {
            self.tau -= dt;
            let f = self.gw_hz(self.tau);
            let f_orb = 0.5 * f as f64;
            // Eccentricity bleeds away as the orbit shrinks.
            let e = (self.e0 * (f / self.f_start).powf(-19.0 / 18.0)).min(self.e0);
            let ce = (self.phi.cos()) as f32;
            let kepler = (1.0 + e * ce).powi(2) / (1.0 - e * e).powf(1.5);
            self.phi += core::f64::consts::TAU * f_orb * kepler as f64 * dt as f64;
            if self.phi >= core::f64::consts::TAU {
                self.phi -= core::f64::consts::TAU;
            }
            let turns = (self.phi / core::f64::consts::TAU).fract() as f32;
            let delta = (self.q - 1.0) / (self.q + 1.0);
            let quad = sin_turns(2.0 * turns + 0.25);
            let h = (1.0 + e * ce) * quad
                + 0.45 * delta * sin_turns(3.0 * turns + 0.25)
                + 0.2 * delta * sin_turns(turns + 0.25);
            // Spin-orbit precession: a wobble that speeds up with the orbit.
            self.precess += 0.07 * self.chi * f_orb as f32 * dt;
            let wobble = 1.0 - 0.45 * self.chi * (0.5 + 0.5 * sin_turns(self.precess));
            let amp = (f / self.hz).powf(2.0 / 3.0);
            out = self.level * amp * wobble * h / (1.0 + e);
            self.last = self.level * amp * wobble;
            if self.tau <= self.tau_merge {
                // Merger: hand over to the ringdown, phase-continuously.
                self.merged = true;
                self.ring_amp = self.last;
                self.ring_hz = self.hz * (1.0 + 0.3 * self.chi);
                // Real ringdowns last a few cycles; this one is allowed to
                // sing (a few tens of cycles, hundreds for fast spins).
                let quality = 12.0 + 400.0 * self.chi * self.chi;
                let t_ring = (quality / (core::f32::consts::PI * self.ring_hz)).max(0.03);
                self.ring_mul = (-dt / t_ring).exp();
                self.memory_target = 0.2 * self.level;
            }
        } else if self.active {
            self.phi += core::f64::consts::TAU * 0.5 * self.ring_hz as f64 * dt as f64;
            if self.phi >= core::f64::consts::TAU {
                self.phi -= core::f64::consts::TAU;
            }
            let turns = (self.phi / core::f64::consts::TAU).fract() as f32;
            out = self.ring_amp * sin_turns(2.0 * turns + 0.25);
            self.ring_amp *= self.ring_mul;
            if self.ring_amp < 1e-6 && self.memory.abs() < 1e-6 {
                self.active = false;
            }
        }
        // Memory: a lasting offset, smoothed in (and blocked downstream, so
        // it is heard as a soft thump).
        self.memory += (self.memory_target - self.memory) * (30.0 * dt).min(1.0);
        if self.merged {
            self.memory_target *= 1.0 - 8.0 * dt;
        }
        (out + self.memory) * 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chirps_up_then_rings_down() {
        let fs = 48_000.0;
        let mut b = Inspiral::new(fs);
        let c = Ctx {
            hz: 400.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            time_s: 1.0,
            ..Ctx::new(fs)
        };
        b.control(&c);
        let mut rng = Noise::new(1);
        b.trigger(&c, &mut rng);
        let out: Vec<f32> = (0..fs as usize * 2).map(|_| b.tick(&c, &mut rng)).collect();
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 2.0));
        let zc = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        let early = &out[4_800..9_600];
        let late = &out[40_000..44_800];
        assert!(zc(late) > 2 * zc(early), "{} {}", zc(early), zc(late));
        assert!(rms(late) > 1.4 * rms(early));
        assert!(rms(&out[out.len() - 4_800..]) < 1e-3);
    }

    #[test]
    fn eccentric_orbits_stay_bounded() {
        let fs = 48_000.0;
        let mut b = Inspiral::new(fs);
        let c = Ctx {
            hz: 2_000.0,
            x: 1.0,
            y: 1.0,
            z: 1.0,
            time_s: 3.0,
            ..Ctx::new(fs)
        };
        b.control(&c);
        let mut rng = Noise::new(1);
        b.trigger(&c, &mut rng);
        for _ in 0..fs as usize * 4 {
            let x = b.tick(&c, &mut rng);
            assert!(x.is_finite() && x.abs() < 3.0, "{x}");
        }
    }
}
