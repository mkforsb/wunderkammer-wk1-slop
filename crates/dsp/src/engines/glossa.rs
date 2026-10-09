//! GLOSSA: a tiny vocal tract that mutters a nonsense word on every hit.
//!
//! The tract is a Kelly–Lochbaum waveguide: a chain of short cylindrical
//! sections (one sample of delay each way, so the section count sets the
//! tract length), with scattering at every change of cross-section, a
//! partly reflecting glottis and an open, lowpass-reflecting mouth.
//!
//! Its shape comes from a three-parameter articulatory model (after Stevens
//! and House): where the tongue constricts the tube, how tightly, and how far
//! the lips are open. Vowels are points in that space and consonants are
//! closures in it, so a "word" is just a path through it: each hit draws one
//! (1–3 syllables, glides, diphthongs and stops) and an intonation (a
//! statement, a question, surprise, sing-song), and the tract moves along
//! it while a source plays into the glottis.
//!
//! X = WORD (48 fixed words, RND: a new one every hit), Y = SOURCE (a mouth
//! pop on each syllable → voice → jaw harp → whisper), Z = MOUTH (tract
//! length, 7 cm chipmunk … 34 cm ogre).

use super::Ctx;
use crate::params::map;
use crate::util::{Noise, lerp, sin_turns, smoothstep};

const SPEED_OF_SOUND: f32 = 343.0;
const NEUTRAL: f32 = 3.0;
const MIN_AREA: f32 = 0.004;
const MAX_TARGETS: usize = 10;
const GLOTTAL_REFLECTION: f32 = 0.75;
const LIP_REFLECTION: f32 = -0.85;

/// Articulation: constriction place (0 glottis … 1 lips), constriction
/// area and lip area.
#[derive(Clone, Copy, Debug)]
struct Shape {
    place: f32,
    tongue: f32,
    lips: f32,
}

#[derive(Clone, Copy, Debug)]
struct Target {
    shape: Shape,
    /// Relative duration.
    weight: f32,
    /// A closure: the release that follows it pops.
    closure: bool,
}

const fn s(place: f32, tongue: f32, lips: f32) -> Shape {
    Shape {
        place,
        tongue,
        lips,
    }
}

/// i e a o u ə æ ɨ
const VOWELS: [Shape; 8] = [
    s(0.74, 0.18, 2.4),
    s(0.68, 0.7, 2.2),
    s(0.28, 0.45, 4.5),
    s(0.38, 0.7, 0.9),
    s(0.62, 0.35, 0.22),
    s(0.5, 2.8, 2.0),
    s(0.36, 1.1, 3.8),
    s(0.6, 0.5, 1.4),
];
/// b d g (closures), w y (glides)
const CONSONANTS: [(Shape, bool); 5] = [
    (s(0.5, 2.5, 0.0), true),
    (s(0.88, 0.0, 2.0), true),
    (s(0.64, 0.0, 2.0), true),
    (s(0.62, 0.3, 0.12), false),
    (s(0.76, 0.12, 2.4), false),
];

#[derive(Clone, Debug)]
pub struct Glossa {
    fs: f32,
    n: usize,
    r: Vec<f32>,
    l: Vec<f32>,
    jr: Vec<f32>,
    jl: Vec<f32>,
    k: Vec<f32>,
    targets: [Target; MAX_TARGETS],
    count: usize,
    contour: u32,
    /// Seconds into the word, its length, and whether it is being spoken.
    t: f32,
    length: f32,
    speaking: bool,
    /// The harp keeps ringing after the word ends.
    harp_env: f32,
    harp_mul: f32,
    phase: f32,
    hz: f32,
    source: f32,
    velocity: f32,
    /// Remaining samples of a pop burst and its amplitude.
    pop_left: u32,
    pop_len: u32,
    pop_amp: f32,
    segment: usize,
    breath: f32,
    lip_lp: f32,
    out_prev: f32,
    /// Softens the radiated highs (cheeks and lips are not hard-walled).
    out_lp: f32,
    word_seed: Option<u32>,
    mouth: f32,
}

impl Glossa {
    pub fn new(fs: f32) -> Self {
        let max = (0.36 * fs / SPEED_OF_SOUND).ceil() as usize + 2;
        let neutral = Target {
            shape: s(0.5, NEUTRAL, NEUTRAL),
            weight: 1.0,
            closure: false,
        };
        Self {
            fs,
            n: 8,
            r: vec![0.0; max],
            l: vec![0.0; max + 1],
            jr: vec![0.0; max + 1],
            jl: vec![0.0; max + 1],
            k: vec![0.0; max],
            targets: [neutral; MAX_TARGETS],
            count: 1,
            contour: 0,
            t: 0.0,
            length: 1.0,
            speaking: false,
            harp_env: 0.0,
            harp_mul: 1.0,
            phase: 0.0,
            hz: 120.0,
            source: 0.33,
            velocity: 1.0,
            pop_left: 0,
            pop_len: 1,
            pop_amp: 0.0,
            segment: 0,
            breath: 0.0,
            lip_lp: 0.0,
            out_prev: 0.0,
            out_lp: 0.0,
            word_seed: Some(0),
            mouth: 0.4,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.hz = c.hz;
        self.source = c.y;
        self.word_seed = map::seed(c.x);
        self.mouth = c.z;
        self.shape_tract(self.articulation());
    }

    fn sections_for(&self, mouth: f32) -> usize {
        let len_m = map::tract_cm(mouth) / 100.0;
        ((len_m * self.fs / SPEED_OF_SOUND).round() as usize).clamp(4, self.r.len())
    }

    /// Draw a word: syllables of an optional consonant and a vowel (or two).
    fn compose(&mut self, rng: &mut Noise) {
        let mut pick = |n: usize| (rng.next_u32() as usize >> 4) % n;
        let syllables = 1 + pick(3);
        let mut count = 0;
        let mut push = |t: Target, count: &mut usize| {
            if *count < MAX_TARGETS {
                self.targets[*count] = t;
                *count += 1;
            }
        };
        for _ in 0..syllables {
            if pick(10) < 6 {
                let (shape, closure) = CONSONANTS[pick(CONSONANTS.len())];
                let weight = if closure { 0.35 } else { 0.6 };
                push(
                    Target {
                        shape,
                        weight,
                        closure,
                    },
                    &mut count,
                );
            }
            push(
                Target {
                    shape: VOWELS[pick(VOWELS.len())],
                    weight: 1.5 + 0.25 * pick(5) as f32,
                    closure: false,
                },
                &mut count,
            );
            if pick(10) < 3 {
                push(
                    Target {
                        shape: VOWELS[pick(VOWELS.len())],
                        weight: 1.0,
                        closure: false,
                    },
                    &mut count,
                );
            }
        }
        self.count = count;
        self.contour = pick(4) as u32;
        let total: f32 = self.targets[..count].iter().map(|t| t.weight).sum();
        for t in &mut self.targets[..count] {
            t.weight /= total;
        }
    }

    /// Where the word is now: (segment index, shape).
    fn articulation(&self) -> Shape {
        let s = (self.t / self.length).clamp(0.0, 1.0);
        let mut start = 0.0;
        for i in 0..self.count {
            let w = self.targets[i].weight;
            if s < start + w || i + 1 == self.count {
                let a = self.targets[i].shape;
                let Some(next) = self.targets.get(i + 1).filter(|_| i + 1 < self.count) else {
                    return a;
                };
                // Hold, then glide into the next target. Closures release fast.
                let hold = if self.targets[i].closure { 0.7 } else { 0.45 };
                let f = smoothstep(((s - start) / w - hold) / (1.0 - hold));
                let b = next.shape;
                return Shape {
                    place: lerp(a.place, b.place, f),
                    tongue: lerp(a.tongue, b.tongue, f),
                    lips: lerp(a.lips, b.lips, f),
                };
            }
            start += w;
        }
        self.targets[0].shape
    }

    fn segment_index(&self) -> usize {
        let s = (self.t / self.length).clamp(0.0, 1.0);
        let mut start = 0.0;
        for i in 0..self.count {
            start += self.targets[i].weight;
            if s < start {
                return i;
            }
        }
        self.count - 1
    }

    fn shape_tract(&mut self, a: Shape) {
        let n = self.n;
        let mut prev = 0.0;
        for j in 0..n {
            let x = (j as f32 + 0.5) / n as f32;
            let d = (x - a.place) / 0.1;
            let mut area = NEUTRAL - (NEUTRAL - a.tongue) * (-d * d).exp();
            if x < 0.12 {
                area = lerp(0.8, area, smoothstep(x / 0.12));
            }
            if x > 0.88 {
                area = lerp(area, a.lips, smoothstep((x - 0.88) / 0.12));
            }
            let area = area.max(MIN_AREA);
            if j > 0 {
                self.k[j] = (prev - area) / (prev + area);
            }
            prev = area;
        }
    }

    pub fn trigger(&mut self, c: &Ctx, rng: &mut Noise) {
        let mut word_rng = match self.word_seed {
            Some(seed) => Noise::new(0x6105_5A00 + seed),
            None => Noise::new(rng.next_u32()),
        };
        self.compose(&mut word_rng);
        let n = self.sections_for(self.mouth);
        if n != self.n {
            self.n = n;
            self.r.iter_mut().for_each(|x| *x = 0.0);
            self.l.iter_mut().for_each(|x| *x = 0.0);
        }
        self.t = 0.0;
        self.length = c.time_s.clamp(0.12, 4.0);
        self.speaking = true;
        self.velocity = c.velocity;
        self.segment = 0;
        self.harp_env = c.velocity;
        self.harp_mul = crate::util::t60_gain(self.length * 1.6 + 0.3, self.fs);
        self.pop(1.0);
        self.shape_tract(self.articulation());
    }

    fn pop(&mut self, amp: f32) {
        self.pop_len = ((0.0012 * self.fs) as u32).max(2);
        self.pop_left = self.pop_len;
        self.pop_amp = amp * self.velocity;
    }

    /// Intonation in octaves at word position `s`.
    fn contour(&self, s: f32) -> f32 {
        match self.contour {
            0 => 0.1 - 0.35 * s,
            1 => -0.05 + 0.45 * s * s,
            2 => 0.4 * sin_turns(0.5 * s) - 0.1,
            _ => 0.15 * sin_turns(1.5 * s),
        }
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        let (i, f) = map::segment(self.source, 4);
        let mut w = [0.0f32; 4];
        w[i] = 1.0 - f;
        w[i + 1] = f;

        let mut voice_env = 0.0;
        if self.speaking {
            self.t += dt;
            let s = self.t / self.length;
            if s >= 1.0 {
                self.speaking = false;
            } else {
                voice_env = (self.t / 0.015).min(1.0) * ((1.0 - s) * self.length / 0.05).min(1.0);
                let seg = self.segment_index();
                if seg != self.segment {
                    // Releasing a closure, or any new syllable for the pop source.
                    if self.targets[self.segment].closure || w[0] > 0.0 {
                        self.pop(if self.targets[self.segment].closure {
                            0.8
                        } else {
                            0.5
                        });
                    }
                    self.segment = seg;
                }
            }
        }
        let s = (self.t / self.length).min(1.0);
        let wobble = 0.012 * sin_turns(5.5 * self.t) + 0.004 * rng.sample();
        let speak_hz = self.hz * (self.contour(s) + wobble).exp2();
        // The harp's reed never changes pitch; only the mouth moves.
        let f0 = lerp(speak_hz, self.hz, w[2]);
        self.phase += f0 * dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let p = self.phase;
        // Rosenberg glottal flow.
        let flow = if p < 0.4 {
            0.5 * (1.0 - sin_turns(p / 0.8 + 0.25))
        } else if p < 0.56 {
            sin_turns((p - 0.4) / 0.64 + 0.25)
        } else {
            0.0
        };
        let noise = rng.sample();
        let voice = voice_env * (flow + 0.03 * flow * noise);
        // Jaw harp: a stiff steel reed, a narrow pulse every cycle.
        self.harp_env *= self.harp_mul;
        let reed = if p < 0.06 {
            0.5 - 0.5 * sin_turns(p / 0.06 + 0.25)
        } else {
            0.0
        };
        // The reed's pulses carry more energy the faster they come.
        let harp = self.harp_env * (reed - 0.06 * 0.5) * (150.0 / f0).sqrt().min(2.0);
        self.breath += 0.35 * (noise - self.breath);
        let whisper = voice_env * self.breath * 0.8;
        let mut pop = 0.0;
        if self.pop_left > 0 {
            let x = 1.0 - self.pop_left as f32 / self.pop_len as f32;
            pop = self.pop_amp * (0.5 - 0.5 * sin_turns(x + 0.25)) * 9.0;
            self.pop_left -= 1;
        }
        let input = w[0] * pop
            + w[1] * (6.0 * voice * self.velocity + 0.08 * pop)
            + w[2] * harp * 7.0
            + w[3] * (1.5 * whisper * self.velocity + 0.05 * pop);

        // Kelly–Lochbaum scattering (after Neil Thapen's Pink Trombone).
        let n = self.n;
        self.jr[0] = self.l[0] * GLOTTAL_REFLECTION + input;
        self.lip_lp += 0.4 * (self.r[n - 1] - self.lip_lp);
        self.jl[n] = self.lip_lp * LIP_REFLECTION + (self.r[n - 1] - self.lip_lp) * -0.3;
        for j in 1..n {
            let w = self.k[j] * (self.r[j - 1] + self.l[j]);
            self.jr[j] = self.r[j - 1] - w;
            self.jl[j] = self.l[j] + w;
        }
        for j in 0..n {
            self.r[j] = crate::util::flush(self.jr[j] * 0.999);
            self.l[j] = crate::util::flush(self.jl[j + 1] * 0.999);
        }
        let lips = self.r[n - 1];
        // Radiation from the mouth favours the highs.
        let out = lips - 0.85 * self.out_prev;
        self.out_prev = lips;
        self.out_lp += (out - self.out_lp) * (0.5 * 48_000.0 / self.fs).min(1.0);
        self.out_lp * 1.2
    }

    /// Render with the articulators updated at control rate, as a voice would.
    #[cfg(test)]
    fn speak(&mut self, c: &Ctx, rng: &mut Noise, samples: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(samples);
        for i in 0..samples {
            if i % 16 == 0 {
                self.control(c);
            }
            out.push(self.tick(c, rng));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_spoken_and_differ() {
        let fs = 48_000.0;
        let mut words = Vec::new();
        for seed in [0.0, 0.3, 0.6] {
            let mut g = Glossa::new(fs);
            let c = Ctx {
                hz: 140.0,
                x: seed,
                y: 0.33,
                z: 0.45,
                time_s: 0.6,
                ..Ctx::new(fs)
            };
            g.control(&c);
            let mut rng = Noise::new(5);
            g.trigger(&c, &mut rng);
            let out = g.speak(&c, &mut rng, 36_000);
            assert!(out.iter().all(|x| x.is_finite() && x.abs() < 8.0));
            let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
            assert!(rms > 0.005, "seed {seed}: {rms}");
            words.push(out);
        }
        assert_ne!(words[0], words[1]);
        assert_ne!(words[1], words[2]);
    }

    #[test]
    fn mouth_size_sets_the_section_count() {
        let fs = 96_000.0;
        let g = Glossa::new(fs);
        let small = g.sections_for(0.0);
        let big = g.sections_for(1.0);
        assert!(small < 25 && big > 85, "{small} {big}");
    }
}
