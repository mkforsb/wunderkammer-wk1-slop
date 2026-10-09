//! CREAK: stick-slip friction dragged across a resonant body.
//!
//! A hand (or a door, or a wet finger) drags across the body with a velocity
//! and a normal pressure that follow a gesture over TIME. The contact obeys
//! a velocity-weakening friction law (S. Bilbao's soft bow model,
//! `Φ(u) = √(2a)·u·exp(−a·u² + ½)`), solved implicitly every sample with
//! Newton's method against the instantaneous admittance of the body's modes.
//! Velocity weakening is what makes things self-oscillate: grip, load,
//! slip, grip again. Depending on speed and pressure that gives a pure
//! squeal locked to one mode, a raucous scratch, or slow irregular slip
//! pulses (a creaking hinge, which also gets a soft spring of its own
//! so the pulses can run slower than any of the body's modes).
//!
//! Each hit drags the other way, like a door opened and closed.
//!
//! X = PRESSURE, Y = GESTURE (swell → swipe → tremble → judder),
//! Z = BODY (door → balloon → glass → cello).

use super::Ctx;
use crate::util::{Mode, Noise, lerp, morph, sin_turns};

const MODES: usize = 9;
/// Friction curve sharpness: the peak sits at `1/√(2a)` m/s.
const A: f32 = 30.0;
/// Kinetic friction floor (so a fast drag still grabs a little).
const FLOOR: f32 = 0.25;

/// Per body: mode ratios (the hinge's is relative to the pitch too).
const RATIOS: [[f32; MODES]; 4] = [
    [1.0, 1.41, 2.13, 2.67, 3.52, 4.31, 5.6, 7.1, 0.3],
    [1.0, 1.62, 2.0, 2.66, 3.1, 3.7, 4.4, 5.2, 0.2],
    [1.0, 2.32, 4.25, 6.63, 9.38, 12.5, 15.9, 19.8, 0.2],
    [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 0.2],
];
const T60: [[f32; MODES]; 4] = [
    [0.5, 0.4, 0.32, 0.28, 0.24, 0.2, 0.16, 0.14, 0.6],
    [0.3, 0.26, 0.22, 0.2, 0.17, 0.15, 0.13, 0.11, 0.3],
    [3.0, 2.0, 1.5, 1.0, 0.8, 0.6, 0.5, 0.4, 0.3],
    [1.5, 1.3, 1.1, 0.9, 0.8, 0.7, 0.6, 0.5, 0.3],
];
/// Mode shape at the contact point.
const SHAPE: [[f32; MODES]; 4] = [
    [1.0, 0.8, 0.9, 0.6, 0.7, 0.5, 0.5, 0.4, 1.6],
    [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.3],
    [1.0, 0.7, 0.5, 0.4, 0.3, 0.2, 0.15, 0.1, 0.0],
    [0.4, 0.73, 0.92, 1.0, 0.94, 0.76, 0.48, 0.14, 0.0],
];
/// How much each mode radiates (the hinge's spring is felt, not heard).
const RADIATION: [f32; MODES] = [1.0, 0.9, 0.85, 0.8, 0.7, 0.6, 0.55, 0.5, 0.1];
/// Pressure scale per body (lossy bodies need a firmer hand).
const PRESSURE: [f32; 4] = [25.0, 20.0, 4.0, 15.0];

#[derive(Clone, Debug)]
pub struct Creak {
    fs: f32,
    modes: [Mode; MODES],
    shape: [f32; MODES],
    admittance: f32,
    pressure: f32,
    gesture: f32,
    /// Seconds into the gesture and its length.
    t: f32,
    length: f32,
    active: bool,
    direction: f32,
    v_max: f32,
    /// Normal force actually applied (released smoothly at the end).
    n_now: f32,
    /// Last relative velocity (Newton's warm start).
    u: f32,
    /// Hand unsteadiness.
    wobble: f32,
    /// Highpassed noise for the rubbing sound.
    rub_lp: f32,
}

impl Creak {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            modes: [Mode::default(); MODES],
            shape: SHAPE[0],
            admittance: 0.0,
            pressure: 1.0,
            gesture: 0.0,
            t: 0.0,
            length: 1.0,
            active: false,
            direction: 1.0,
            v_max: 0.2,
            n_now: 0.0,
            u: 0.0,
            wobble: 0.0,
            rub_lp: 0.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        let ratios = morph(&RATIOS, c.z);
        let t60 = morph(&T60, c.z);
        self.shape = morph(&SHAPE, c.z);
        for (k, m) in self.modes.iter_mut().enumerate() {
            m.set(c.hz * ratios[k], t60[k], c.fs);
        }
        let scale = morph(
            &[[PRESSURE[0]], [PRESSURE[1]], [PRESSURE[2]], [PRESSURE[3]]],
            c.z,
        )[0];
        self.pressure = scale * (0.9 + 3.5 * c.x * c.x);
        self.admittance = self.shape.iter().map(|s| s * s).sum::<f32>() / c.fs;
        self.gesture = c.y;
    }

    pub fn trigger(&mut self, c: &Ctx, _rng: &mut Noise) {
        self.t = 0.0;
        self.length = c.time_s.max(0.08);
        self.active = true;
        self.direction = -self.direction;
        self.v_max = 0.05 + 0.25 * c.velocity;
    }

    /// Hand speed (0..1) and pressure (0..1) for a gesture shape at time `t`.
    fn shape_at(kind: usize, s: f32, t: f32) -> (f32, f32) {
        let arch = sin_turns(0.5 * s).max(0.0);
        match kind {
            // Swell: slowly in and out (a door swung open).
            0 => (arch.powf(1.2), 0.75 + 0.25 * arch),
            // Swipe: fast attack, slowing down (a finger across rubber).
            1 => ((s / 0.06).min(1.0) * (1.0 - s).powf(1.5), 1.0),
            // Tremble: a shaky hand.
            2 => (
                arch * (0.55 + 0.45 * sin_turns(7.3 * t)),
                0.8 + 0.2 * sin_turns(5.1 * t + 0.15),
            ),
            // Judder: stop-start jerks.
            _ => {
                let jerk = 0.5 + 0.5 * crate::util::fast_tanh(6.0 * sin_turns(8.5 * t));
                (arch.sqrt() * jerk, 1.0)
            }
        }
    }

    /// Friction coefficient and its slope at relative velocity `u`.
    #[inline]
    fn friction(u: f32) -> (f32, f32) {
        let s = (2.0 * A).sqrt();
        let e = (0.5 - A * u * u).exp();
        let t = crate::util::fast_tanh(40.0 * u);
        (
            s * u * e + FLOOR * t,
            s * e * (1.0 - 2.0 * A * u * u) + FLOOR * 40.0 * (1.0 - t * t),
        )
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        // Free evolution of the body.
        let mut v_free = 0.0;
        for (m, &phi) in self.modes.iter_mut().zip(&self.shape) {
            m.tick(0.0);
            v_free += phi * m.re;
        }

        let (v_bow, n_target) = if self.active {
            self.t += dt;
            let s = self.t / self.length;
            if s >= 1.0 {
                self.active = false;
                (0.0, 0.0)
            } else {
                let (i, f) = crate::params::map::segment(self.gesture, 4);
                let (va, na) = Self::shape_at(i, s, self.t);
                let (vb, nb) = Self::shape_at(i + 1, s, self.t);
                self.wobble += 0.002 * (rng.sample() - self.wobble);
                let hand = 1.0 + 4.0 * self.wobble;
                (
                    self.direction * self.v_max * lerp(va, vb, f),
                    self.pressure * lerp(na, nb, f) * hand,
                )
            }
        } else {
            (0.0, 0.0)
        };
        // Press on quickly, lift off gently.
        let k = if n_target > self.n_now { 0.02 } else { 0.004 };
        self.n_now += (n_target - self.n_now) * k;
        if self.n_now < 1e-4 && !self.active {
            self.n_now = 0.0;
            self.u = 0.0;
        }

        let mut force = 0.0;
        if self.n_now > 0.0 {
            // Solve u + Y·N·Φ(u) = v_bow − v_free for the relative velocity.
            let yn = self.admittance * self.n_now;
            let w = v_bow - v_free;
            let mut u = self.u;
            for _ in 0..4 {
                let (f, df) = Self::friction(u);
                let g = u + yn * f - w;
                let dg = (1.0 + yn * df).max(0.05);
                u -= g / dg;
            }
            if !u.is_finite() {
                u = 0.0;
            }
            self.u = u;
            force = self.n_now * Self::friction(u).0;
        }
        // Sliding also rubs: noise in proportion to the power dissipated.
        let n = rng.sample();
        self.rub_lp += 0.3 * (n - self.rub_lp);
        let rub = (n - self.rub_lp) * (force * self.u).abs().sqrt() * 0.12;

        let mut out = 0.0;
        for ((m, &phi), &rad) in self.modes.iter_mut().zip(&self.shape).zip(&RADIATION) {
            m.re += phi * force * dt;
            out += rad * m.re;
        }
        out * 2.5 + rub
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friction_makes_sound_and_stays_bounded() {
        let fs = 96_000.0;
        for body in [0.0, 0.33, 0.66, 1.0] {
            for x in [0.0, 0.5, 1.0] {
                let mut e = Creak::new(fs);
                let c = Ctx {
                    hz: 300.0,
                    x,
                    y: 0.4,
                    z: body,
                    time_s: 0.5,
                    ..Ctx::new(fs)
                };
                e.control(&c);
                let mut rng = Noise::new(3);
                e.trigger(&c, &mut rng);
                let out: Vec<f32> = (0..fs as usize).map(|_| e.tick(&c, &mut rng)).collect();
                let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
                assert!(out.iter().all(|v| v.is_finite()), "body {body} x {x}");
                assert!(peak > 1e-3 && peak < 8.0, "body {body} x {x}: {peak}");
            }
        }
    }
}
