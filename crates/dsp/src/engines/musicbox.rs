//! MUSICBOX: a wind-up music box whose pins were punched by number theory.
//!
//! The hit winds the spring and the cylinder starts turning, plucking steel
//! tines (cantilever modes at 1 : 6.27 : 17.55) inside a small wooden box.
//! The spring runs down over TIME, so the tune slows to a halt.
//!
//! The tune is a number sequence, one value per pin:
//!
//! * COLLATZ: the hailstone path of a starting number (`n → n/2` or
//!   `3n + 1`) down to 1. Every path ends in 8, 4, 2, 1, a falling cadence
//!   home to the tonic.
//! * RECAMAN: `a(n) = a(n−1) − n` if that is new and positive, else `+ n`.
//! * PI: the digits of π (a 0 is a rest).
//! * FIB: the Fibonacci numbers modulo m, which loop with the Pisano period.
//! * NORGARD: Per Nørgård's self-similar "infinity series".
//!
//! In the HARMONIC scale a value `v` plays the `v`-th harmonic of the tonic
//! folded into two octaves (so Collatz paths play in pure just intonation,
//! with all their prime-number oddities). The other scales just count steps.
//! Values divisible by three strike a second tine a third below.
//!
//! X = TUNE (sequence and starting point; RND: a new one per hit),
//! Y = SCALE, Z = WEAR (detuned tines, missing pins, a wobbly governor and
//! a buzzing comb).

use super::Ctx;
use crate::params::{SCALE_NAMES, map};
use crate::util::{Mode, Noise};

const MAX_NOTES: usize = 128;
const TINES: usize = 12;
const REST: i32 = i32::MIN;

const PI_DIGITS: &[u8] = b"31415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679821480865132823066470938446095505822317253594081284811174502841027019385211055596446229489549303819644288109756659334461284756482337867831652712019091456485669234603486104543266482133936072602491412737245870066063155881748815209209628292540917153643678925903600113305305488204665213841469519415116094330572703657595919530921861173819326117931051185480744623799627495673518857527248912279381830119491";

const PENTA: &[i32] = &[0, 2, 4, 7, 9];
const MAJOR: &[i32] = &[0, 2, 4, 5, 7, 9, 11];
const HIJAZ: &[i32] = &[0, 1, 4, 5, 7, 8, 10];
const WHOLE: &[i32] = &[0, 2, 4, 6, 8, 10];

#[derive(Clone, Copy, Debug, Default)]
struct Tine {
    modes: [Mode; 3],
    hz: f32,
    level: f32,
}

#[derive(Clone, Debug)]
pub struct MusicBox {
    fs: f32,
    notes: [i32; MAX_NOTES],
    count: usize,
    harmonic_values: bool,
    tune: Option<(u32, u32)>,
    scale: usize,
    wear: f32,
    tonic: f32,
    /// Position in the tune, steps per second at full wind, spring time.
    step: usize,
    to_next: f32,
    rate: f32,
    t: f32,
    spring: f32,
    playing: bool,
    level: f32,
    tines: [Tine; TINES],
    next_tine: usize,
    body: [Mode; 3],
    tick_env: f32,
    flutter: f32,
}

impl MusicBox {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            notes: [REST; MAX_NOTES],
            count: 0,
            harmonic_values: false,
            tune: Some((0, 0)),
            scale: 0,
            wear: 0.0,
            tonic: 440.0,
            step: 0,
            to_next: 0.0,
            rate: 8.0,
            t: 0.0,
            spring: 1.0,
            playing: false,
            level: 1.0,
            tines: [Tine::default(); TINES],
            next_tine: 0,
            body: [Mode::default(); 3],
            tick_env: 0.0,
            flutter: 0.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.tune = map::tune(c.x);
        self.scale = map::choose(c.y, SCALE_NAMES.len());
        self.wear = c.z;
        self.tonic = c.hz;
        for (m, hz) in self.body.iter_mut().zip([180.0, 410.0, 760.0]) {
            m.set(hz, 0.08, c.fs);
        }
    }

    /// Punch the cylinder.
    fn punch(&mut self, kind: u32, n: u32) {
        let mut out = [REST; MAX_NOTES];
        let mut len = 0;
        let mut push = |v: i32, len: &mut usize| {
            if *len < MAX_NOTES {
                out[*len] = v;
                *len += 1;
            }
        };
        self.harmonic_values = false;
        match kind {
            0 => {
                let mut v = map::collatz_start(n) as u64;
                self.harmonic_values = true;
                push(v as i32, &mut len);
                while v != 1 && len < MAX_NOTES {
                    v = if v.is_multiple_of(2) {
                        v / 2
                    } else {
                        3 * v + 1
                    };
                    push(v as i32, &mut len);
                }
            }
            1 => {
                let mut seen = [false; 4096];
                let mut a: i64 = 0;
                seen[0] = true;
                let start = n as i64;
                for i in 1..(start + 64) {
                    let down = a - i;
                    a = if down > 0 && !seen[down as usize] {
                        down
                    } else {
                        a + i
                    };
                    if (a as usize) < seen.len() {
                        seen[a as usize] = true;
                    }
                    if i >= start {
                        push(a as i32, &mut len);
                    }
                }
            }
            2 => {
                let start = (n as usize * 7) % (PI_DIGITS.len() - 64);
                for &d in &PI_DIGITS[start..start + 64] {
                    let d = (d - b'0') as i32;
                    push(if d == 0 { REST } else { d }, &mut len);
                }
            }
            3 => {
                let m = 5 + (n % 9) as i32;
                let (mut a, mut b) = (0i32, 1i32);
                for _ in 0..48 {
                    push(a, &mut len);
                    (a, b) = (b, (a + b) % m);
                }
            }
            _ => {
                // Nørgård: a(0) = 0, a(2n) = −a(n), a(2n+1) = a(n) + 1.
                let start = n as usize * 8;
                let mut series = [0i32; 128];
                for i in 1..series.len() {
                    series[i] = if i % 2 == 0 {
                        -series[i / 2]
                    } else {
                        series[i / 2] + 1
                    };
                }
                for &v in series.iter().skip(start % 64).take(64) {
                    push(v, &mut len);
                }
            }
        }
        self.notes = out;
        self.count = len;
    }

    /// Frequency of a sequence value in the current scale.
    pub fn note_hz(&self, v: i32) -> f32 {
        let tonic = self.tonic;
        let semis = |scale: &[i32], d: i32| {
            let len = scale.len() as i32;
            let d = d.rem_euclid(2 * len);
            scale[(d % len) as usize] + 12 * (d / len)
        };
        match self.scale {
            0 => {
                // The v-th harmonic, folded into two octaves.
                let h = if self.harmonic_values {
                    v.max(1)
                } else {
                    v.unsigned_abs() as i32 + 1
                };
                let h = h as f32;
                let oct = h.log2().floor();
                let folded = h / (oct - (oct as i32 % 2) as f32).exp2();
                tonic * folded
            }
            1 => tonic * (semis(PENTA, v) as f32 / 12.0).exp2(),
            2 => tonic * (semis(MAJOR, v) as f32 / 12.0).exp2(),
            3 => tonic * (semis(HIJAZ, v) as f32 / 12.0).exp2(),
            4 => tonic * (semis(WHOLE, v) as f32 / 12.0).exp2(),
            // Bohlen–Pierce: 13 equal steps of a tritave.
            _ => tonic * 3f32.powf(v.rem_euclid(20) as f32 / 13.0),
        }
    }

    pub fn trigger(&mut self, c: &Ctx, rng: &mut Noise) {
        let (kind, n) = self
            .tune
            .unwrap_or_else(|| (rng.next_u32() % 5, rng.next_u32() % 40));
        self.punch(kind, n);
        self.step = 0;
        self.to_next = 0.0;
        self.t = 0.0;
        self.spring = c.time_s.max(0.3) * 1.5;
        self.rate = 5.0 + 7.0 * c.velocity;
        self.level = 0.4 + 0.6 * c.velocity;
        self.playing = true;
    }

    /// A fixed detune per pitch, in cents at full wear (so a worn box is
    /// consistently out of tune rather than randomly so).
    fn rust(hz: f32) -> f32 {
        let h = (hz * 7.31).to_bits().wrapping_mul(0x9E37_79B9);
        ((h >> 8) as f32 / 16_777_216.0 - 0.5) * 2.0
    }

    fn pluck(&mut self, hz: f32, amp: f32) {
        let hz = hz * (self.wear * self.wear * 0.6 * Self::rust(hz) / 12.0).exp2();
        // Reuse the tine already tuned to this pitch, else the next one.
        let slot = self
            .tines
            .iter()
            .position(|t| (t.hz - hz).abs() < 0.01)
            .unwrap_or_else(|| {
                self.next_tine = (self.next_tine + 1) % TINES;
                self.next_tine
            });
        let tine = &mut self.tines[slot];
        if tine.hz != hz {
            tine.hz = hz;
            let t60 = (2.2 * (523.0 / hz).sqrt()).clamp(0.4, 5.0);
            tine.modes[0].set(hz, t60, self.fs);
            tine.modes[1].set(hz * 6.27, 0.25, self.fs);
            tine.modes[2].set(hz * 17.55, 0.06, self.fs);
            for m in &mut tine.modes {
                m.clear();
            }
        }
        tine.level = amp;
        for (m, g) in tine.modes.iter_mut().zip([1.0, 0.28, 0.08]) {
            m.re += amp * g;
        }
        for (m, g) in self.body.iter_mut().zip([0.6, 0.4, 0.3]) {
            m.re += amp * g * 0.08;
        }
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        if self.playing {
            self.t += dt;
            let wind = (1.0 - self.t / self.spring).max(0.0).sqrt();
            self.flutter += 0.0005 * (rng.sample() - self.flutter);
            let speed = self.rate * wind * (1.0 + 6.0 * self.wear * self.flutter);
            if speed < 0.4 || self.step >= self.count {
                self.playing = false;
            } else {
                self.to_next -= speed * dt;
                if self.to_next <= 0.0 {
                    self.to_next += 1.0;
                    let v = self.notes[self.step];
                    self.step += 1;
                    self.tick_env = 0.02 + 0.06 * self.wear;
                    let missing = rng.uniform() < 0.35 * self.wear * self.wear;
                    if v != REST && !missing {
                        let hz = self.note_hz(v);
                        let level = self.level;
                        self.pluck(hz, level);
                        if v % 3 == 0 && v != 0 {
                            let below = self.note_hz(v - 2);
                            self.pluck(below, level * 0.45);
                        }
                    }
                }
            }
        }
        let mut out = 0.0;
        for tine in &mut self.tines {
            let [a, b, c] = &mut tine.modes;
            out += a.tick(0.0) + b.tick(0.0) + c.tick(0.0);
        }
        // A worn comb buzzes against the cylinder on loud notes.
        let buzz = ((self.wear - 0.5) * 2.0).clamp(0.0, 1.0);
        if buzz > 0.0 {
            out += buzz * 0.8 * (out - out.clamp(-0.15, 0.15));
        }
        let mut body = 0.0;
        for m in &mut self.body {
            body += m.tick(0.0);
        }
        self.tick_env *= 1.0 - 2_500.0 * dt;
        let mechanism = self.tick_env * rng.sample();
        0.4 * out + body + mechanism
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collatz_ends_on_the_tonic() {
        let mut m = MusicBox::new(48_000.0);
        m.punch(0, 0);
        assert_eq!(m.notes[0], 27);
        assert_eq!(m.count, 112);
        assert_eq!(&m.notes[m.count - 4..m.count], &[8, 4, 2, 1]);
        m.tonic = 400.0;
        m.scale = 0;
        assert!((m.note_hz(1) - 400.0).abs() < 1e-3);
        assert!((m.note_hz(2) - 800.0).abs() < 1e-3);
        assert!((m.note_hz(4) - 400.0).abs() < 1e-3);
        // 27/16 is a Pythagorean major sixth above the tonic.
        assert!((m.note_hz(27) - 400.0 * 27.0 / 16.0).abs() < 1e-2);
    }

    #[test]
    fn recaman_starts_right() {
        let mut m = MusicBox::new(48_000.0);
        m.punch(1, 1);
        assert_eq!(&m.notes[..8], &[1, 3, 6, 2, 7, 13, 20, 12]);
    }

    #[test]
    fn norgard_series() {
        let mut m = MusicBox::new(48_000.0);
        m.punch(4, 0);
        assert_eq!(&m.notes[..8], &[0, 1, -1, 2, 1, 0, -2, 3]);
    }

    #[test]
    fn plays_then_runs_down() {
        let fs = 48_000.0;
        let mut m = MusicBox::new(fs);
        let c = Ctx {
            hz: 500.0,
            x: 0.0,
            time_s: 1.0,
            ..Ctx::new(fs)
        };
        m.control(&c);
        let mut rng = Noise::new(1);
        m.trigger(&c, &mut rng);
        let out: Vec<f32> = (0..fs as usize * 8).map(|_| m.tick(&c, &mut rng)).collect();
        assert!(!m.playing);
        assert!(m.step > 4, "{}", m.step);
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 4.0));
        let tail = &out[out.len() - 4_800..];
        assert!(tail.iter().all(|x| x.abs() < 1e-3));
    }
}
