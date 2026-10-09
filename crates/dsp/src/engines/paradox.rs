//! PARADOX: auditory illusions as a percussion voice.
//!
//! *Shepard–Risset glissando.* Ten partials spaced by a fixed ratio glide
//! together under a fixed bell-shaped spectral window. As each one leaves the
//! top of the window, too quiet to hear, it reappears at the bottom. The
//! ear follows the glide and never hears the wrap, so the hit falls (or rises)
//! forever without getting any lower. The glide runs fastest at the hit and
//! slows as the sound decays.
//!
//! *Risset rhythm.* PULSE chops the tone with five layers of beats at
//! doubling tempos, again under a window, crossfading upward, so the beat
//! accelerates forever without ever getting faster. The layers share one
//! master phase so every beat of a slow layer lands on a beat of the faster
//! ones.
//!
//! Spacing by something other than an octave (a fifth, the golden ratio, a
//! tritave) bends the illusion into stranger, bell-like shapes.
//!
//! X = GLIDE (bipolar: endless fall … hover … endless rise), Y = PULSE
//! (Risset rhythm depth), Z = SPACING (a fifth … octave … tritave).

use super::Ctx;
use crate::params::map;
use crate::util::{Noise, sin_turns, t60_gain};

const PARTIALS: usize = 10;
const LAYERS: usize = 5;
/// Slowest pulse layer, beats per second.
const PULSE_HZ: f32 = 1.5;
/// Fraction of an octave the beat climbs per second at full glide.
const TEMPO_DRIFT: f32 = 0.35;

#[derive(Clone, Debug)]
pub struct Paradox {
    fs: f32,
    /// Position of the partial comb in spacing steps, `0..1`.
    offset: f32,
    phases: [f32; PARTIALS],
    /// Per-partial phase increments and window weights, refreshed every
    /// few samples (the comb moves slowly).
    incs: [f32; PARTIALS],
    weights: [f32; PARTIALS],
    refresh: u32,
    glide: f32,
    pulse: f32,
    ratio: f32,
    hz: f32,
    env: f32,
    env_mul: f32,
    peak: f32,
    /// Risset rhythm: master beat phase and tempo position (`0..1` octaves).
    beat: f32,
    tempo: f32,
}

impl Paradox {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            offset: 0.0,
            phases: [0.0; PARTIALS],
            incs: [0.0; PARTIALS],
            weights: [0.0; PARTIALS],
            refresh: 0,
            glide: 0.0,
            pulse: 0.0,
            ratio: 2.0,
            hz: 200.0,
            env: 0.0,
            env_mul: 0.0,
            peak: 1.0,
            beat: 0.0,
            tempo: 0.0,
        }
    }

    pub fn reset(&mut self) {
        let fs = self.fs;
        *self = Self::new(fs);
    }

    pub fn control(&mut self, c: &Ctx) {
        self.glide = map::glide_rate(c.x);
        self.pulse = c.y;
        self.ratio = map::paradox_ratio(c.z);
        self.hz = c.hz;
    }

    pub fn trigger(&mut self, c: &Ctx, _rng: &mut Noise) {
        self.peak = 0.3 + 0.7 * c.velocity;
        self.env = self.env.max(self.peak);
        self.env_mul = t60_gain(c.time_s.max(0.1), self.fs);
    }

    /// Bell window over the comb, zero at both ends so the wrap is silent.
    #[inline]
    fn window(p: f32) -> f32 {
        0.5 - 0.5 * sin_turns(p / PARTIALS as f32 + 0.25)
    }

    #[inline]
    pub fn tick(&mut self, _c: &Ctx, _rng: &mut Noise) -> f32 {
        let dt = 1.0 / self.fs;
        if self.env <= 0.0 {
            return 0.0;
        }
        self.env *= self.env_mul;
        if self.env < 1e-6 {
            self.env = 0.0;
        }
        let drive = 0.3 + 0.7 * (self.env / self.peak).min(1.0);
        self.offset += self.glide * drive * dt;
        // On a wrap every partial takes over its neighbour's slot, keeping
        // its phase; the one that wraps around is silent at the window edge.
        let mut wrapped = true;
        if self.offset >= 1.0 {
            self.offset -= 1.0;
            self.phases.rotate_right(1);
        } else if self.offset < 0.0 {
            self.offset += 1.0;
            self.phases.rotate_left(1);
        } else {
            wrapped = false;
        }
        if wrapped || self.refresh == 0 {
            self.refresh = 16;
            let nyquist = 0.45 * self.fs;
            let log_ratio = self.ratio.log2();
            let mut norm = 0.0;
            for i in 0..PARTIALS {
                let p = i as f32 + self.offset;
                let hz = self.hz * ((p - 0.5 * PARTIALS as f32) * log_ratio).exp2();
                let w = Self::window(p);
                norm += w;
                self.incs[i] = hz * dt;
                self.weights[i] = if hz < nyquist { w } else { 0.0 };
            }
            let norm = 1.0 / norm.max(1e-3);
            self.weights.iter_mut().for_each(|w| *w *= norm);
        }
        self.refresh -= 1;
        let mut tone = 0.0;
        for i in 0..PARTIALS {
            let phase = &mut self.phases[i];
            *phase += self.incs[i];
            *phase -= phase.floor();
            tone += self.weights[i] * sin_turns(*phase);
        }

        let mut amp = 1.0;
        if self.pulse > 0.0 {
            // The beat accelerates in the direction the pitch glides away
            // from (a falling tone gets an ever-faster beat).
            self.tempo += TEMPO_DRIFT * self.glide.abs().max(0.3) * drive * dt;
            let master = PULSE_HZ * self.tempo.exp2();
            self.beat += master * dt;
            if self.tempo >= 1.0 {
                // Every layer now plays what the next one up was playing.
                self.tempo -= 1.0;
                self.beat *= 2.0;
            }
            self.beat -= self.beat.floor();
            let mut sum = 0.0;
            let mut wsum = 0.0;
            for j in 0..LAYERS {
                let pos = (j as f32 + self.tempo) / LAYERS as f32;
                let w = 0.5 - 0.5 * sin_turns(pos + 0.25);
                let ph = self.beat * (1u32 << j) as f32;
                let ph = ph - ph.floor();
                sum += w * (-10.0 * ph).exp();
                wsum += w;
            }
            let rhythm = sum / wsum.max(1e-3);
            amp = 1.0 - self.pulse + self.pulse * rhythm * 1.6;
        }
        tone * amp * self.env * 0.8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wrap_is_silent_and_the_tone_is_steady() {
        // Spectral centroid of the window stays put while the comb glides.
        let fs = 48_000.0;
        let mut p = Paradox::new(fs);
        let c = Ctx {
            hz: 300.0,
            x: 0.0,
            y: 0.0,
            z: 0.415,
            time_s: 10.0,
            ..Ctx::new(fs)
        };
        p.control(&c);
        p.trigger(&c, &mut Noise::new(1));
        let mut rng = Noise::new(1);
        let mut last = 0.0;
        let mut max_jump = 0.0f32;
        for _ in 0..fs as usize * 3 {
            let x = p.tick(&c, &mut rng);
            max_jump = max_jump.max((x - last).abs());
            last = x;
        }
        assert!(max_jump < 0.2, "{max_jump}");
        assert!(p.offset >= 0.0 && p.offset < 1.0);
    }

    #[test]
    fn pulse_stays_bounded_through_tempo_wraps() {
        let fs = 16_000.0;
        let mut p = Paradox::new(fs);
        let c = Ctx {
            x: 1.0,
            y: 1.0,
            time_s: 10.0,
            ..Ctx::new(fs)
        };
        p.control(&c);
        p.trigger(&c, &mut Noise::new(1));
        let mut rng = Noise::new(1);
        for _ in 0..fs as usize * 10 {
            let x = p.tick(&c, &mut rng);
            assert!(x.is_finite() && x.abs() < 2.0);
        }
    }
}
