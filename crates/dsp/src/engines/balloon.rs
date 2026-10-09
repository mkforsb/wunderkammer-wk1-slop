//! BALLOON: a balloon let go.
//!
//! The pressure inside a rubber balloon is not monotonic in its size. For a
//! neo-Hookean membrane `P ∝ 1/λ − 1/λ⁷` (λ the stretch), which *rises*
//! as the balloon shrinks, peaks at λ = 7^1/6 ≈ 1.38, then collapses. So a
//! deflating balloon whines upward before it gives out in a flapping
//! raspberry. Air escapes through the nozzle at a rate ∝ √P, and the time to
//! empty is set by TIME.
//!
//! The nozzle's rubber lips are a self-oscillating flap, modelled as a Van
//! der Pol oscillator whose drive and pitch follow the pressure. A little
//! flap is a pure squeal; more is a relaxation buzz, and above halfway a
//! second, lower lip mode couples in and the flapping breaks into
//! subharmonics and chaos. Its flow, rectified by the lips, is what
//! radiates.
//!
//! Let go (FLIGHT), the balloon jets around the room on a random path,
//! steered by its own thrust: its distance sets the level and dulls the
//! highs, its speed towards or away from you Doppler-shifts the pitch
//! (exaggerated, as if the room were full of a slow, heavy gas), and
//! it lands with a soft plop at the end. Hitting it again blows more air in.
//!
//! X = SIZE (initial stretch: small balloons start near the pressure peak,
//! big ones whine upward for longer), Y = FLAP, Z = FLIGHT.

use super::Ctx;
use crate::params::map;
use crate::util::{Mode, Noise, sin_turns};

/// Peak of `1/λ − 1/λ⁷`, at λ = 7^1/6.
const P_PEAK: f32 = 0.619_7;
const THRESHOLD: f32 = 0.1;
const SOUND: f32 = 343.0;
const ROOM: f32 = 3.5;
const START: [f32; 3] = [0.0, 0.0, 1.2];

fn pressure(lambda: f32) -> f32 {
    let l = lambda.max(1.0);
    ((1.0 / l) - (1.0 / l.powi(7))) / P_PEAK
}

#[derive(Clone, Debug)]
pub struct Balloon {
    fs: f32,
    lambda: f32,
    lambda0: f32,
    rate: f32,
    active: bool,
    hz: f32,
    flap: f32,
    flight: f32,
    level: f32,
    /// Lip oscillators (displacement, velocity).
    x: f32,
    v: f32,
    y: f32,
    w: f32,
    flow_prev: f32,
    hiss: f32,
    /// Flight state.
    pos: [f32; 3],
    vel: [f32; 3],
    dir: [f32; 3],
    spin: f32,
    air_lp: f32,
    plop: Mode,
    t: f32,
}

impl Balloon {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            lambda: 1.0,
            lambda0: 2.5,
            rate: 1.0,
            active: false,
            hz: 300.0,
            flap: 0.3,
            flight: 0.0,
            level: 1.0,
            x: 0.0,
            v: 0.0,
            y: 0.0,
            w: 0.0,
            flow_prev: 0.0,
            hiss: 0.0,
            pos: START,
            vel: [0.0; 3],
            dir: [0.0, 0.3, -0.95],
            spin: 0.0,
            air_lp: 0.0,
            plop: Mode::default(),
            t: 0.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.lambda0 = map::balloon_stretch(c.x);
        self.hz = c.hz;
        self.flap = c.y;
        self.flight = c.z;
        self.plop.set(95.0, 0.18, c.fs);
    }

    /// `∫ 3λ²/√P dλ` from 1 to λ₀ (substituting λ = 1 + u² to tame the
    /// square-root singularity at 1): emptying time is this over the rate.
    fn emptying_integral(lambda0: f32) -> f32 {
        let top = (lambda0 - 1.0).max(1e-4).sqrt();
        let n = 128;
        let du = top / n as f32;
        (0..n)
            .map(|i| {
                let u = (i as f32 + 0.5) * du;
                let l = 1.0 + u * u;
                3.0 * l * l * 2.0 * u / pressure(l).max(1e-9).sqrt() * du
            })
            .sum()
    }

    pub fn trigger(&mut self, c: &Ctx, _rng: &mut Noise) {
        if self.active {
            // A puff of air into a balloon that is still going.
            self.lambda = (self.lambda + 0.6 * (self.lambda0 - 1.0) * c.velocity).min(3.9);
        } else {
            self.lambda = self.lambda0;
            self.pos = START;
            self.vel = [0.0; 3];
            self.dir = [0.0, 0.3, -0.95];
            self.t = 0.0;
            self.x = 0.01;
            self.y = 0.005;
        }
        self.rate = Self::emptying_integral(self.lambda0) / c.time_s.max(0.15);
        self.level = 0.35 + 0.65 * c.velocity;
        self.active = true;
    }

    fn fly(&mut self, thrust: f32, dt: f32, rng: &mut Noise) -> (f32, f32) {
        if self.flight <= 0.0 {
            return (1.0, 1.0);
        }
        // Wobbling heading, thrust along it, air drag, a bit of gravity.
        let turn = 7.0 * self.flight;
        for d in &mut self.dir {
            *d += turn * rng.sample() * dt.sqrt() * 1.5;
        }
        let n = self.dir.iter().map(|d| d * d).sum::<f32>().sqrt().max(1e-6);
        let push = thrust * self.flight * 40.0;
        for i in 0..3 {
            self.dir[i] /= n;
            self.vel[i] += (push * self.dir[i] - 1.6 * self.vel[i]) * dt;
        }
        self.vel[1] -= (1.0 - thrust).max(0.0) * 3.0 * dt;
        for i in 0..3 {
            self.pos[i] += self.vel[i] * dt;
            if self.pos[i].abs() > ROOM {
                self.pos[i] = self.pos[i].clamp(-ROOM, ROOM);
                self.vel[i] *= -0.6;
                self.dir[i] = -self.dir[i];
            }
        }
        let d = self.pos.iter().map(|p| p * p).sum::<f32>().sqrt().max(0.35);
        let radial = (0..3).map(|i| self.pos[i] * self.vel[i]).sum::<f32>() / d;
        // Exaggerated with FLIGHT, as if the room were filled with a slow gas.
        let sound = SOUND / (1.0 + 3.0 * self.flight);
        let doppler = sound / (sound + radial.clamp(-0.6 * sound, 0.6 * sound));
        let gain = (START[2] / d).min(2.5);
        let g = 1.0 - self.flight;
        (g + self.flight * doppler, g + self.flight * gain)
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        let mut p = 0.0;
        if self.active {
            self.t += dt;
            p = pressure(self.lambda);
            let l = self.lambda;
            self.lambda -= self.rate * p.sqrt() / (3.0 * l * l) * dt;
            if self.lambda <= 1.000_05 || p < 0.004 {
                self.active = false;
                self.lambda = 1.0;
                if self.flight > 0.3 {
                    self.plop.re += 0.5 * self.flight * self.level;
                }
            }
        }
        let (doppler, gain) = self.fly(p, dt, rng);

        let drive = p - THRESHOLD;
        // Tighter lips under more pressure: the whine climbs with it.
        let hz = self.hz * (0.1 + 0.9 * p * p) * doppler;
        let omega = core::f32::consts::TAU * hz.min(0.3 * self.fs);
        let mu = 0.15 + 4.0 * self.flap * self.flap;
        let couple = ((self.flap - 0.5) * 1.2).max(0.0);
        let omega_y = 0.53 * omega;
        // Semi-implicit Euler, substepped so ω·h stays small.
        let steps = ((omega * dt / 0.2).ceil() as usize).clamp(1, 4);
        let h = dt / steps as f32;
        for _ in 0..steps {
            let ax = mu * omega * (drive - self.x * self.x) * self.v - omega * omega * self.x
                + couple * omega * omega * (self.y - self.x);
            self.v += ax * h;
            self.x += self.v * h;
            let ay = mu * omega_y * (0.6 * drive - self.y * self.y) * self.w
                - omega_y * omega_y * self.y
                + couple * omega_y * omega_y * (self.x - self.y);
            self.w += ay * h;
            self.y += self.w * h;
        }
        if drive < 0.0 {
            // Slack rubber stops flapping almost at once.
            let k = 1.0 - (200.0 * dt).min(1.0);
            self.v *= k;
            self.w *= k;
        }
        if !(self.x.is_finite() && self.v.is_finite() && self.y.is_finite() && self.w.is_finite())
            || self.x.abs() > 10.0
        {
            self.x = 0.0;
            self.v = 0.0;
            self.y = 0.0;
            self.w = 0.0;
        }
        // The lips gate the airflow: rectified, so it buzzes.
        let open = (self.x + 0.4 * drive.max(0.0).sqrt()).max(0.0);
        let flow = open * open.sqrt() * p.sqrt();
        // The pulsing flow itself, brightened by a touch of its slope.
        let slope = (flow - self.flow_prev) * self.fs / omega.max(1.0);
        self.flow_prev = flow;
        let buzz = 0.35 * flow + 0.08 * slope;
        self.hiss += 0.5 * (rng.sample() - self.hiss);
        let hiss = self.hiss * p * (0.02 + 0.05 * open.min(1.0));
        let squeal = self.x * 0.5;
        let tone = squeal * (1.0 - self.flap) + buzz * self.flap + hiss;

        // Spinning in flight: the nozzle swings towards and away from you.
        self.spin += (2.0 + 3.0 * self.flight) * dt;
        let spin = 1.0 - 0.3 * self.flight * (0.5 + 0.5 * sin_turns(self.spin));
        // Distance dulls the highs.
        let k = (1.0 / gain.max(0.4)).min(1.0);
        let out = tone * gain * spin;
        self.air_lp += (1.0 - 0.75 * k) * (out - self.air_lp);
        let plop = self.plop.tick(0.0);
        crate::util::fast_tanh((self.air_lp + plop) * self.level * 0.8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_peaks_near_1_38() {
        assert!((pressure(7f32.powf(1.0 / 6.0)) - 1.0).abs() < 1e-3);
        assert!(pressure(2.5) < pressure(1.5));
        assert!(pressure(1.0) == 0.0);
    }

    #[test]
    fn deflates_in_about_time() {
        let fs = 48_000.0;
        for size in [0.0, 0.5, 1.0] {
            let mut b = Balloon::new(fs);
            let c = Ctx {
                hz: 400.0,
                x: size,
                y: 0.3,
                z: 0.0,
                time_s: 1.0,
                ..Ctx::new(fs)
            };
            b.control(&c);
            let mut rng = Noise::new(1);
            b.trigger(&c, &mut rng);
            let mut n = 0;
            while b.active && n < fs as usize * 5 {
                let x = b.tick(&c, &mut rng);
                assert!(x.is_finite() && x.abs() < 6.0);
                n += 1;
            }
            let secs = n as f32 / fs;
            assert!((0.6..1.4).contains(&secs), "size {size}: {secs}");
        }
    }

    #[test]
    fn flight_and_flap_extremes_stay_bounded() {
        let fs = 96_000.0;
        let mut b = Balloon::new(fs);
        let c = Ctx {
            hz: 2_000.0,
            x: 1.0,
            y: 1.0,
            z: 1.0,
            time_s: 3.0,
            ..Ctx::new(fs)
        };
        b.control(&c);
        let mut rng = Noise::new(3);
        b.trigger(&c, &mut rng);
        let mut peak = 0.0f32;
        for _ in 0..fs as usize * 4 {
            let x = b.tick(&c, &mut rng);
            assert!(x.is_finite());
            peak = peak.max(x.abs());
        }
        assert!(peak > 0.01 && peak < 6.0, "{peak}");
    }
}
