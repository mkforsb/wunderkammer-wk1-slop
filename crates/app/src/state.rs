//! Panel state, the controller shared through the Dioxus context, and
//! persistence (localStorage on the web, `~/.config/wunderkammer` on desktop).

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use wunderkammer_dsp::params::{
    ENGINE_AUTOMATA, ENGINE_AVIARY, ENGINE_BALLOON, ENGINE_BOUNCE, ENGINE_COIN, ENGINE_CREAK,
    ENGINE_GLOSSA, ENGINE_ICE, ENGINE_INSPIRAL, ENGINE_MUSICBOX, ENGINE_PARADOX, ENGINE_POUR,
    FILTER_BP, FILTER_HP, FILTER_LP, MEDIUM_NAMES, MEDIUM_WALL, MEDIUM_WATER, ParamKind,
    default_patch,
};
use wunderkammer_dsp::util::Noise;
use wunderkammer_dsp::{ALL_PARAMS, PARAM_COUNT, Param, VOICES, VoicePatch};

use crate::audio::{AudioHandle, Command, ParamCache};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelState {
    pub voices: [VoicePatch; VOICES],
    pub master: f32,
    /// Per-channel MUTATE knob: 0 = off … 1 = full randomization on every
    /// trigger. A performance control rather than part of the sound, so it
    /// lives outside the patch and survives presets, RND and channel copies.
    pub mutate: [f32; VOICES],
    /// Controls that RND and MUTATE must leave alone (Ctrl-click to toggle).
    /// Like MUTATE they belong to the panel, not to the sound.
    pub locks: [Locks; VOICES],
}

/// One lock per parameter, indexed by `Param::index`.
pub type Locks = [bool; PARAM_COUNT];

impl Default for PanelState {
    fn default() -> Self {
        // Give the two channels different starting sounds.
        let preset = |name: &str| {
            wunderkammer_dsp::presets::PRESETS
                .iter()
                .find(|p| p.name == name)
                .map(|p| p.patch())
                .unwrap_or_else(default_patch)
        };
        let voices = [preset("Ping-Pong Accelerando"), preset("Filling a Bottle")];
        Self {
            voices,
            master: wunderkammer_dsp::engine::DEFAULT_MASTER,
            mutate: [0.0; VOICES],
            locks: [[false; PARAM_COUNT]; VOICES],
        }
    }
}

/// Where each object sounds best: (PITCH, TIME) ranges for RND.
fn sweet_spot(engine: u32) -> ((f32, f32), (f32, f32)) {
    match engine {
        ENGINE_BOUNCE => ((0.35, 0.55), (0.3, 0.7)),
        ENGINE_COIN => ((0.4, 0.78), (0.5, 0.72)),
        ENGINE_CREAK => ((0.3, 0.65), (0.45, 0.75)),
        ENGINE_GLOSSA => ((0.22, 0.55), (0.38, 0.62)),
        ENGINE_POUR => ((0.3, 0.55), (0.5, 0.72)),
        ENGINE_AVIARY => ((0.28, 0.48), (0.55, 0.8)),
        ENGINE_MUSICBOX => ((0.5, 0.68), (0.65, 0.9)),
        ENGINE_ICE => ((0.6, 0.85), (0.5, 0.85)),
        ENGINE_INSPIRAL => ((0.4, 0.65), (0.45, 0.7)),
        ENGINE_AUTOMATA => ((0.25, 0.45), (0.55, 0.85)),
        ENGINE_PARADOX => ((0.35, 0.6), (0.6, 0.9)),
        ENGINE_BALLOON => ((0.4, 0.75), (0.5, 0.78)),
        _ => ((0.3, 0.6), (0.4, 0.7)),
    }
}

/// A random patch, as produced by the RND button (and MUTATE at RND on every
/// hit). `current` supplies the values of the controls that RND leaves
/// alone: the channel's level, sense and pan, and anything in `locks`.
///
/// Every control draws exactly one number, in parameter order, whether or
/// not it is locked (so locking one control never changes what the others
/// roll). The draws are shaped so the results are playable: each object gets
/// pitches and times where it sounds good, and the effects and replies are
/// usually off and occasionally wild.
pub fn random_patch(current: &VoicePatch, locks: &Locks, rng: &mut Noise) -> VoicePatch {
    let mut patch = *current;
    let mut draws = [0.0f32; PARAM_COUNT];
    for p in ALL_PARAMS {
        draws[p.index()] = rng.sample() * 0.5 + 0.5;
    }
    let keep = |p: Param| !p.is_mutable() || locks[p.index()];
    let bipolar = |x: f32, depth: f32| 0.5 + depth * (2.0 * x - 1.0).powi(3);
    // "Usually off": the bottom `off` of the draw maps to `rest`, the rest of
    // it spreads over the control.
    let rarely = |x: f32, off: f32| ((x - off) / (1.0 - off)).max(0.0);
    for p in ALL_PARAMS {
        if keep(p) {
            continue;
        }
        let x = draws[p.index()];
        // Engine is drawn first, so the ranges below follow the new object.
        let engine = patch[Param::Engine.index()] as u32;
        let ((p_lo, p_hi), (t_lo, t_hi)) = sweet_spot(engine);
        patch[p.index()] = p.sanitize(match (p, p.info().kind) {
            (Param::Medium, _) => {
                let r = rarely(x, 0.55);
                if r <= 0.0 {
                    0.0
                } else {
                    1.0 + (r * (MEDIUM_NAMES.len() - 1) as f32).floor()
                }
            }
            (_, ParamKind::Choice(names)) => (x * names.len() as f32).floor(),
            (Param::Pitch, _) => p_lo + (p_hi - p_lo) * x,
            (Param::Time, _) => t_lo + (t_hi - t_lo) * x,
            (Param::Fine, _) => 0.3 + 0.4 * x,
            (Param::BendTime, _) => 0.3 + 0.6 * x,
            (Param::BendDepth, _) => bipolar(x, 0.35),
            (Param::Whim, _) => 0.3 * x * x,
            (Param::ShiftHz, _) => {
                let r = rarely(x, 0.7);
                if r <= 0.0 {
                    0.5
                } else {
                    0.5 + 0.45 * (2.0 * r - 1.0)
                }
            }
            (Param::ShiftSpiral, _) => 0.8 * x.powi(3),
            (Param::Cutoff, _) => 0.6 + 0.4 * x,
            (Param::Reso, _) => 0.4 * x,
            (Param::MediumDepth, _) => 0.35 + 0.6 * x,
            (Param::ReplyOdds, _) => 0.9 * rarely(x, 0.6),
            (Param::ReplyMood, _) => bipolar(x, 0.4),
            // Often dry, sometimes drenched.
            (Param::ReverbMix, _) => 0.65 * x * x,
            _ => x,
        });
    }
    // Walls and water are lowpasses already: a band- or highpass in front of
    // them leaves almost nothing, so they get the open lowpass.
    let medium = patch[Param::Medium.index()] as u32;
    if matches!(medium, MEDIUM_WALL | MEDIUM_WATER) && !keep(Param::FilterType) {
        patch[Param::FilterType.index()] = FILTER_LP as f32;
    }
    // A highpass or bandpass with a high cutoff would leave little of most
    // objects: pull its cutoff down into the body of the sound instead.
    if !keep(Param::Cutoff) {
        let x = draws[Param::Cutoff.index()];
        match patch[Param::FilterType.index()] as u32 {
            FILTER_HP => patch[Param::Cutoff.index()] = 0.05 + 0.35 * x,
            FILTER_BP => patch[Param::Cutoff.index()] = 0.35 + 0.35 * x,
            _ => {}
        }
    }
    patch
}

/// Move every unlocked control `amount` (0..=1) of the way towards a fresh
/// random patch: `clamp(current + (random - current) * amount)`. At 1 this is
/// exactly [`random_patch`]; choice controls round to the nearest position.
pub fn mutate_patch(
    current: &VoicePatch,
    locks: &Locks,
    amount: f32,
    rng: &mut Noise,
) -> VoicePatch {
    let target = random_patch(current, locks, rng);
    if amount >= 1.0 {
        return target;
    }
    let amount = amount.max(0.0);
    let mut patch = *current;
    for p in ALL_PARAMS {
        let (c, t) = (current[p.index()], target[p.index()]);
        patch[p.index()] = p.sanitize(c + (t - c) * amount);
    }
    patch
}

pub fn mutate_display(amount: f32) -> String {
    if amount <= 0.0 {
        "OFF".into()
    } else if amount >= 1.0 {
        "RND".into()
    } else {
        format!("{:.0}%", amount * 100.0)
    }
}

const FORMAT_HEADER: &str = "wunderkammer-state 1";

impl PanelState {
    pub fn to_cache(self) -> ParamCache {
        ParamCache {
            voices: self.voices,
            master: self.master,
        }
    }

    pub fn serialize(&self) -> String {
        let mut out = format!("{FORMAT_HEADER}\nmaster {}\n", self.master);
        for (v, m) in self.mutate.iter().enumerate() {
            out += &format!("mutate {v} {m}\n");
        }
        for (v, locks) in self.locks.iter().enumerate() {
            for p in ALL_PARAMS.iter().filter(|p| locks[p.index()]) {
                out += &format!("lock {v} {p:?}\n");
            }
        }
        for (v, patch) in self.voices.iter().enumerate() {
            for p in ALL_PARAMS {
                out += &format!("{v} {p:?} {}\n", patch[p.index()]);
            }
        }
        out
    }

    /// Lenient: unknown lines are skipped and missing values keep defaults.
    pub fn deserialize(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim() != FORMAT_HEADER {
            return None;
        }
        let mut state = PanelState::default();
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts.as_slice() {
                ["master", v] => {
                    if let Ok(v) = v.parse::<f32>() {
                        state.master = v.clamp(0.0, 1.0);
                    }
                }
                ["lock", voice, name] => {
                    let param = ALL_PARAMS.iter().find(|p| format!("{p:?}") == *name);
                    if let (Ok(voice), Some(p)) = (voice.parse::<usize>(), param)
                        && let Some(locks) = state.locks.get_mut(voice)
                        && p.is_mutable()
                    {
                        locks[p.index()] = true;
                    }
                }
                ["mutate", voice, v] => {
                    if let (Ok(voice), Ok(v)) = (voice.parse::<usize>(), v.parse::<f32>())
                        && let Some(m) = state.mutate.get_mut(voice)
                        && v.is_finite()
                    {
                        *m = v.clamp(0.0, 1.0);
                    }
                }
                [voice, name, v] => {
                    let (Ok(voice), Ok(v)) = (voice.parse::<usize>(), v.parse::<f32>()) else {
                        continue;
                    };
                    let param = ALL_PARAMS.iter().find(|p| format!("{p:?}") == *name);
                    if let (Some(patch), Some(p)) = (state.voices.get_mut(voice), param) {
                        patch[p.index()] = p.sanitize(v);
                    }
                }
                _ => {}
            }
        }
        Some(state)
    }
}

/// What a drag gesture is adjusting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Param(usize, Param),
    Master,
    /// A channel's MUTATE knob.
    Mutate(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub target: Target,
    pub start_y: f64,
    pub start_value: f32,
    /// Pixels of vertical travel for the full range.
    pub span_px: f64,
    pub fine: bool,
}

/// Everything the controls need, shared via `use_context::<Synth>()`.
#[derive(Clone)]
pub struct Synth {
    pub state: Signal<PanelState>,
    pub audio: AudioHandle,
    /// Last touched parameter per channel, for the channel display.
    pub touched: Signal<[Option<Param>; VOICES]>,
    /// Trigger counters per channel; the trigger LED keys its flash on these.
    pub hits: Signal<[u32; VOICES]>,
    pub drag: Signal<Option<Drag>>,
    /// One generator for RND and MUTATE, so rapid hits never share a seed.
    rng: Rc<RefCell<Noise>>,
}

impl Synth {
    pub fn new() -> Self {
        let initial = load().unwrap_or_default();
        Self {
            audio: AudioHandle::new(initial.to_cache()),
            state: Signal::new(initial),
            touched: Signal::new([None; VOICES]),
            hits: Signal::new([0; VOICES]),
            drag: Signal::new(None),
            rng: Rc::new(RefCell::new(Noise::new(random_seed()))),
        }
    }

    pub fn value(&self, target: Target) -> f32 {
        let s = self.state.read();
        match target {
            Target::Param(v, p) => s.voices[v][p.index()],
            Target::Master => s.master,
            Target::Mutate(v) => s.mutate[v],
        }
    }

    pub fn set(&self, target: Target, value: f32) {
        match target {
            Target::Param(voice, param) => self.set_param(voice, param, value),
            Target::Master => {
                let value = value.clamp(0.0, 1.0);
                let mut state = self.state;
                if state.peek().master != value {
                    state.write().master = value;
                    self.audio.send(Command::Master(value));
                }
            }
            Target::Mutate(voice) => {
                // UI-only: nothing to send, it acts when the channel is triggered.
                let value = value.clamp(0.0, 1.0);
                let mut state = self.state;
                if state.peek().mutate[voice] != value {
                    state.write().mutate[voice] = value;
                }
            }
        }
    }

    pub fn set_param(&self, voice: usize, param: Param, value: f32) {
        let value = param.sanitize(value);
        let mut state = self.state;
        if state.peek().voices[voice][param.index()] != value {
            state.write().voices[voice][param.index()] = value;
            self.audio.set_param(voice, param, value);
        }
        let mut touched = self.touched;
        if touched.peek()[voice] != Some(param) {
            touched.write()[voice] = Some(param);
        }
    }

    pub fn reset(&self, target: Target) {
        match target {
            Target::Param(_, p) => self.set(target, p.default_value()),
            Target::Master => self.set(target, wunderkammer_dsp::engine::DEFAULT_MASTER),
            Target::Mutate(_) => self.set(target, 0.0),
        }
        self.save();
    }

    pub fn load_patch(&self, voice: usize, patch: &VoicePatch) {
        self.apply_patch(voice, patch);
        self.save();
    }

    /// Set a whole patch, sending only the values that changed. The channel
    /// display falls back to its summary (mode, tune, decay).
    fn apply_patch(&self, voice: usize, patch: &VoicePatch) {
        let mut state = self.state;
        for p in ALL_PARAMS {
            let value = p.sanitize(patch[p.index()]);
            if state.peek().voices[voice][p.index()] != value {
                state.write().voices[voice][p.index()] = value;
                self.audio.set_param(voice, p, value);
            }
        }
        let mut touched = self.touched;
        touched.write()[voice] = None;
    }

    /// Randomize the sound-shaping controls, leaving level, sense, pan and
    /// any locked control alone.
    pub fn randomize(&self, voice: usize) {
        let state = self.state.peek();
        let (current, locks) = (state.voices[voice], state.locks[voice]);
        drop(state);
        let patch = random_patch(&current, &locks, &mut self.rng.borrow_mut());
        self.load_patch(voice, &patch);
    }

    pub fn is_locked(&self, voice: usize, param: Param) -> bool {
        self.state.read().locks[voice][param.index()]
    }

    /// Ctrl-click: lock or unlock a control against RND and MUTATE.
    pub fn toggle_lock(&self, voice: usize, param: Param) {
        if !param.is_mutable() {
            return;
        }
        let mut state = self.state;
        {
            let mut state = state.write();
            let locked = &mut state.locks[voice][param.index()];
            *locked = !*locked;
        }
        self.save();
    }

    pub fn lock_count(&self, voice: usize) -> usize {
        self.state.read().locks[voice]
            .iter()
            .filter(|&&l| l)
            .count()
    }

    pub fn unlock_all(&self, voice: usize) {
        let mut state = self.state;
        state.write().locks[voice] = [false; PARAM_COUNT];
        self.save();
    }

    pub fn copy_voice(&self, from: usize, to: usize) {
        let mut patch = self.state.peek().voices[from];
        // Keep the destination's placement in the stereo field.
        patch[Param::Pan.index()] = self.state.peek().voices[to][Param::Pan.index()];
        self.load_patch(to, &patch);
    }

    /// Hit a channel. With MUTATE above zero the channel's patch is mutated
    /// first, and the trigger snaps the voice so the new sound starts exactly
    /// on this hit rather than gliding in.
    pub fn trigger(&self, voice: usize, velocity: f32) {
        self.audio.user_gesture();
        let amount = self.state.peek().mutate[voice];
        let mutated = amount > 0.0;
        if mutated {
            let state = self.state.peek();
            let (current, locks) = (state.voices[voice], state.locks[voice]);
            drop(state);
            let patch = mutate_patch(&current, &locks, amount, &mut self.rng.borrow_mut());
            self.apply_patch(voice, &patch);
            self.save();
        }
        self.audio.trigger(voice, velocity, mutated);
        let mut hits = self.hits;
        hits.write()[voice] += 1;
    }

    pub fn save(&self) {
        save(&self.state.peek());
    }
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn random_seed() -> u32 {
    (js_sys::Math::random() * u32::MAX as f64) as u32
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn random_seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() ^ d.as_secs() as u32)
        .unwrap_or(12345)
}

// --- persistence -----------------------------------------------------------

#[cfg(all(feature = "web", not(feature = "desktop")))]
const STORAGE_KEY: &str = "wunderkammer.state";

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn load() -> Option<PanelState> {
    let storage = web_sys::window()?.local_storage().ok()??;
    PanelState::deserialize(&storage.get_item(STORAGE_KEY).ok()??)
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn save(state: &PanelState) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item(STORAGE_KEY, &state.serialize());
    }
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn state_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("wunderkammer").join("state.txt"))
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn load() -> Option<PanelState> {
    PanelState::deserialize(&std::fs::read_to_string(state_path()?).ok()?)
}

#[cfg(not(all(feature = "web", not(feature = "desktop"))))]
fn save(state: &PanelState) {
    let Some(path) = state_path() else { return };
    let result = path
        .parent()
        .map(std::fs::create_dir_all)
        .unwrap_or(Ok(()))
        .and_then(|_| std::fs::write(&path, state.serialize()));
    if let Err(e) = result {
        eprintln!(
            "wunderkammer: could not save state to {}: {e}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wunderkammer_dsp::params::ENGINE_COUNT;

    const NO_LOCKS: Locks = [false; PARAM_COUNT];

    fn is_sanitized(patch: &VoicePatch) -> bool {
        ALL_PARAMS
            .iter()
            .all(|p| p.sanitize(patch[p.index()]) == patch[p.index()])
    }

    #[test]
    fn mutate_off_changes_nothing() {
        let current = default_patch();
        assert_eq!(
            mutate_patch(&current, &NO_LOCKS, 0.0, &mut Noise::new(7)),
            current
        );
    }

    #[test]
    fn mutate_at_rnd_equals_the_rnd_button() {
        let current = default_patch();
        let a = mutate_patch(&current, &NO_LOCKS, 1.0, &mut Noise::new(7));
        let b = random_patch(&current, &NO_LOCKS, &mut Noise::new(7));
        assert_eq!(a, b);
        assert_ne!(a, current);
    }

    #[test]
    fn partial_mutation_moves_part_way_towards_the_random_patch() {
        let current = default_patch();
        for seed in 1..50 {
            let target = random_patch(&current, &NO_LOCKS, &mut Noise::new(seed));
            let half = mutate_patch(&current, &NO_LOCKS, 0.5, &mut Noise::new(seed));
            assert!(is_sanitized(&half));
            for p in ALL_PARAMS {
                let (c, t, h) = (current[p.index()], target[p.index()], half[p.index()]);
                match p.info().kind {
                    ParamKind::Continuous => {
                        assert!((h - (c + (t - c) * 0.5)).abs() < 1e-6, "{p:?}")
                    }
                    ParamKind::Choice(_) => assert_eq!(h, p.sanitize(c + (t - c) * 0.5), "{p:?}"),
                }
            }
        }
    }

    #[test]
    fn mutation_keeps_level_sense_and_pan() {
        let mut current = default_patch();
        current[Param::Level.index()] = 0.33;
        current[Param::Sense.index()] = 0.66;
        current[Param::Pan.index()] = 0.1;
        let mut rng = Noise::new(3);
        for amount in [0.25, 0.5, 1.0] {
            let m = mutate_patch(&current, &NO_LOCKS, amount, &mut rng);
            for p in [Param::Level, Param::Sense, Param::Pan] {
                assert_eq!(m[p.index()], current[p.index()]);
            }
        }
    }

    #[test]
    fn locked_controls_survive_rnd_and_mutate() {
        let mut current = default_patch();
        current[Param::Engine.index()] = ENGINE_MUSICBOX as f32;
        current[Param::MacroX.index()] = 0.123;
        current[Param::Pitch.index()] = 0.987;
        let mut locks = NO_LOCKS;
        for p in [Param::Engine, Param::MacroX, Param::Pitch] {
            locks[p.index()] = true;
        }
        let mut rng = Noise::new(9);
        for i in 0..200 {
            let m = if i % 2 == 0 {
                random_patch(&current, &locks, &mut rng)
            } else {
                mutate_patch(&current, &locks, 0.6, &mut rng)
            };
            for p in [Param::Engine, Param::MacroX, Param::Pitch] {
                assert_eq!(m[p.index()], current[p.index()], "{p:?}");
            }
        }
    }

    #[test]
    fn a_lock_doesnt_change_what_the_others_roll() {
        let current = default_patch();
        let mut locks = NO_LOCKS;
        locks[Param::Cutoff.index()] = true;
        let a = random_patch(&current, &NO_LOCKS, &mut Noise::new(4));
        let b = random_patch(&current, &locks, &mut Noise::new(4));
        for p in ALL_PARAMS.iter().filter(|p| !matches!(p, Param::Cutoff)) {
            assert_eq!(a[p.index()], b[p.index()], "{p:?}");
        }
        assert_eq!(b[Param::Cutoff.index()], current[Param::Cutoff.index()]);
    }

    #[test]
    fn repeated_mutation_stays_in_range() {
        let mut patch = default_patch();
        let mut rng = Noise::new(11);
        for i in 0..1000 {
            patch = mutate_patch(&patch, &NO_LOCKS, [0.1, 0.5, 0.9][i % 3], &mut rng);
            assert!(is_sanitized(&patch));
        }
    }

    #[test]
    fn mutate_display_names_the_ends() {
        assert_eq!(mutate_display(0.0), "OFF");
        assert_eq!(mutate_display(0.5), "50%");
        assert_eq!(mutate_display(1.0), "RND");
    }

    #[test]
    fn state_roundtrip() {
        let mut s = PanelState {
            master: 0.33,
            mutate: [0.5, 1.0],
            ..Default::default()
        };
        s.voices[0][Param::Engine.index()] = 4.0;
        s.voices[1][Param::Pitch.index()] = 0.123;
        s.locks[1][Param::MacroY.index()] = true;
        s.locks[0][Param::Engine.index()] = true;
        let back = PanelState::deserialize(&s.serialize()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn rejects_foreign_text() {
        assert!(PanelState::deserialize("hello").is_none());
        assert!(PanelState::deserialize("xenokussion-state 1\nmaster 0.5\n").is_none());
    }

    #[test]
    fn tolerates_garbage_lines() {
        let text = format!(
            "{FORMAT_HEADER}\nmaster 0.5\n0 Nope 1\n7 Pitch 0.2\n0 Pitch abc\n1 Engine 3\nlock 0 Level\nlock 9 Pitch\nlock 1 Bogus\n"
        );
        let s = PanelState::deserialize(&text).unwrap();
        assert_eq!(s.master, 0.5);
        assert_eq!(s.voices[1][Param::Engine.index()], 3.0);
        assert!(s.locks.iter().all(|l| l.iter().all(|&x| !x)));
    }

    /// RND (and MUTATE at RND) must always give something audible and sane,
    /// whichever object it lands on.
    #[test]
    fn random_patches_are_playable() {
        use wunderkammer_dsp::voice::Voice;
        let mut rng = Noise::new(2024);
        let mut engines = [0usize; ENGINE_COUNT];
        let fs = 24_000.0;
        for i in 0..600 {
            let patch = random_patch(&default_patch(), &NO_LOCKS, &mut rng);
            engines[patch[Param::Engine.index()] as usize] += 1;
            let mut v = Voice::new(fs, i);
            for p in ALL_PARAMS {
                v.set_param(p, patch[p.index()]);
            }
            v.snap_params();
            v.trigger(0.9);
            let out: Vec<f32> = (0..(fs * 2.5) as usize).map(|_| v.tick()).collect();
            let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(out.iter().all(|x| x.is_finite()), "patch {i}");
            assert!(
                peak > 2e-3 && peak < 4.0,
                "patch {i}: peak {peak} {patch:?}"
            );
        }
        assert!(engines.iter().all(|&n| n > 25), "engine spread {engines:?}");
    }

    /// Both channels at RND: one press of Space gives both a brand new sound.
    #[test]
    fn space_at_rnd_rerolls_both_channels() {
        let mut state = PanelState {
            mutate: [1.0; VOICES],
            ..Default::default()
        };
        let mut rng = Noise::new(5);
        for _ in 0..20 {
            let before = state.voices;
            for (patch, &amount) in state.voices.iter_mut().zip(&state.mutate) {
                *patch = mutate_patch(patch, &NO_LOCKS, amount, &mut rng);
            }
            for (now, then) in state.voices.iter().zip(&before) {
                assert_ne!(now, then);
            }
            assert_ne!(state.voices[0], state.voices[1]);
        }
    }
}
