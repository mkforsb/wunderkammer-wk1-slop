//! POUR: water poured into a bottle.
//!
//! The air above the water is a tube closed at the bottom (the water) and
//! open at the top, so it resonates at odd harmonics of `c / 4L`. As the
//! bottle fills, L shrinks and the pitch rises along `f₀ / (1 − k·t)`: slowly
//! at first, then faster and faster towards the brim, which is what makes
//! the sound so recognisable. The column is a delay loop with an inverting
//! reflection, excited by the stream's turbulence and by bubbles. Each bubble is
//! a damped sine at its Minnaert frequency (≈ 3.26 m·Hz / radius) that chirps
//! upward as it rises (after K. van den Doel's bubble model). Big bubbles
//! become glugs; tiny ones become fizz.
//!
//! TIME is how long the pour lasts (the pitch at the end is set by FILL).
//!
//! X = FILL (up: fill, rising up to 3.2 octaves; down: drain, falling),
//! Y = FLOW (a smooth stream → bubbly → glugging), Z = FIZZ (carbonation).

use super::Ctx;
use crate::params::map;
use crate::util::{DelayLine, Noise, sin_turns, smoothstep};

const BUBBLES: usize = 16;
const FIZZ: usize = 8;
const MIN_HZ: f32 = 18.0;

#[derive(Clone, Copy, Debug, Default)]
struct Bubble {
    phase: f32,
    hz: f32,
    /// Fractional pitch rise per second.
    chirp: f32,
    amp: f32,
    mul: f32,
    t: f32,
}

#[derive(Clone, Debug)]
pub struct Pour {
    fs: f32,
    tube: DelayLine,
    tube_lp: f32,
    f0: f32,
    octaves: f32,
    flow: f32,
    fizz: f32,
    t: f32,
    length: f32,
    pouring: bool,
    level: f32,
    stream_env: f32,
    /// Band-limited stream noise.
    lp_hi: f32,
    lp_lo: f32,
    turbulence: f32,
    glug_phase: f32,
    splash: f32,
    bubbles: [Bubble; BUBBLES],
    pops: [Bubble; FIZZ],
    fizz_env: f32,
}

impl Pour {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            tube: DelayLine::new((fs / (2.0 * MIN_HZ)) as usize + 4),
            tube_lp: 0.0,
            f0: 200.0,
            octaves: 2.0,
            flow: 0.3,
            fizz: 0.0,
            t: 0.0,
            length: 1.0,
            pouring: false,
            level: 1.0,
            stream_env: 0.0,
            lp_hi: 0.0,
            lp_lo: 0.0,
            turbulence: 0.0,
            glug_phase: 0.0,
            splash: 0.0,
            bubbles: [Bubble::default(); BUBBLES],
            pops: [Bubble::default(); FIZZ],
            fizz_env: 0.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.f0 = c.hz.max(MIN_HZ);
        self.octaves = map::fill_octaves(c.x);
        self.flow = c.y;
        self.fizz = c.z;
    }

    pub fn trigger(&mut self, c: &Ctx, _rng: &mut Noise) {
        self.t = 0.0;
        self.length = c.time_s.max(0.15);
        self.pouring = true;
        self.level = 0.3 + 0.7 * c.velocity;
        self.splash = self.level;
        self.fizz_env = self.fizz_env.max(0.3);
    }

    /// The air column's fundamental at pour position `s` (0..1).
    pub fn column_hz(&self, s: f32) -> f32 {
        let s = s.clamp(0.0, 1.0);
        let k = 1.0 - (-self.octaves.abs()).exp2();
        if self.octaves >= 0.0 {
            self.f0 / (1.0 - k * s)
        } else {
            self.f0 * (1.0 - k * s)
        }
    }

    fn spawn(pool: &mut [Bubble], hz: f32, amp: f32, chirp_scale: f32) {
        // Reuse the quietest slot.
        let slot = pool
            .iter_mut()
            .min_by(|a, b| {
                a.amp
                    .partial_cmp(&b.amp)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .unwrap();
        let damping = 0.13 * hz + 0.0072 * hz.powf(1.5);
        *slot = Bubble {
            phase: 0.0,
            hz,
            chirp: 0.1 * damping * chirp_scale,
            amp,
            mul: 0.0,
            t: 0.0,
        };
        slot.mul = damping;
    }

    #[inline]
    fn ring(pool: &mut [Bubble], dt: f32, nyquist: f32) -> f32 {
        let mut out = 0.0;
        for b in pool.iter_mut() {
            if b.amp < 1e-5 {
                continue;
            }
            b.t += dt;
            let hz = b.hz * (1.0 + b.chirp * b.t);
            if hz >= nyquist {
                b.amp = 0.0;
                continue;
            }
            b.phase += hz * dt;
            if b.phase >= 1.0 {
                b.phase -= 1.0;
            }
            // Smooth onset over a quarter cycle avoids a click.
            let onset = (b.t * b.hz * 4.0).min(1.0);
            out += b.amp * onset * sin_turns(b.phase);
            b.amp *= 1.0 - b.mul * dt;
        }
        out
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        let mut s = 1.0;
        let mut on = 0.0;
        if self.pouring {
            self.t += dt;
            s = self.t / self.length;
            if s >= 1.0 {
                self.pouring = false;
                s = 1.0;
            } else {
                on = 1.0;
            }
        }
        let k_env = if on > self.stream_env { 0.0008 } else { 0.0003 };
        self.stream_env += (on - self.stream_env) * k_env * 48_000.0 / self.fs;

        let glug =
            smoothstep((self.flow - 0.5) / 0.45) + if self.octaves < 0.0 { 0.35 } else { 0.0 };
        let bubbly = sin_turns(0.5 * self.flow).max(0.0);

        // The stream: band-limited turbulence, gated into glugs at high FLOW.
        let n = rng.sample();
        self.lp_hi += 0.25 * (n - self.lp_hi);
        self.lp_lo += 0.02 * (n - self.lp_lo);
        let band = self.lp_hi - self.lp_lo;
        self.turbulence += 0.0015 * (rng.sample() - self.turbulence);
        self.glug_phase += (5.5 + 4.0 * glug + 25.0 * self.turbulence) * dt;
        if self.glug_phase >= 1.0 {
            self.glug_phase -= 1.0;
            if self.stream_env > 0.2 && glug > 0.05 {
                // Each glug is a big bubble breaking free.
                let hz = 140.0 + 260.0 * rng.uniform();
                Self::spawn(&mut self.bubbles, hz, 0.5 * glug.min(1.0) * self.level, 0.6);
            }
        }
        let gate = 1.0 - glug.min(1.0) * (0.85 - 0.85 * sin_turns(self.glug_phase).max(0.0));
        let stream = band * self.stream_env * gate * (1.0 + 6.0 * self.turbulence) * self.level;

        // Bubbles: Poisson arrivals while the water runs.
        let rate = self.stream_env * (6.0 + 70.0 * bubbly);
        if rng.uniform() < rate * dt {
            // Radii between 0.6 and 4 mm, log-uniformly.
            let radius = 0.0006 * (4.0f32 / 0.6).powf(rng.uniform());
            let hz = (3.26 / radius).min(0.4 * self.fs);
            let amp = 0.3 * (radius / 0.002).sqrt() * self.level * (0.5 + rng.uniform());
            Self::spawn(&mut self.bubbles, hz, amp, 1.0);
        }
        // Fizz: tiny bubbles bursting at the surface, carrying on after the pour.
        let fizz = self.fizz * self.fizz;
        self.fizz_env = if self.pouring {
            (self.fizz_env + dt / self.length).min(1.0)
        } else {
            self.fizz_env * (1.0 - dt / (1.2 * self.length + 0.5))
        };
        if fizz > 0.0 && rng.uniform() < fizz * self.fizz_env * 500.0 * dt {
            let hz = 2_000.0 + 7_000.0 * rng.uniform();
            Self::spawn(&mut self.pops, hz, 0.12 * fizz.sqrt() * rng.uniform(), 0.0);
        }
        let nyquist = 0.45 * self.fs;
        let bubbles = Self::ring(&mut self.bubbles, dt, nyquist);
        let pops = Self::ring(&mut self.pops, dt, nyquist);

        self.splash *= 1.0 - 30.0 * dt;
        let splash = self.splash * n * 0.4;

        // The air column: a delay loop with an inverting, lossy reflection.
        let hz = self.column_hz(s);
        let delay = self.fs / (2.0 * hz);
        let back = self.tube.read(delay);
        self.tube_lp += 0.55 * (back - self.tube_lp);
        let input = 0.35 * stream + 0.5 * bubbles + splash;
        let y = input - 0.975 * self.tube_lp;
        self.tube.write(y);

        0.9 * y + 0.35 * bubbles + pops + 0.05 * stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filling_raises_the_column_pitch_hyperbolically() {
        let fs = 48_000.0;
        let mut p = Pour::new(fs);
        let c = Ctx {
            hz: 200.0,
            x: 1.0,
            ..Ctx::new(fs)
        };
        p.control(&c);
        let oct = map::fill_octaves(1.0);
        assert!((p.column_hz(0.0) - 200.0).abs() < 0.01);
        assert!((p.column_hz(1.0) / 200.0 - oct.exp2()).abs() < 0.01);
        // Accelerating: the second half climbs further than the first.
        let first = (p.column_hz(0.5) / p.column_hz(0.0)).log2();
        let second = (p.column_hz(1.0) / p.column_hz(0.5)).log2();
        assert!(second > first * 1.5, "{first} {second}");
        let c = Ctx { x: 0.0, ..c };
        p.control(&c);
        assert!(p.column_hz(1.0) < 100.0);
    }

    #[test]
    fn pours_and_settles() {
        let fs = 48_000.0;
        let mut p = Pour::new(fs);
        let c = Ctx {
            hz: 180.0,
            x: 0.9,
            y: 0.5,
            z: 0.5,
            time_s: 1.0,
            ..Ctx::new(fs)
        };
        p.control(&c);
        let mut rng = Noise::new(9);
        p.trigger(&c, &mut rng);
        let out: Vec<f32> = (0..fs as usize * 6).map(|_| p.tick(&c, &mut rng)).collect();
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 6.0));
        assert!(rms(&out[..48_000]) > 0.01);
        assert!(rms(&out[out.len() - 4_800..]) < 0.01 * rms(&out[..48_000]));
    }
}
