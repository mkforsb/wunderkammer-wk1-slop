//! Two-channel engine: runs both voices at 2× the output rate, decimates each
//! one, sends it through its own plate reverb, pans, mixes and applies the
//! output stage. It also carries the conversation between the channels:
//! a hit on one channel may be answered by the other (REPLY), and that
//! answer may be answered in turn.

use crate::params::{Param, VoicePatch, map};
use crate::reverb::Plate;
use crate::util::{DcBlocker, Noise, fast_tanh};
use crate::voice::Voice;

pub const VOICES: usize = 2;
pub const OVERSAMPLE: usize = 2;
pub const DEFAULT_MASTER: f32 = 0.8;
/// Longest chain of replies to one hit (an argument has to end somewhere).
pub const MAX_REPLY_DEPTH: u32 = 16;
/// Replies quieter than this are not worth making.
const MIN_REPLY_VELOCITY: f32 = 0.04;
const MAX_PENDING: usize = 16;

/// A reply waiting to happen.
#[derive(Clone, Copy, Debug)]
struct Reply {
    voice: usize,
    frames_left: u32,
    velocity: f32,
    pitch_oct: f32,
    depth: u32,
}

/// Windowed-sinc lowpass + 2:1 decimator.
#[derive(Clone, Debug)]
struct Decimator {
    taps: [f32; Self::TAPS],
    /// Doubled history so the dot product is always over a contiguous slice.
    hist: [f32; 2 * Self::TAPS],
    pos: usize,
}

impl Decimator {
    const TAPS: usize = 47;

    fn new() -> Self {
        // Cutoff at 0.23 × the oversampled rate (≈ 22 kHz at 96 kHz).
        let fc = 0.23f32;
        let m = (Self::TAPS - 1) as f32 / 2.0;
        let mut taps = [0.0f32; Self::TAPS];
        for (n, t) in taps.iter_mut().enumerate() {
            let x = n as f32 - m;
            let sinc = if x == 0.0 {
                2.0 * fc
            } else {
                (2.0 * core::f32::consts::PI * fc * x).sin() / (core::f32::consts::PI * x)
            };
            let w = n as f32 / (Self::TAPS - 1) as f32 * core::f32::consts::TAU;
            let blackman = 0.42 - 0.5 * w.cos() + 0.08 * (2.0 * w).cos();
            *t = sinc * blackman;
        }
        let sum: f32 = taps.iter().sum();
        taps.iter_mut().for_each(|t| *t /= sum);
        Self {
            taps,
            hist: [0.0; 2 * Self::TAPS],
            pos: 0,
        }
    }

    #[inline]
    fn push(&mut self, x: f32) {
        self.pos = if self.pos == 0 {
            Self::TAPS - 1
        } else {
            self.pos - 1
        };
        self.hist[self.pos] = x;
        self.hist[self.pos + Self::TAPS] = x;
    }

    #[inline]
    fn process(&mut self, block: [f32; OVERSAMPLE]) -> f32 {
        for x in block {
            self.push(x);
        }
        let h = &self.hist[self.pos..self.pos + Self::TAPS];
        h.iter().zip(self.taps.iter()).map(|(a, b)| a * b).sum()
    }
}

#[derive(Clone, Debug)]
pub struct Engine {
    sample_rate: f32,
    voices: [Voice; VOICES],
    master_target: f32,
    master: f32,
    k_master: f32,
    dc_r: f32,
    /// One decimator per voice (voices are mono until panned).
    dec: [Decimator; VOICES],
    reverbs: [Plate; VOICES],
    dc: [DcBlocker; 2],
    pending: [Option<Reply>; MAX_PENDING],
    rng: Noise,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let fs = sample_rate * OVERSAMPLE as f32;
        Self {
            sample_rate,
            voices: [Voice::new(fs, 1), Voice::new(fs, 2)],
            master_target: DEFAULT_MASTER,
            master: DEFAULT_MASTER,
            k_master: 1.0 - (-1.0 / (0.01 * sample_rate)).exp(),
            dc_r: 1.0 - core::f32::consts::TAU * 8.0 / sample_rate,
            dec: [Decimator::new(), Decimator::new()],
            reverbs: [Plate::new(sample_rate), Plate::new(sample_rate)],
            dc: [DcBlocker::default(), DcBlocker::default()],
            pending: [None; MAX_PENDING],
            rng: Noise::new(0x5EED),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn set_param(&mut self, voice: usize, param: Param, value: f32) {
        if let Some(v) = self.voices.get_mut(voice) {
            v.set_param(param, value);
        }
    }

    pub fn param(&self, voice: usize, param: Param) -> f32 {
        self.voices[voice].param(param)
    }

    pub fn load_patch(&mut self, voice: usize, patch: &VoicePatch) {
        if let Some(v) = self.voices.get_mut(voice) {
            for p in crate::params::ALL_PARAMS {
                v.set_param(p, patch[p.index()]);
            }
        }
    }

    /// Jump smoothed parameters straight to their targets.
    pub fn snap_params(&mut self) {
        self.voices.iter_mut().for_each(Voice::snap_params);
        self.master = self.master_target;
    }

    /// Master volume, `0..=1` knob position.
    pub fn set_master(&mut self, value: f32) {
        self.master_target = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            DEFAULT_MASTER
        };
    }

    /// Jump one voice's smoothed parameters to their targets, e.g. so a patch
    /// change sent together with a trigger takes effect exactly on the hit.
    pub fn snap_voice(&mut self, voice: usize) {
        if let Some(v) = self.voices.get_mut(voice) {
            v.snap_params();
        }
    }

    pub fn trigger(&mut self, voice: usize, velocity: f32) {
        if let Some(v) = self.voices.get_mut(voice) {
            v.trigger(velocity);
            self.maybe_reply(voice, velocity, 0.0, 0);
        }
    }

    /// The other channel may answer a hit on `from`, according to its own
    /// REPLY controls.
    fn maybe_reply(&mut self, from: usize, velocity: f32, pitch_oct: f32, depth: u32) {
        let to = (from + 1) % VOICES;
        let v = &self.voices[to];
        let odds = v.param(Param::ReplyOdds);
        if odds <= 0.0 || depth >= MAX_REPLY_DEPTH || self.rng.uniform() >= odds {
            return;
        }
        let (factor, step) = map::reply_mood(v.param(Param::ReplyMood));
        let velocity = (velocity * factor).min(1.0);
        if velocity < MIN_REPLY_VELOCITY {
            return;
        }
        // A little human hesitation.
        let lag =
            map::reply_lag_seconds(v.param(Param::ReplyLag)) * (0.92 + 0.16 * self.rng.uniform());
        let reply = Reply {
            voice: to,
            frames_left: ((lag * self.sample_rate) as u32).max(1),
            velocity,
            pitch_oct: pitch_oct + step,
            depth: depth + 1,
        };
        if let Some(slot) = self.pending.iter_mut().find(|p| p.is_none()) {
            *slot = Some(reply);
        }
    }

    /// Count down pending replies and fire the ones that are due.
    fn run_replies(&mut self) {
        for i in 0..MAX_PENDING {
            let Some(mut r) = self.pending[i] else {
                continue;
            };
            r.frames_left -= 1;
            if r.frames_left == 0 {
                self.pending[i] = None;
                self.voices[r.voice].trigger_with(r.velocity, r.pitch_oct);
                self.maybe_reply(r.voice, r.velocity, r.pitch_oct, r.depth);
            } else {
                self.pending[i] = Some(r);
            }
        }
    }

    /// Replies scheduled and not yet made.
    pub fn pending_replies(&self) -> usize {
        self.pending.iter().flatten().count()
    }

    /// Render stereo audio. `left` and `right` must be the same length.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        debug_assert_eq!(left.len(), right.len());
        let pans: [(f32, f32); VOICES] = core::array::from_fn(|i| {
            // Equal-power pan, normalized so the centre is unity per side.
            let a = self.voices[i].param(Param::Pan) * core::f32::consts::FRAC_PI_2;
            (
                a.cos() * core::f32::consts::SQRT_2,
                a.sin() * core::f32::consts::SQRT_2,
            )
        });
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if self.pending.iter().any(Option::is_some) {
                self.run_replies();
            }
            let (mut yl, mut yr) = (0.0, 0.0);
            for (((v, dec), plate), (gl, gr)) in self
                .voices
                .iter_mut()
                .zip(self.dec.iter_mut())
                .zip(self.reverbs.iter_mut())
                .zip(pans)
            {
                let mut block = [0.0; OVERSAMPLE];
                for s in block.iter_mut() {
                    *s = v.tick();
                }
                let dry = dec.process(block);
                // Post-fader send into the channel's own plate. The plate always
                // runs so the tail keeps ringing when AMOUNT is turned down.
                let controls = plate.controls(
                    v.smoothed(Param::ReverbDecay),
                    v.smoothed(Param::ReverbTone),
                    v.smoothed(Param::ReverbPredelay),
                );
                let (wl, wr) = plate.process(dry, &controls);
                let send = map::reverb_gain(v.smoothed(Param::ReverbMix));
                // Pan acts as a balance control on the stereo wet signal.
                yl += (dry + send * wl) * gl;
                yr += (dry + send * wr) * gr;
            }
            self.master += (self.master_target - self.master) * self.k_master;
            let gain = map::level_gain(self.master) * 1.4;
            *l = soft_clip(self.dc[0].process(yl, self.dc_r) * gain);
            *r = soft_clip(self.dc[1].process(yr, self.dc_r) * gain);
        }
    }

    /// Convenience for offline rendering.
    pub fn render_vec(&mut self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut l = vec![0.0; frames];
        let mut r = vec![0.0; frames];
        self.render(&mut l, &mut r);
        (l, r)
    }
}

/// Output stage: transparent below ~-6 dBFS, gently saturating above.
#[inline]
fn soft_clip(x: f32) -> f32 {
    const KNEE: f32 = 0.5;
    let a = x.abs();
    if a <= KNEE {
        x
    } else {
        let over = (a - KNEE) / (1.0 - KNEE);
        (KNEE + (1.0 - KNEE) * fast_tanh(over)).copysign(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ALL_PARAMS, ENGINE_COUNT, ParamKind};

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn silent_until_triggered() {
        let mut e = Engine::new(48_000.0);
        let (l, r) = e.render_vec(4_800);
        assert!(rms(&l) < 1e-4 && rms(&r) < 1e-4);
    }

    #[test]
    fn every_engine_makes_sound_and_decays() {
        for engine in 0..ENGINE_COUNT {
            let mut e = Engine::new(48_000.0);
            e.set_param(0, Param::Engine, engine as f32);
            e.set_param(0, Param::Time, 0.3);
            e.set_param(0, Param::ReverbMix, 0.0);
            e.snap_params();
            e.trigger(0, 1.0);
            let (l, _) = e.render_vec(24_000);
            let early = rms(&l);
            assert!(early > 0.005, "engine {engine} too quiet: {early}");
            let (l, _) = e.render_vec(48_000 * 8);
            let late = rms(&l[l.len() - 4_800..]);
            assert!(late < early * 0.01, "engine {engine} did not decay: {late}");
        }
    }

    #[test]
    fn output_is_finite_and_bounded_under_random_patches() {
        let mut rng = crate::util::Noise::new(99);
        let mut e = Engine::new(44_100.0);
        for round in 0..80 {
            for v in 0..VOICES {
                for p in ALL_PARAMS {
                    let x = rng.uniform();
                    let val = match p.info().kind {
                        ParamKind::Continuous => x,
                        ParamKind::Choice(n) => (x * n.len() as f32).floor(),
                    };
                    e.set_param(v, p, val);
                }
                e.trigger(v, rng.uniform());
            }
            e.set_master(1.0);
            let (l, r) = e.render_vec(2_205);
            for s in l.iter().chain(r.iter()) {
                assert!(s.is_finite() && s.abs() <= 1.0, "round {round}: {s}");
            }
        }
    }

    #[test]
    fn higher_velocity_is_louder() {
        for engine in 0..ENGINE_COUNT {
            let level = |vel: f32| {
                let mut e = Engine::new(48_000.0);
                e.set_param(0, Param::Engine, engine as f32);
                e.set_param(0, Param::Sense, 0.5);
                e.set_param(0, Param::Time, 0.3);
                e.snap_params();
                e.trigger(0, vel);
                rms(&e.render_vec(24_000).0)
            };
            assert!(level(1.0) > level(0.3) * 1.3, "engine {engine}");
        }
    }

    #[test]
    fn the_other_channel_replies_after_its_lag() {
        let mut e = Engine::new(48_000.0);
        for v in 0..VOICES {
            e.set_param(v, Param::Engine, crate::params::ENGINE_COIN as f32);
            e.set_param(v, Param::Time, 0.0);
            e.set_param(v, Param::ReverbMix, 0.0);
        }
        e.set_param(0, Param::Pan, 0.0);
        e.set_param(1, Param::Pan, 1.0);
        e.set_param(1, Param::ReplyOdds, 1.0);
        e.set_param(1, Param::ReplyLag, 0.5); // 200 ms
        e.snap_params();
        e.trigger(0, 1.0);
        assert_eq!(e.pending_replies(), 1);
        let (_, r) = e.render_vec(48_000);
        let lag = (map::reply_lag_seconds(0.5) * 48_000.0) as usize;
        assert!(rms(&r[..lag * 9 / 10]) < 1e-4, "right spoke too early");
        assert!(
            rms(&r[lag * 11 / 10..lag * 2]) > 1e-3,
            "right never replied"
        );
        assert_eq!(e.pending_replies(), 0);
    }

    #[test]
    fn arguments_end() {
        let mut e = Engine::new(48_000.0);
        for v in 0..VOICES {
            e.set_param(v, Param::ReplyOdds, 1.0);
            e.set_param(v, Param::ReplyLag, 0.0);
            e.set_param(v, Param::ReplyMood, 1.0);
        }
        e.snap_params();
        e.trigger(0, 1.0);
        e.render_vec(48_000 * 2);
        assert_eq!(e.pending_replies(), 0);
    }

    fn hit(configure: impl Fn(&mut Engine), frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut e = Engine::new(48_000.0);
        e.set_param(0, Param::Engine, crate::params::ENGINE_COIN as f32);
        e.set_param(0, Param::MacroZ, 0.0);
        e.set_param(0, Param::Time, 0.25);
        e.set_param(0, Param::ReverbMix, 0.0);
        configure(&mut e);
        e.snap_params();
        e.trigger(0, 1.0);
        e.render_vec(frames)
    }

    #[test]
    fn reverb_at_zero_amount_leaves_the_dry_signal_untouched() {
        let dry = hit(|_| {}, 24_000);
        assert!(rms(&dry.0) > 1e-3);
        let other = hit(
            |e| {
                e.set_param(0, Param::ReverbDecay, 1.0);
                e.set_param(0, Param::ReverbTone, 0.0);
                e.set_param(0, Param::ReverbPredelay, 1.0);
            },
            24_000,
        );
        assert_eq!(dry, other);
    }

    #[test]
    fn reverb_adds_a_tail() {
        let frames = 48_000 * 2;
        let with_reverb = |amount: f32| {
            let (l, r) = hit(
                |e| {
                    e.set_param(0, Param::ReverbMix, amount);
                    e.set_param(0, Param::ReverbDecay, 0.6);
                },
                frames,
            );
            rms(&l[48_000..72_000]) + rms(&r[48_000..72_000])
        };
        let dry = with_reverb(0.0);
        let wet = with_reverb(0.7);
        assert!(dry < 1e-5, "dry tail {dry}");
        assert!(wet > 1e-3, "reverb tail {wet}");
    }

    #[test]
    fn pan_hard_left_with_reverb() {
        let (l, r) = hit(
            |e| {
                e.set_param(0, Param::Pan, 0.0);
                e.set_param(0, Param::ReverbMix, 0.8);
            },
            24_000,
        );
        assert!(rms(&r) < rms(&l) * 0.01);
    }

    #[test]
    fn snap_voice_applies_pending_changes_to_that_voice_only() {
        let mut e = Engine::new(48_000.0);
        e.snap_params();
        for v in 0..VOICES {
            e.set_param(v, Param::Pitch, 0.9);
        }
        e.snap_voice(0);
        assert_eq!(e.voices[0].smoothed(Param::Pitch), 0.9);
        assert_ne!(
            e.voices[1].smoothed(Param::Pitch),
            0.9,
            "voice 2 should still glide"
        );
    }
}
