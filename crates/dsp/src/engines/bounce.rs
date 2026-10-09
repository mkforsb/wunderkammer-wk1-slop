//! BOUNCE: balls dropped on a struck drumhead.
//!
//! The hit strikes the head and throws the balls resting on it into the air.
//! Each ball then follows real ballistics (g = 9.81 m/s²) and bounces with a
//! coefficient of restitution, so the rhythm writes itself: the familiar
//! accelerating "tok … tok .. tok.tktktrrr" of a dropped ping-pong ball, or,
//! with many tiny balls, popcorn and snare-wire sizzle. Every impact rings
//! both the head (eight circular-membrane modes) and the ball itself.
//!
//! SHAKE drives the head like a vibrating table (the classic "bouncing ball
//! on a vibrating plate", a textbook route to chaos). Below Γ = 1 a resting
//! ball stays put; above it the table throws the balls, and their rhythm
//! locks to the table, period-doubles or turns chaotic, for as long as TIME
//! keeps the table running.
//!
//! X = COUNT (1 heavy ball … 24 grains), Y = BOUNCE (restitution),
//! Z = SHAKE (peak table acceleration Γ in g).

use super::Ctx;
use crate::params::map;
use crate::util::{Mode, Noise, sin_turns};

const MAX_BALLS: usize = 24;
const G: f32 = 9.81;
/// Table shake frequency.
const SHAKE_HZ: f32 = 11.0;
/// Bessel-zero ratios of an ideal circular membrane.
const HEAD_RATIOS: [f32; 8] = [1.0, 1.594, 2.136, 2.296, 2.653, 2.918, 3.156, 3.501];
const HEAD_GAINS: [f32; 8] = [1.0, 0.75, 0.6, 0.5, 0.42, 0.36, 0.3, 0.25];
/// Ball groups, each tuned slightly differently (balls are never identical).
const GROUPS: usize = 4;
const GROUP_DETUNE: [f32; GROUPS] = [1.0, 1.137, 0.917, 1.271];
/// Below this impact speed a ball comes to rest.
const REST_SPEED: f32 = 0.018;

#[derive(Clone, Copy, Debug, Default)]
struct Ball {
    /// Height above the table's rest position, metres.
    h: f32,
    /// Vertical velocity, m/s.
    v: f32,
    resting: bool,
}

#[derive(Clone, Debug)]
pub struct Bounce {
    fs: f32,
    balls: [Ball; MAX_BALLS],
    count: usize,
    restitution: f32,
    head: [Mode; 8],
    pings: [[Mode; 2]; GROUPS],
    /// Contact click envelope and the noise it gates.
    click: f32,
    click_mul: f32,
    /// Table: phase in turns, peak displacement now and at the hit.
    shake_phase: f32,
    shake_amp: f32,
    shake_peak: f32,
    /// Seconds since the hit (drives the table's run-down).
    t: f32,
    time_s: f32,
    mass: f32,
}

impl Bounce {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            balls: [Ball {
                resting: true,
                ..Default::default()
            }; MAX_BALLS],
            count: 1,
            restitution: 0.8,
            head: [Mode::default(); 8],
            pings: [[Mode::default(); 2]; GROUPS],
            click: 0.0,
            click_mul: (-1.0 / (0.000_25 * fs)).exp(),
            shake_phase: 0.0,
            shake_amp: 0.0,
            shake_peak: 0.0,
            t: 10.0,
            time_s: 1.0,
            mass: 1.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        let count = map::ball_count(c.x);
        if count > self.count {
            // New balls appear resting on the table.
            for b in &mut self.balls[self.count..count] {
                *b = Ball {
                    resting: true,
                    ..Default::default()
                };
            }
        }
        self.count = count;
        self.mass = 1.6 / (count as f32).sqrt();
        self.restitution = map::restitution(c.y);
        self.time_s = c.time_s;
        let omega = core::f32::consts::TAU * SHAKE_HZ;
        self.shake_peak = map::shake_gamma(c.z) * G / (omega * omega);

        let head_t60 = (0.12 + 0.5 * c.time_s).min(2.5);
        for (k, m) in self.head.iter_mut().enumerate() {
            m.set(
                c.hz * HEAD_RATIOS[k],
                head_t60 / (1.0 + 0.45 * k as f32),
                c.fs,
            );
        }
        // Few big balls ping low and long; many small ones high and short.
        let size = (count - 1) as f32 / (MAX_BALLS - 1) as f32;
        let ping_hz = c.hz * (3.1 + 4.5 * size);
        let ping_t60 = 0.09 - 0.06 * size;
        for (g, pair) in self.pings.iter_mut().enumerate() {
            pair[0].set(ping_hz * GROUP_DETUNE[g], ping_t60, c.fs);
            pair[1].set(ping_hz * GROUP_DETUNE[g] * 2.76, ping_t60 * 0.5, c.fs);
        }
    }

    pub fn trigger(&mut self, c: &Ctx, rng: &mut Noise) {
        self.t = 0.0;
        // Strike the head.
        for (k, m) in self.head.iter_mut().enumerate() {
            m.re += 0.45 * c.velocity * HEAD_GAINS[k] * if k % 2 == 0 { 1.0 } else { -1.0 };
        }
        self.click = self.click.max(c.velocity);
        // Throw every ball that is on (or very near) the head.
        let (y, vt, _) = self.table();
        let launch = 0.15 + 1.1 * c.velocity;
        let spread = if self.count > 1 { 0.6 } else { 0.1 };
        for b in &mut self.balls[..self.count] {
            if b.resting || b.h - y < 0.003 {
                b.resting = false;
                b.h = y;
                b.v = vt + launch * (1.0 - spread * rng.uniform());
            }
        }
    }

    /// Table displacement, velocity and acceleration.
    #[inline]
    fn table(&self) -> (f32, f32, f32) {
        if self.shake_amp <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let w = core::f32::consts::TAU * SHAKE_HZ;
        let s = sin_turns(self.shake_phase);
        let co = sin_turns(self.shake_phase + 0.25);
        (
            self.shake_amp * s,
            self.shake_amp * w * co,
            -self.shake_amp * w * w * s,
        )
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        self.t += dt;
        // The table runs at full shake for TIME, then winds down.
        let run = self.time_s.max(0.05);
        self.shake_amp = if self.t < run {
            self.shake_peak
        } else {
            self.shake_peak * (-(self.t - run) / (0.25 * run)).exp()
        };
        self.shake_phase += SHAKE_HZ * dt;
        if self.shake_phase >= 1.0 {
            self.shake_phase -= 1.0;
        }
        let (y, vt, at) = self.table();

        let mut head_in = 0.0;
        let mut ping_in = [0.0f32; GROUPS];
        let e = self.restitution;
        for (i, b) in self.balls[..self.count].iter_mut().enumerate() {
            if b.resting {
                b.h = y;
                b.v = vt;
                // The table drops away faster than gravity: the ball takes off.
                if at < -G {
                    b.resting = false;
                }
                continue;
            }
            b.v -= G * dt;
            b.h += b.v * dt;
            if b.h <= y {
                let rel = b.v - vt;
                b.h = y;
                if rel < 0.0 {
                    let speed = -rel;
                    b.v = vt + e * speed;
                    head_in += self.mass * speed;
                    ping_in[i % GROUPS] += speed;
                    self.click = self.click.max(speed * 0.6);
                    if speed < REST_SPEED {
                        b.resting = true;
                        b.v = vt;
                    }
                }
            }
        }

        let mut head = 0.0;
        for (k, m) in self.head.iter_mut().enumerate() {
            let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
            head += m.tick(0.35 * head_in * HEAD_GAINS[k] * sign);
        }
        let mut ping = 0.0;
        for (pair, &input) in self.pings.iter_mut().zip(&ping_in) {
            ping += pair[0].tick(0.3 * input) + 0.4 * pair[1].tick(0.3 * input);
        }
        self.click *= self.click_mul;
        let click = self.click * rng.sample() * 0.12;
        head + ping + click
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_ball_bounces_faster_and_faster() {
        let fs = 48_000.0;
        let mut b = Bounce::new(fs);
        let c = Ctx {
            x: 0.0,
            y: 0.75,
            z: 0.0,
            hz: 150.0,
            time_s: 0.3,
            ..Ctx::new(fs)
        };
        b.control(&c);
        let mut rng = Noise::new(1);
        b.trigger(&c, &mut rng);
        let mut impacts = Vec::new();
        for n in 0..fs as usize * 2 {
            let before = b.balls[0].v;
            let x = b.tick(&c, &mut rng);
            assert!(x.is_finite() && x.abs() < 4.0);
            if before < 0.0 && b.balls[0].v >= 0.0 {
                impacts.push(n as f32 / fs);
            }
        }
        assert!(impacts.len() >= 5, "{impacts:?}");
        let gaps: Vec<f32> = impacts.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.windows(2).all(|g| g[1] < g[0]), "{gaps:?}");
        // The first flight is set by the throw: 2·v/g.
        assert!(impacts[0] > 0.1 && impacts[0] < 0.4, "{impacts:?}");
        assert!(b.balls[0].resting, "ball should come to rest");
    }

    #[test]
    fn a_shaking_table_keeps_the_balls_going() {
        let fs = 48_000.0;
        let mut b = Bounce::new(fs);
        let c = Ctx {
            x: 0.3,
            y: 0.5,
            z: 0.8,
            time_s: 5.0,
            ..Ctx::new(fs)
        };
        b.control(&c);
        let mut rng = Noise::new(1);
        b.trigger(&c, &mut rng);
        for _ in 0..fs as usize * 3 {
            b.tick(&c, &mut rng);
        }
        let late: Vec<f32> = (0..fs as usize).map(|_| b.tick(&c, &mut rng)).collect();
        let rms = (late.iter().map(|v| v * v).sum::<f32>() / late.len() as f32).sqrt();
        assert!(rms > 1e-3, "{rms}");
    }
}
