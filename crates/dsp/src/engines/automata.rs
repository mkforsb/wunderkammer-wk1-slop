//! AUTOMATA: an elementary cellular automaton playing the harmonic series.
//!
//! Thirty-two cells in a ring evolve by one of Wolfram's elementary rules.
//! Each cell owns a partial: while the cell lives its partial sounds, and every
//! generation the pattern (and so the chord) changes. Rule 90 grows
//! Sierpiński triangles out of a single cell, so you hear the fractal
//! unfold. Rule 30 is chaotic, rule 110 is Turing-complete, and the rest are
//! somewhere in between. At slow rates it arpeggiates; past ~30 generations a
//! second the chord changes become a buzzing timbre of their own.
//!
//! X = RULE (16 hand-picked rules from a single seed cell; RND: a random
//! long-lived rule and a random first row each hit), Y = RATE (2–300
//! generations/s), Z = MAP (cell k → harmonic k+1 → odd harmonics →
//! stretched bell partials → undertones → a pentatonic ladder, morphing in
//! log frequency).

use super::Ctx;
use crate::params::{CELLMAP_NAMES, map};
use crate::util::{Noise, cos_turns, sin_turns, t60_gain};

const CELLS: usize = 32;
const PENTA: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];

#[derive(Clone, Debug)]
pub struct Automata {
    fs: f32,
    cells: u32,
    rule: u8,
    rule_choice: Option<u8>,
    rate: f32,
    map: f32,
    hz: f32,
    to_next: f32,
    /// Partial oscillators as complex phasors, and their rotations.
    re: [f32; CELLS],
    im: [f32; CELLS],
    rot: [(f32, f32); CELLS],
    weight: [f32; CELLS],
    amp: [f32; CELLS],
    /// Partials enter quickly and leave over a fraction of a generation.
    k_on: f32,
    k_amp: f32,
    env: f32,
    env_mul: f32,
    renorm: u32,
}

impl Automata {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            cells: 1 << 16,
            rule: 90,
            rule_choice: Some(90),
            rate: 10.0,
            map: 0.0,
            hz: 110.0,
            to_next: 1.0,
            re: [1.0; CELLS],
            im: [0.0; CELLS],
            rot: [(1.0, 0.0); CELLS],
            weight: [0.0; CELLS],
            amp: [0.0; CELLS],
            k_on: 0.01,
            k_amp: 0.01,
            env: 0.0,
            env_mul: 0.0,
            renorm: 0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    /// The frequency ratio of cell `k` under each map.
    fn ratio(map_index: usize, k: usize) -> f32 {
        let n = (k + 1) as f32;
        match map_index {
            0 => n,
            1 => 2.0 * n - 1.0,
            2 => n.powf(1.45),
            3 => 16.0 / n,
            _ => {
                let octave = (k / 5) as f32;
                (octave + PENTA[k % 5] / 12.0 - 1.0).exp2()
            }
        }
    }

    pub fn control(&mut self, c: &Ctx) {
        self.rule_choice = map::rule(c.x);
        self.rate = map::ca_rate(c.y);
        self.hz = c.hz;
        self.map = c.z;
        let (i, f) = map::segment(c.z, CELLMAP_NAMES.len());
        let nyquist = 0.45 * c.fs;
        for k in 0..CELLS {
            let a = Self::ratio(i, k).log2();
            let b = Self::ratio(i + 1, k).log2();
            let hz = c.hz * (a + (b - a) * f).exp2();
            if hz >= nyquist || hz < 10.0 {
                self.weight[k] = 0.0;
                self.rot[k] = (1.0, 0.0);
                continue;
            }
            // A gentle spectral tilt keeps the high partials from screaming.
            self.weight[k] = (hz / c.hz).powf(-0.35).min(1.4);
            let turns = hz / c.fs;
            self.rot[k] = (cos_turns(turns), sin_turns(turns));
        }
        let tau = (0.35 / self.rate).clamp(0.002, 0.04);
        self.k_amp = 1.0 - (-1.0 / (tau * c.fs)).exp();
        self.k_on = 1.0 - (-1.0 / (0.002 * c.fs)).exp();
    }

    fn step(&mut self) {
        let x = self.cells;
        let mut next = 0u32;
        for i in 0..CELLS as u32 {
            let l = (x >> ((i + 1) % 32)) & 1;
            let c = (x >> i) & 1;
            let r = (x >> ((i + 31) % 32)) & 1;
            let pattern = (l << 2) | (c << 1) | r;
            next |= (((self.rule >> pattern) & 1) as u32) << i;
        }
        self.cells = next;
    }

    /// A rule is worth playing if it neither dies nor freezes nor blinks.
    fn lively(rule: u8, start: u32) -> bool {
        let mut a = Self {
            rule,
            cells: start,
            ..Self::new(1.0)
        };
        let mut history = [0u32; 3];
        for g in 0..48 {
            a.step();
            if a.cells == 0 || a.cells == u32::MAX {
                return false;
            }
            if g >= 45 {
                history[g - 45] = a.cells;
            }
        }
        history[0] != history[2] && history[1] != history[2]
    }

    pub fn trigger(&mut self, c: &Ctx, rng: &mut Noise) {
        match self.rule_choice {
            Some(rule) => {
                self.rule = rule;
                self.cells = 1 << 16;
            }
            None => {
                for _ in 0..64 {
                    let rule = (rng.next_u32() >> 9) as u8;
                    let start = rng.next_u32() & rng.next_u32() | 1 << 16;
                    if Self::lively(rule, start) {
                        self.rule = rule;
                        self.cells = start;
                        break;
                    }
                }
            }
        }
        self.to_next = 1.0;
        self.env = 0.3 + 0.7 * c.velocity;
        self.env_mul = t60_gain(c.time_s.max(0.1), self.fs);
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, _rng: &mut Noise) -> f32 {
        self.to_next -= self.rate / self.fs;
        if self.to_next <= 0.0 {
            self.to_next += 1.0;
            self.step();
            // Rule 90 on a ring of 32 dies after 16 generations, and others
            // die or fill up too: plant a new seed and let it grow again.
            if self.cells == 0 || self.cells == u32::MAX {
                self.cells = 1 << 16;
            }
        }
        self.env *= self.env_mul;
        if self.env < 1e-6 {
            self.env = 0.0;
        }
        let mut out = 0.0;
        let mut power = 0.0;
        for k in 0..CELLS {
            let target = ((self.cells >> k) & 1) as f32 * self.weight[k];
            let rate = if target > self.amp[k] {
                self.k_on
            } else {
                self.k_amp
            };
            self.amp[k] += (target - self.amp[k]) * rate;
            let (c, s) = self.rot[k];
            let re = self.re[k] * c - self.im[k] * s;
            let im = self.re[k] * s + self.im[k] * c;
            self.re[k] = re;
            self.im[k] = im;
            out += self.amp[k] * im;
            power += self.amp[k] * self.amp[k];
        }
        // Keep the phasors on the unit circle.
        self.renorm += 1;
        if self.renorm >= 256 {
            self.renorm = 0;
            for k in 0..CELLS {
                let m = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt();
                if m > 0.0 {
                    self.re[k] /= m;
                    self.im[k] /= m;
                } else {
                    self.re[k] = 1.0;
                }
            }
        }
        // As loud with one live cell as with twenty.
        out * self.env * 0.45 / (0.15 + power.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_90_draws_sierpinski() {
        let mut a = Automata::new(1.0);
        a.rule = 90;
        a.cells = 1 << 16;
        a.step();
        assert_eq!(a.cells, (1 << 15) | (1 << 17));
        a.step();
        assert_eq!(a.cells, (1 << 14) | (1 << 18));
        a.step();
        assert_eq!(a.cells, (1 << 13) | (1 << 15) | (1 << 17) | (1 << 19));
    }

    #[test]
    fn plays_and_decays() {
        let fs = 48_000.0;
        for x in [0.0, 0.3, 0.7, 1.0] {
            let mut a = Automata::new(fs);
            let c = Ctx {
                hz: 110.0,
                x,
                y: 0.4,
                z: 0.5,
                time_s: 0.5,
                ..Ctx::new(fs)
            };
            a.control(&c);
            let mut rng = Noise::new(2);
            a.trigger(&c, &mut rng);
            let out: Vec<f32> = (0..fs as usize * 2).map(|_| a.tick(&c, &mut rng)).collect();
            let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
            assert!(out.iter().all(|v| v.is_finite() && v.abs() < 3.0));
            assert!(rms(&out[..4_800]) > 0.01, "x {x}");
            assert!(rms(&out[out.len() - 4_800..]) < 1e-4, "x {x}");
        }
    }
}
