//! ICE: flexural waves in a frozen lake (or a guy-wire, or a slinky).
//!
//! Bending waves in a thin plate are dispersive (ω ∝ k²), so high
//! frequencies outrun low ones and a single knock arrives as a falling
//! chirp: the "pew" of a skater on thin ice or the Star Wars blaster (a
//! hammer on a radio tower's guy-wire). Rather than approximating the
//! dispersion with a pile of allpass filters, this object evaluates the
//! plate's exact impulse response at distance x,
//!
//! ```text
//!     h(t) ∝ t^-½ · cos(x² / 4at − π/4)
//! ```
//!
//! whose instantaneous frequency falls as 1/t². Each path (the direct wave
//! and echoes off the shore, each from further away and so slower and more
//! drawn out) is one such chirp, starting the moment its frequency drops
//! below Nyquist. PITCH is the frequency the direct chirp passes at its
//! reference time; TIME fades the whole lake out.
//!
//! X = DISTANCE (a 8 ms "tick-pew" … a 480 ms "peeeyoooo"), Y = ECHOES
//! (1–12 paths), Z = SHATTER (spontaneous cracks, each its own little chirp).

use super::Ctx;
use crate::params::map;
use crate::util::{Noise, sin_turns};

const PATHS: usize = 24;

#[derive(Clone, Copy, Debug, Default)]
struct Path {
    /// Reference time: the chirp passes `hz` this long after it starts.
    tau: f64,
    hz: f64,
    gain: f32,
    /// Seconds since this path's knock.
    t: f64,
    start: f64,
    active: bool,
}

#[derive(Clone, Debug)]
pub struct Ice {
    fs: f32,
    paths: [Path; PATHS],
    tau: f32,
    echoes: usize,
    shatter: f32,
    hz: f32,
    t: f32,
    length: f32,
    level: f32,
    thump: crate::util::Mode,
}

impl Ice {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            paths: [Path::default(); PATHS],
            tau: 0.05,
            echoes: 1,
            shatter: 0.0,
            hz: 400.0,
            t: 10.0,
            length: 1.0,
            level: 1.0,
            thump: crate::util::Mode::default(),
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.tau = map::ice_distance_seconds(c.x);
        self.echoes = map::ice_echoes(c.y);
        self.shatter = c.z;
        self.hz = c.hz;
        self.thump.set(c.hz * 0.25 + 30.0, 0.08, c.fs);
    }

    /// Time after the knock at which a path's chirp falls below Nyquist
    /// (its frequency is hz·(τ/t)²).
    fn audible_from(&self, tau: f32, hz: f32) -> f64 {
        // Start below 12 kHz (or 16× the pitch): a chirp from the very top
        // of the band is mostly a click.
        let f_max = (0.42 * self.fs).min(12_000.0).min(16.0 * hz) as f64;
        tau as f64 * (hz as f64 / f_max).sqrt()
    }

    /// Start a path whose knock was `clock` seconds ago.
    fn launch(&mut self, tau: f32, hz: f32, gain: f32, clock: f64) {
        let start = self.audible_from(tau, hz);
        let hz = hz as f64;
        let tau = tau as f64;
        let slot = self
            .paths
            .iter_mut()
            .min_by(|a, b| {
                let key = |p: &Path| if p.active { p.gain.abs() } else { -1.0 };
                key(a)
                    .partial_cmp(&key(b))
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .unwrap();
        *slot = Path {
            tau,
            hz,
            gain,
            t: clock,
            start,
            active: true,
        };
    }

    pub fn trigger(&mut self, c: &Ctx, rng: &mut Noise) {
        self.t = 0.0;
        self.length = c.time_s.max(0.1);
        self.level = 0.3 + 0.7 * c.velocity;
        let mut tau = self.tau;
        let mut gain = self.level;
        // Skip the silence before the direct wave's first audible cycle;
        // the echoes keep their delays relative to it.
        let clock = self.audible_from(tau, self.hz);
        for _ in 0..self.echoes {
            self.launch(tau, self.hz, gain, clock);
            // The shore is irregular: each echo travels a bit further.
            tau *= 1.25 + 0.5 * rng.uniform();
            gain *= 0.62 + 0.25 * rng.uniform();
        }
        self.thump.re += 0.6 * self.level;
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        self.t += dt;
        // Cracks: the ice keeps settling while it rings.
        let alive = (-self.t / self.length).exp();
        if self.shatter > 0.0 && rng.uniform() < self.shatter * self.shatter * 40.0 * alive * dt {
            let tau = (0.002 + 0.02 * rng.uniform()) * (self.tau / 0.05).sqrt();
            let hz = self.hz * (0.5 + 3.5 * rng.uniform());
            let gain = self.level * alive * (0.1 + 0.4 * rng.uniform());
            let clock = self.audible_from(tau, hz);
            self.launch(tau, hz, gain, clock);
        }
        let mut out = 0.0;
        let fade = alive * alive;
        for p in &mut self.paths {
            if !p.active {
                continue;
            }
            p.t += dt as f64;
            if p.t < p.start {
                continue;
            }
            let ratio = p.tau / p.t;
            let freq = p.hz * ratio * ratio;
            let amp = p.gain * (ratio.sqrt() as f32).min(4.0);
            if freq < 12.0 || amp * fade < 1e-5 {
                p.active = false;
                continue;
            }
            // Fade out as the chirp sinks below hearing, rather than stopping.
            let low = crate::util::smoothstep(((freq - 12.0) / 50.0) as f32);
            // Phase in turns: hz·τ²/t (the instantaneous frequency is its derivative).
            let phase = p.hz * p.tau * ratio;
            let turns = (phase - phase.floor()) as f32;
            // Ease in over the first few cycles.
            let onset = (((p.t - p.start) * freq * 0.5).min(1.0)) as f32;
            out += amp * onset * low * sin_turns(turns - 0.125);
        }
        let thump = self.thump.tick(0.0);
        out * fade * 0.5 + thump * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chirp_falls() {
        let fs = 48_000.0;
        let mut ice = Ice::new(fs);
        let c = Ctx {
            hz: 800.0,
            x: 0.6,
            y: 0.0,
            z: 0.0,
            time_s: 3.0,
            ..Ctx::new(fs)
        };
        ice.control(&c);
        let mut rng = Noise::new(1);
        ice.trigger(&c, &mut rng);
        let out: Vec<f32> = (0..fs as usize).map(|_| ice.tick(&c, &mut rng)).collect();
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 4.0));
        let zc = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let tau = (map::ice_distance_seconds(0.6) * fs) as usize;
        let early = zc(&out[tau / 2..tau / 2 + 960]);
        let late = zc(&out[2 * tau..2 * tau + 960]);
        assert!(early > late * 4, "{early} vs {late}");
    }
}
