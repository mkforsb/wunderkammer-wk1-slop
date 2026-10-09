//! Per-channel effects after the object: the SHIFT spiral and the THROUGH
//! medium.

use crate::params::{
    MEDIUM_HORN, MEDIUM_RADIO, MEDIUM_TIN_CAN, MEDIUM_TUBE, MEDIUM_WALL, MEDIUM_WATER, map,
};
use crate::util::{
    DelayLine, Hilbert, Noise, Svf, SvfCoeffs, fast_tanh, flush, one_pole_coeff, sin_turns,
};

/// Single-sideband frequency shifter in a feedback delay loop.
///
/// Every trip round the loop shifts the sound by the same number of Hz, so
/// with SPIRAL up each echo comes back further detuned and climbs (or falls)
/// out of the audible range: a barber-pole echo. A shift of a few Hz beats
/// and phases; hundreds of Hz turn any harmonic sound into bent metal, because
/// shifting by Hz (not by ratio) breaks the harmonic series.
#[derive(Clone, Debug)]
pub struct Shifter {
    fs: f32,
    hilbert: Hilbert,
    delay: DelayLine,
    phase: f32,
    hz: f32,
    /// 0 = plain echo loop, 1 = fully shifted (crossfades near zero shift).
    mix: f32,
    feedback: f32,
    lag: f32,
    lag_now: f32,
}

impl Shifter {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            hilbert: Hilbert::default(),
            delay: DelayLine::new((fs * 0.65) as usize),
            phase: 0.0,
            hz: 0.0,
            mix: 0.0,
            feedback: 0.0,
            lag: 0.1,
            lag_now: 0.1,
        }
    }

    pub fn control(&mut self, shift: f32, spiral: f32, lag: f32) {
        self.hz = map::shift_hz(shift);
        self.mix = (map::bipolar(shift).abs() / 0.06).min(1.0);
        self.feedback = map::shift_spiral(spiral);
        self.lag = map::shift_lag_seconds(lag) * self.fs;
    }

    pub fn is_bypassed(&self) -> bool {
        self.mix == 0.0 && self.feedback == 0.0
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Glide the delay time so turning LAG doesn't crackle.
        self.lag_now += (self.lag - self.lag_now) * 0.0005;
        let back = if self.feedback > 0.0 {
            self.feedback * fast_tanh(self.delay.read(self.lag_now))
        } else {
            0.0
        };
        let s = x + back;
        let y = if self.mix > 0.0 {
            let (re, im) = self.hilbert.process(s);
            self.phase += self.hz / self.fs;
            self.phase -= self.phase.floor();
            let shifted = re * sin_turns(self.phase + 0.25) + im * sin_turns(self.phase);
            s + (shifted - s) * self.mix
        } else {
            s
        };
        self.delay.write(y);
        y
    }
}

/// THROUGH: the place the sound is heard from.
#[derive(Clone, Debug)]
pub struct Medium {
    fs: f32,
    kind: u32,
    depth: f32,
    svf: [Svf; 4],
    coeffs: [SvfCoeffs; 4],
    delay: DelayLine,
    lp: f32,
    lfo: [f32; 2],
    follow: f32,
    k_attack: f32,
    k_release: f32,
    crackle: f32,
    whistle: f32,
}

impl Medium {
    pub fn new(fs: f32) -> Self {
        let c = SvfCoeffs::new(1_000.0, 0.7, fs);
        Self {
            fs,
            kind: 0,
            depth: 0.0,
            svf: Default::default(),
            coeffs: [c; 4],
            delay: DelayLine::new((fs * 0.05) as usize),
            lp: 0.0,
            lfo: [0.0, 0.37],
            follow: 0.0,
            k_attack: one_pole_coeff(0.001, fs),
            k_release: one_pole_coeff(0.06, fs),
            crackle: 0.0,
            whistle: 0.0,
        }
    }

    pub fn control(&mut self, kind: u32, depth: f32) {
        if kind != self.kind {
            self.svf.iter_mut().for_each(Svf::reset);
            self.delay.clear();
            self.kind = kind;
        }
        self.depth = depth;
        let fs = self.fs;
        let d = depth;
        let c = |hz: f32, q: f32| SvfCoeffs::new(hz, q, fs);
        self.coeffs = match kind {
            // The can's band, and its own metal ring.
            MEDIUM_TIN_CAN => [
                c(1_100.0, 0.9),
                c(1_520.0, 9.0 + 10.0 * d),
                c(2_630.0, 9.0 + 10.0 * d),
                c(4_080.0, 8.0 + 8.0 * d),
            ],
            MEDIUM_WATER => [
                c(1_400.0 * (1.0 - 0.6 * d), 0.9),
                c(1_000.0, 0.7),
                c(180.0, 2.0),
                c(1_000.0, 0.7),
            ],
            MEDIUM_WALL => [
                c(900.0 * (1.0 - 0.6 * d), 0.8),
                c(1_000.0, 0.7),
                c(68.0, 4.0),
                c(1_000.0, 0.7),
            ],
            MEDIUM_RADIO => [
                c(320.0, 0.7),
                c(3_200.0 - 1_400.0 * d, 0.9),
                c(1_000.0, 0.7),
                c(1_000.0, 0.7),
            ],
            MEDIUM_HORN => [
                c(650.0, 4.0 + 3.0 * d),
                c(1_320.0, 4.0 + 3.0 * d),
                c(2_480.0, 4.0 + 3.0 * d),
                c(240.0, 0.7),
            ],
            _ => self.coeffs,
        };
    }

    /// Envelope of the input, so added noise only comes with the sound.
    #[inline]
    fn follow(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let k = if a > self.follow {
            self.k_attack
        } else {
            self.k_release
        };
        self.follow = flush(self.follow + (a - self.follow) * k);
        self.follow
    }

    #[inline]
    pub fn process(&mut self, x: f32, rng: &mut Noise) -> f32 {
        let d = self.depth;
        let fs = self.fs;
        let wet = match self.kind {
            MEDIUM_TIN_CAN => {
                let band = self.svf[0].process(x, &self.coeffs[0]).1 * 2.0;
                let ring = self.svf[1].process(band, &self.coeffs[1]).1
                    + self.svf[2].process(band, &self.coeffs[2]).1
                    + 0.7 * self.svf[3].process(band, &self.coeffs[3]).1;
                // The string between the cans: a short, metallic comb.
                let string = self.delay.read(0.0019 * fs);
                let y = fast_tanh(1.5 * (band + 0.25 * d * ring)) + (0.35 + 0.3 * d) * string;
                self.delay.write(y * 0.6);
                y * 0.8
            }
            MEDIUM_TUBE => {
                let len = (0.0025 + 0.009 * d) * fs;
                let back = self.delay.read(len);
                self.lp += 0.35 * (back - self.lp);
                let y = x + (0.55 + 0.3 * d) * self.lp;
                self.delay.write(y);
                y * 0.55
            }
            MEDIUM_WATER => {
                let low = self.svf[0].process(x, &self.coeffs[0]).0;
                let boom = self.svf[2].process(low, &self.coeffs[2]).1;
                self.delay.write(low);
                // Two slowly wandering paths: the wobble of light through water.
                self.lfo[0] += 0.7 / fs;
                self.lfo[1] += 1.13 / fs;
                let a = self
                    .delay
                    .read((0.006 + 0.003 * sin_turns(self.lfo[0])) * fs);
                let b = self
                    .delay
                    .read((0.011 + 0.004 * sin_turns(self.lfo[1])) * fs);
                (low + 0.6 * a + 0.4 * b) * 0.8 + boom * 0.6
            }
            MEDIUM_WALL => {
                let low = self.svf[0].process(x, &self.coeffs[0]).0;
                let boom = self.svf[2].process(low, &self.coeffs[2]).1;
                let slap = self.delay.read(0.023 * fs);
                self.delay.write(low);
                (low * 1.4 + boom * (0.8 + 1.2 * d) + 0.35 * slap) * 0.9
            }
            MEDIUM_RADIO => {
                let env = self.follow(x);
                let hp = self.svf[0].process(x, &self.coeffs[0]).2;
                let band = self.svf[1].process(hp, &self.coeffs[1]).0;
                let driven = fast_tanh((1.0 + 3.0 * d) * band) / (1.0 + 1.5 * d).sqrt();
                // Static and crackle ride on the signal.
                let mut static_ = rng.sample() * 0.022 * d;
                if rng.uniform() < 40.0 / fs {
                    self.crackle = rng.sample();
                }
                self.crackle *= 0.9;
                static_ += self.crackle * 0.25 * d;
                self.whistle += 3_700.0 / fs;
                self.whistle -= self.whistle.floor();
                let whistle = 0.04 * d * sin_turns(self.whistle);
                driven + (static_ + whistle) * (env * 3.0).min(1.0).powi(2)
            }
            MEDIUM_HORN => {
                let env = self.follow(x);
                let hp = self.svf[3].process(x, &self.coeffs[3]).2;
                let horn = self.svf[0].process(hp, &self.coeffs[0]).1
                    + self.svf[1].process(hp, &self.coeffs[1]).1
                    + 0.8 * self.svf[2].process(hp, &self.coeffs[2]).1;
                // Wow from an eccentric turntable.
                self.lfo[0] += 0.55 / fs;
                self.delay.write(horn);
                let wow = self
                    .delay
                    .read((0.004 + 0.0012 * d * sin_turns(self.lfo[0])) * fs);
                if rng.uniform() < 25.0 / fs {
                    self.crackle = rng.sample();
                }
                self.crackle *= 0.75;
                fast_tanh(1.2 * wow) + self.crackle * 0.3 * d * (env * 4.0).min(1.0).powi(2)
            }
            _ => return x,
        };
        // DEPTH is also the wet/dry balance, so a little goes a long way.
        let m = (d * 1.6).min(1.0);
        x + (wet - x) * m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::MEDIUM_NAMES;

    #[test]
    fn shifter_moves_a_sine_by_the_shift() {
        let fs = 48_000.0;
        let mut s = Shifter::new(fs);
        // Find the knob position for about +100 Hz.
        let knob = (0..10_000)
            .map(|i| 0.5 + i as f32 / 20_000.0)
            .find(|&k| map::shift_hz(k) >= 100.0)
            .unwrap();
        s.control(knob, 0.0, 0.3);
        let shift = map::shift_hz(knob);
        let out: Vec<f32> = (0..48_000)
            .map(|n| s.process(sin_turns(1_000.0 * n as f32 / fs)))
            .collect();
        let zc = out[4_800..]
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        let measured = zc as f32 / (43_200.0 / fs);
        assert!(
            (measured - 1_000.0 - shift).abs() < 5.0,
            "{measured} vs {}",
            1_000.0 + shift
        );
    }

    #[test]
    fn spiral_feedback_stays_bounded() {
        let fs = 48_000.0;
        let mut s = Shifter::new(fs);
        s.control(0.8, 1.0, 0.2);
        let mut rng = Noise::new(1);
        for n in 0..fs as usize * 5 {
            let x = if n < 4_800 { rng.sample() } else { 0.0 };
            let y = s.process(x);
            assert!(y.is_finite() && y.abs() < 8.0, "{y}");
        }
    }

    #[test]
    fn every_medium_is_bounded_and_silent_on_silence() {
        let fs = 48_000.0;
        for kind in 0..MEDIUM_NAMES.len() as u32 {
            for depth in [0.0, 0.5, 1.0] {
                let mut m = Medium::new(fs);
                m.control(kind, depth);
                let mut rng = Noise::new(2);
                let mut peak = 0.0f32;
                for n in 0..48_000 {
                    let x = if n < 24_000 {
                        0.8 * sin_turns(220.0 * n as f32 / fs)
                    } else {
                        0.0
                    };
                    let y = m.process(x, &mut rng);
                    assert!(y.is_finite());
                    if n < 24_000 {
                        peak = peak.max(y.abs());
                    }
                    if n > 46_000 {
                        assert!(y.abs() < 1e-3, "{kind} {depth}: tail {y}");
                    }
                }
                assert!(peak > 0.01 && peak < 4.0, "{kind} {depth}: {peak}");
            }
        }
    }
}
