//! AVIARY: a flock of synthetic creatures, startled by the hit.
//!
//! Each species is drawn from a seed: songbirds (whistles, up and down
//! sweeps, hooks, chips and trills), crickets (a pure carrier chopped into
//! pulse trains) or frogs (a buzzy, harmonic-rich croak). A species' call is a
//! short grammar of one to four syllables, each repeated a few times.
//!
//! The hit startles every creature at once. They all call within a few
//! hundred milliseconds, then calm down over TIME: calls get rarer until the
//! last one falls silent. PITCH sets the register (birds sing several octaves
//! above it, frogs croak near it).
//!
//! X = SPECIES (48 fixed, RND: new species every hit), Y = FLOCK (1–12
//! creatures; bigger flocks mix more species), Z = FRENZY (call rate, wilder
//! sweeps, more trills).

use super::Ctx;
use crate::params::map;
use crate::util::{Noise, lerp, sin_turns};

const MAX_CREATURES: usize = 12;
const MAX_SPECIES: usize = 4;
const MAX_SYLLABLES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Bird,
    Cricket,
    Frog,
}

#[derive(Clone, Copy, Debug)]
struct Syllable {
    seconds: f32,
    /// Start and end frequency, relative to the species' register.
    from: f32,
    to: f32,
    /// Extra excursion in the middle (a hook), relative.
    bend: f32,
    trill_hz: f32,
    trill_depth: f32,
    /// Pulse rate for crickets and frogs (0 = smooth).
    pulse_hz: f32,
    repeats: u32,
    gap: f32,
}

#[derive(Clone, Copy, Debug)]
struct Species {
    kind: Kind,
    register: f32,
    syllables: [Syllable; MAX_SYLLABLES],
    count: usize,
    call_gap: f32,
    harmonics: f32,
    /// Frog croak: the weight of each harmonic (precomputed).
    croak: [f32; 6],
}

#[derive(Clone, Copy, Debug)]
struct Creature {
    species: usize,
    detune: f32,
    singing: bool,
    syllable: usize,
    repeat: u32,
    /// Seconds into the current syllable (or the gap after it).
    t: f32,
    next_call: f32,
    quiet_after: f32,
    phase: f32,
    trill_phase: f32,
    pulse_phase: f32,
    /// The current syllable's sweep end points after FRENZY.
    from: f32,
    to: f32,
}

const QUIET: Syllable = Syllable {
    seconds: 0.1,
    from: 1.0,
    to: 1.0,
    bend: 0.0,
    trill_hz: 0.0,
    trill_depth: 0.0,
    pulse_hz: 0.0,
    repeats: 1,
    gap: 0.05,
};

#[derive(Clone, Debug)]
pub struct Aviary {
    fs: f32,
    species: [Species; MAX_SPECIES],
    creatures: [Creature; MAX_CREATURES],
    count: usize,
    seed: Option<u32>,
    flock: usize,
    frenzy: f32,
    hz: f32,
    t: f32,
    length: f32,
    level: f32,
}

impl Aviary {
    pub fn new(fs: f32) -> Self {
        let quiet = Species {
            kind: Kind::Bird,
            register: 8.0,
            syllables: [QUIET; MAX_SYLLABLES],
            count: 1,
            call_gap: 0.4,
            harmonics: 0.0,
            croak: [0.0; 6],
        };
        let idle = Creature {
            species: 0,
            detune: 1.0,
            singing: false,
            syllable: 0,
            repeat: 0,
            t: 0.0,
            next_call: f32::MAX,
            quiet_after: 0.0,
            phase: 0.0,
            trill_phase: 0.0,
            pulse_phase: 0.0,
            from: 1.0,
            to: 1.0,
        };
        Self {
            fs,
            species: [quiet; MAX_SPECIES],
            creatures: [idle; MAX_CREATURES],
            count: 1,
            seed: Some(0),
            flock: 1,
            frenzy: 0.3,
            hz: 200.0,
            t: 0.0,
            length: 1.0,
            level: 1.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.seed = map::seed(c.x);
        self.flock = map::flock_size(c.y);
        self.frenzy = c.z;
        self.hz = c.hz;
    }

    fn breed(rng: &mut Noise) -> Species {
        let mut r = |lo: f32, hi: f32| lo + (hi - lo) * rng.uniform();
        let roll = r(0.0, 1.0);
        let kind = if roll < 0.7 {
            Kind::Bird
        } else if roll < 0.85 {
            Kind::Cricket
        } else {
            Kind::Frog
        };
        let mut syllables = [QUIET; MAX_SYLLABLES];
        let (count, register, call_gap, harmonics) = match kind {
            Kind::Bird => {
                let count = 1 + (r(0.0, 3.99) as usize);
                for s in &mut syllables[..count] {
                    let style = r(0.0, 6.0) as u32;
                    let (from, to, bend) = match style {
                        0 => (r(0.7, 1.0), r(1.2, 1.9), 0.0),
                        1 => (r(1.3, 1.9), r(0.55, 0.9), 0.0),
                        2 => {
                            let f = r(0.85, 1.2);
                            (f, f * r(0.95, 1.05), 0.0)
                        }
                        3 => (r(0.8, 1.0), r(0.8, 1.0), r(0.3, 0.7)),
                        4 => (r(1.5, 2.2), r(0.6, 0.8), 0.0),
                        _ => (r(0.9, 1.1), r(1.0, 1.3), -r(0.1, 0.3)),
                    };
                    let chip = style == 4;
                    *s = Syllable {
                        seconds: if chip { r(0.015, 0.035) } else { r(0.04, 0.2) },
                        from,
                        to,
                        bend,
                        trill_hz: if r(0.0, 1.0) < 0.3 {
                            r(25.0, 70.0)
                        } else {
                            0.0
                        },
                        trill_depth: r(0.05, 0.22),
                        pulse_hz: 0.0,
                        repeats: 1 + (r(0.0, 3.5) as u32),
                        gap: r(0.01, 0.07),
                    };
                }
                (count, r(5.0, 14.0), r(0.15, 0.8), r(0.0, 0.15))
            }
            Kind::Cricket => {
                syllables[0] = Syllable {
                    seconds: r(0.06, 0.2),
                    from: 1.0,
                    to: r(0.98, 1.02),
                    bend: 0.0,
                    trill_hz: 0.0,
                    trill_depth: 0.0,
                    pulse_hz: r(25.0, 48.0),
                    repeats: 1 + (r(0.0, 2.5) as u32),
                    gap: r(0.04, 0.12),
                };
                (1, r(16.0, 26.0), r(0.2, 0.5), 0.02)
            }
            Kind::Frog => {
                syllables[0] = Syllable {
                    seconds: r(0.1, 0.32),
                    from: r(1.0, 1.15),
                    to: r(0.8, 1.0),
                    bend: 0.0,
                    trill_hz: 0.0,
                    trill_depth: 0.0,
                    pulse_hz: r(22.0, 60.0),
                    repeats: 1 + (r(0.0, 2.0) as u32),
                    gap: r(0.05, 0.15),
                };
                (1, r(0.8, 2.0), r(0.3, 0.9), r(0.6, 0.9))
            }
        };
        let croak = core::array::from_fn(|k| {
            let k = k as f32 + 1.0;
            harmonics.powf(k - 1.0) / (1.0 + 0.2 * k) * 0.6
        });
        Species {
            kind,
            register,
            syllables,
            count,
            call_gap,
            harmonics,
            croak,
        }
    }

    pub fn trigger(&mut self, c: &Ctx, rng: &mut Noise) {
        let mut breeder = match self.seed {
            Some(s) => Noise::new(0xA71A_0000 + s),
            None => Noise::new(rng.next_u32()),
        };
        let kinds = (1 + (self.flock - 1) / 3).min(MAX_SPECIES);
        for s in &mut self.species[..kinds] {
            *s = Self::breed(&mut breeder);
        }
        self.count = self.flock;
        self.t = 0.0;
        self.length = c.time_s.max(0.2);
        self.level = 0.3 + 0.7 * c.velocity;
        let startle = 0.3 * (1.0 - 0.7 * self.frenzy);
        for (i, cr) in self.creatures[..self.count].iter_mut().enumerate() {
            cr.species = i % kinds;
            cr.detune = (0.25 * (breeder.uniform() - 0.5)).exp2();
            cr.singing = false;
            cr.next_call = startle * rng.uniform();
            cr.quiet_after = self.length * (0.5 + 0.7 * rng.uniform());
        }
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        self.t += dt;
        let nyquist = 0.42 * self.fs;
        let calm = 1.0 + 2.0 * (self.t / self.length);
        let mut out = 0.0;
        for cr in &mut self.creatures[..self.count] {
            let sp = &self.species[cr.species];
            // Frenzy exaggerates every sweep.
            let wild = 1.0 + self.frenzy;
            let begin = |cr: &mut Creature, syl: &Syllable| {
                cr.from = syl.from.powf(wild);
                cr.to = syl.to.powf(wild);
            };
            if !cr.singing {
                if self.t >= cr.next_call && self.t < cr.quiet_after {
                    cr.singing = true;
                    cr.syllable = 0;
                    cr.repeat = 0;
                    cr.t = 0.0;
                    begin(cr, &sp.syllables[0]);
                }
                continue;
            }
            let syl = &sp.syllables[cr.syllable];
            cr.t += dt;
            if cr.t >= syl.seconds + syl.gap {
                cr.t = 0.0;
                cr.repeat += 1;
                if cr.repeat >= syl.repeats {
                    cr.repeat = 0;
                    cr.syllable += 1;
                    if cr.syllable >= sp.count {
                        cr.singing = false;
                        let jitter = 0.7 + 0.6 * rng.uniform();
                        cr.next_call =
                            self.t + sp.call_gap * calm * jitter / (1.0 + 3.0 * self.frenzy);
                    } else {
                        begin(cr, &sp.syllables[cr.syllable]);
                    }
                }
                continue;
            }
            if cr.t >= syl.seconds {
                continue;
            }
            let tau = cr.t / syl.seconds;
            let (from, to) = (cr.from, cr.to);
            cr.trill_phase += syl.trill_hz * (1.0 + self.frenzy) * dt;
            let trill = 1.0 + syl.trill_depth * sin_turns(cr.trill_phase);
            let ratio = (lerp(from, to, tau) + syl.bend * wild * sin_turns(0.5 * tau)) * trill;
            let hz = (self.hz * sp.register * cr.detune * ratio).min(nyquist);
            cr.phase += hz * dt;
            if cr.phase >= 1.0 {
                cr.phase -= 1.0;
            }
            let env = sin_turns(0.5 * tau);
            let mut env = env * env;
            let p = cr.phase;
            let voice = match sp.kind {
                Kind::Bird | Kind::Cricket => {
                    let h = sp.harmonics;
                    if h < 0.03 {
                        sin_turns(p)
                    } else {
                        sin_turns(p) + h * sin_turns(2.0 * p) + 0.5 * h * h * sin_turns(3.0 * p)
                    }
                }
                Kind::Frog => {
                    // A buzzy croak: a band of harmonics.
                    sp.croak
                        .iter()
                        .enumerate()
                        .map(|(k, w)| w * sin_turns((k + 1) as f32 * p))
                        .sum::<f32>()
                }
            };
            if syl.pulse_hz > 0.0 {
                cr.pulse_phase += syl.pulse_hz * dt;
                if cr.pulse_phase >= 1.0 {
                    cr.pulse_phase -= 1.0;
                }
                let pulse = sin_turns(0.5 * cr.pulse_phase).max(0.0);
                env *= pulse * pulse * pulse;
            }
            out += voice * env;
        }
        out * self.level * 0.4 / (self.count as f32).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flock_calls_then_calms_down() {
        let fs = 48_000.0;
        let mut a = Aviary::new(fs);
        let c = Ctx {
            hz: 250.0,
            x: 0.2,
            y: 0.6,
            z: 0.5,
            time_s: 1.5,
            ..Ctx::new(fs)
        };
        a.control(&c);
        let mut rng = Noise::new(4);
        a.trigger(&c, &mut rng);
        let out: Vec<f32> = (0..fs as usize * 4).map(|_| a.tick(&c, &mut rng)).collect();
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        assert!(out.iter().all(|x| x.is_finite() && x.abs() < 4.0));
        assert!(rms(&out[..48_000]) > 0.01, "{}", rms(&out[..48_000]));
        assert!(rms(&out[out.len() - 9_600..]) < 1e-4);
    }

    #[test]
    fn every_seed_breeds_something_singable() {
        for seed in 0..200 {
            let s = Aviary::breed(&mut Noise::new(seed));
            assert!(s.count >= 1 && s.count <= MAX_SYLLABLES);
            assert!(s.register > 0.5);
            for syl in &s.syllables[..s.count] {
                assert!(syl.seconds > 0.01 && syl.repeats >= 1);
            }
        }
    }
}
