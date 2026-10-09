//! One Wunderkammer channel.
//!
//! ```text
//!  PITCH ─┬─ BEND ─┬─ WHIM ─┬─ reply offset ─► pitch, X, Y, Z ─┐
//!         │        │        │                                 ▼
//!  TRIGGER ─────────────────────────────────────────► OBJECT (1 of 12) ─► SHIFT ─► TONE ─► THROUGH ─► OUTPUT ─► out
//! ```
//!
//! Controls are evaluated once per [`CONTROL_BLOCK`] samples (parameter
//! smoothing, modulation, object coefficients). The objects and the audio
//! path run every sample.

use crate::engines::{Ctx, Engines};
use crate::fx::{Medium, Shifter};
use crate::params::{FILTER_BP, FILTER_HP, PARAM_COUNT, Param, VoicePatch, default_patch, map};
use crate::util::{DcBlocker, Noise, Svf, SvfCoeffs, one_pole_coeff};

/// Samples per control update.
pub const CONTROL_BLOCK: usize = 16;
/// Parameter de-zippering.
const SMOOTH_TAU_S: f32 = 0.004;
/// WHIM range at full setting.
const WHIM_PITCH_OCT: f32 = 1.0;
const WHIM_MACRO: f32 = 0.35;
/// Most a chain of replies can drift in pitch.
const REPLY_MAX_OCT: f32 = 2.0;

/// Which parameters are de-zippered (the continuous ones).
const SMOOTHED: [bool; PARAM_COUNT] = {
    let mut s = [true; PARAM_COUNT];
    s[Param::Engine as usize] = false;
    s[Param::FilterType as usize] = false;
    s[Param::Medium as usize] = false;
    s
};

#[derive(Clone, Debug)]
pub struct Voice {
    fs: f32,
    target: VoicePatch,
    smooth: VoicePatch,
    k_smooth: f32,

    noise: Noise,
    engines: Engines,
    engine: u32,
    ctx: Ctx,
    block_pos: usize,

    velocity: f32,
    /// Bend envelope, 1 → 0.
    bend: f32,
    bend_mul: f32,
    /// Per-hit offsets: pitch (octaves), X, Y, Z.
    whim: [f32; 4],
    /// Pitch offset of a reply, in octaves.
    reply_oct: f32,

    dc: DcBlocker,
    dc_r: f32,
    shifter: Shifter,
    svf: Svf,
    svf_coeffs: SvfCoeffs,
    filter_type: u32,
    medium: Medium,
    level: f32,
    trim: f32,
}

impl Voice {
    /// `fs` is the rate `tick` is called at (the oversampled rate).
    pub fn new(fs: f32, seed: u32) -> Self {
        let patch = default_patch();
        let mut v = Self {
            fs,
            target: patch,
            smooth: patch,
            k_smooth: one_pole_coeff(SMOOTH_TAU_S, fs / CONTROL_BLOCK as f32),
            noise: Noise::new(seed),
            engines: Engines::new(fs),
            engine: 0,
            ctx: Ctx::new(fs),
            block_pos: 0,
            velocity: 1.0,
            bend: 0.0,
            bend_mul: 0.0,
            whim: [0.0; 4],
            reply_oct: 0.0,
            dc: DcBlocker::default(),
            dc_r: 1.0 - core::f32::consts::TAU * 12.0 / fs,
            shifter: Shifter::new(fs),
            svf: Svf::default(),
            svf_coeffs: SvfCoeffs::new(1_000.0, 0.7, fs),
            filter_type: 0,
            medium: Medium::new(fs),
            level: 0.0,
            trim: 1.0,
        };
        v.control();
        v
    }

    pub fn set_param(&mut self, param: Param, value: f32) {
        let v = param.sanitize(value);
        self.target[param.index()] = v;
        if !SMOOTHED[param.index()] {
            self.smooth[param.index()] = v;
        }
    }

    pub fn param(&self, param: Param) -> f32 {
        self.target[param.index()]
    }

    /// The de-zippered value the voice is currently using.
    pub fn smoothed(&self, param: Param) -> f32 {
        self.smooth[param.index()]
    }

    /// Jump all smoothed parameters to their targets.
    pub fn snap_params(&mut self) {
        self.smooth = self.target;
        self.control();
    }

    /// Hit the channel. `velocity` is the raw hit strength `0..=1`; SENSE
    /// scales it like a trigger input's preamp.
    pub fn trigger(&mut self, velocity: f32) {
        self.trigger_with(velocity, 0.0);
    }

    /// Hit the channel with an extra pitch offset (a reply's tone of voice).
    pub fn trigger_with(&mut self, velocity: f32, pitch_oct: f32) {
        let gain = map::sense_gain(self.target[Param::Sense.index()]);
        self.velocity = (velocity.clamp(0.0, 1.0) * gain).clamp(0.02, 1.0);
        self.bend = 1.0;
        self.reply_oct = pitch_oct.clamp(-REPLY_MAX_OCT, REPLY_MAX_OCT);
        let amount = self.smooth[Param::Whim.index()];
        self.whim = if amount > 0.0 {
            [
                WHIM_PITCH_OCT * amount * amount * self.noise.sample(),
                WHIM_MACRO * amount * self.noise.sample(),
                WHIM_MACRO * amount * self.noise.sample(),
                WHIM_MACRO * amount * self.noise.sample(),
            ]
        } else {
            [0.0; 4]
        };
        // Evaluate the controls now so the object sees this hit's pitch,
        // whim and velocity from its first sample.
        self.control();
        self.block_pos = 1;
        self.engines
            .trigger(self.engine, &self.ctx, &mut self.noise);
    }

    /// The selected object.
    pub fn engine(&self) -> u32 {
        self.engine
    }

    /// Control-rate update: smoothing, modulation, coefficients.
    fn control(&mut self) {
        let fs = self.fs;
        for ((cur, &target), &smoothed) in self.smooth.iter_mut().zip(&self.target).zip(&SMOOTHED) {
            if smoothed {
                *cur += (target - *cur) * self.k_smooth;
            }
        }
        let p = self.smooth;
        let param = |q: Param| p[q.index()];

        let engine = param(Param::Engine) as u32;
        if engine != self.engine {
            self.engines.reset(engine);
            self.engine = engine;
        }
        self.trim = crate::engines::trim(engine);

        let oct = map::pitch_octaves(param(Param::Pitch), param(Param::Fine))
            + map::bend_octaves(param(Param::BendDepth)) * self.bend
            + self.whim[0]
            + self.reply_oct;
        let macro_value = |q: Param, j: f32| (param(q) + j).clamp(0.0, 1.0);
        self.ctx = Ctx {
            fs,
            hz: oct.exp2().clamp(8.0, 0.45 * fs),
            x: macro_value(Param::MacroX, self.whim[1]),
            y: macro_value(Param::MacroY, self.whim[2]),
            z: macro_value(Param::MacroZ, self.whim[3]),
            time_s: map::time_seconds(param(Param::Time)),
            velocity: self.velocity,
        };
        self.engines.control(self.engine, &self.ctx);

        self.bend_mul = (-1.0 / (map::bend_seconds(param(Param::BendTime)) * fs)).exp();
        self.shifter.control(
            param(Param::ShiftHz),
            param(Param::ShiftSpiral),
            param(Param::ShiftLag),
        );
        self.svf_coeffs = SvfCoeffs::new(
            map::cutoff_hz(param(Param::Cutoff)),
            map::reso_q(param(Param::Reso)),
            fs,
        );
        self.filter_type = param(Param::FilterType) as u32;
        self.medium
            .control(param(Param::Medium) as u32, param(Param::MediumDepth));
        self.level = map::level_gain(param(Param::Level));
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        if self.block_pos == 0 {
            self.control();
        }
        self.block_pos += 1;
        if self.block_pos == CONTROL_BLOCK {
            self.block_pos = 0;
        }
        self.bend *= self.bend_mul;
        if self.bend < 1e-6 {
            self.bend = 0.0;
        }

        let mut x = self.engines.tick(self.engine, &self.ctx, &mut self.noise) * self.trim;
        if !x.is_finite() {
            // Never let a numerical accident poison the channel.
            self.engines.reset(self.engine);
            self.svf.reset();
            x = 0.0;
        }
        x = self.dc.process(x, self.dc_r);

        if !self.shifter.is_bypassed() {
            x = self.shifter.process(x);
        }

        let (lp, bp, hp) = self.svf.process(x, &self.svf_coeffs);
        x = match self.filter_type {
            FILTER_BP => bp,
            FILTER_HP => hp,
            _ => lp,
        };

        x = self.medium.process(x, &mut self.noise);
        x * self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ENGINE_COUNT, ENGINE_PARADOX};

    fn render(v: &mut Voice, n: usize) -> Vec<f32> {
        (0..n).map(|_| v.tick()).collect()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0f32, |a, &b| a.max(b.abs()))
    }

    #[test]
    fn each_object_sounds_and_goes_quiet() {
        let fs = 96_000.0;
        for engine in 0..ENGINE_COUNT {
            let mut v = Voice::new(fs, 1);
            v.set_param(Param::Engine, engine as f32);
            v.set_param(Param::Time, 0.3);
            v.snap_params();
            v.trigger(1.0);
            // INSPIRAL builds to its climax TIME after the hit.
            let early = peak(&render(&mut v, 48_000));
            let _ = render(&mut v, fs as usize * 8);
            let late = peak(&render(&mut v, 9_600));
            assert!(early > 0.02, "engine {engine}: quiet ({early})");
            assert!(early < 2.5, "engine {engine}: loud ({early})");
            assert!(late < early * 0.01, "engine {engine}: {early} -> {late}");
        }
    }

    #[test]
    fn bend_down_starts_high() {
        let fs = 96_000.0;
        let mut v = Voice::new(fs, 1);
        v.set_param(Param::Engine, ENGINE_PARADOX as f32);
        v.set_param(Param::MacroX, 0.5); // no glide
        v.set_param(Param::MacroY, 0.0); // no pulse
        v.set_param(Param::Pitch, 0.4);
        v.set_param(Param::BendDepth, 0.85);
        v.set_param(Param::BendTime, 0.4);
        v.set_param(Param::Time, 0.9);
        v.snap_params();
        v.trigger(1.0);
        let out = render(&mut v, 96_000);
        let zc = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let first = zc(&out[..9_600]);
        let last = zc(&out[out.len() - 9_600..]);
        assert!(first as f32 > last as f32 * 1.5, "{first} vs {last}");
    }

    #[test]
    fn whim_makes_hits_differ() {
        let fs = 48_000.0;
        let mut v = Voice::new(fs, 7);
        v.set_param(Param::Engine, ENGINE_PARADOX as f32);
        v.set_param(Param::MacroX, 0.5);
        v.set_param(Param::MacroY, 0.0);
        v.set_param(Param::Whim, 1.0);
        v.set_param(Param::Time, 0.8);
        v.snap_params();
        let mut hits = Vec::new();
        for _ in 0..4 {
            v.trigger(1.0);
            hits.push(render(&mut v, 4_800));
        }
        assert!(hits.windows(2).all(|w| w[0] != w[1]));
    }

    #[test]
    fn extremes_stay_finite() {
        let fs = 96_000.0;
        for engine in 0..ENGINE_COUNT {
            for &corner in &[0.0f32, 1.0] {
                let mut v = Voice::new(fs, 9);
                v.set_param(Param::Engine, engine as f32);
                for p in [
                    Param::Pitch,
                    Param::Time,
                    Param::MacroX,
                    Param::MacroY,
                    Param::MacroZ,
                    Param::BendDepth,
                    Param::ShiftHz,
                    Param::ShiftSpiral,
                    Param::Reso,
                    Param::MediumDepth,
                ] {
                    v.set_param(p, corner);
                }
                v.set_param(Param::Medium, if corner > 0.5 { 5.0 } else { 1.0 });
                v.snap_params();
                v.trigger(1.0);
                for s in render(&mut v, 96_000) {
                    assert!(
                        s.is_finite() && s.abs() < 50.0,
                        "engine {engine} corner {corner}: {s}"
                    );
                }
            }
        }
    }
}
