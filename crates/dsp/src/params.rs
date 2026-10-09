//! Parameter definitions shared by the engine, the wasm worklet and the UI.
//!
//! Every parameter is stored as an `f32`. Continuous parameters are normalized
//! to `0.0..=1.0` (think "knob position"); choice parameters hold the index of
//! the selected option. The [`map`] module converts knob positions into
//! physical units and is used by both the DSP and the UI read-outs, so what the
//! panel displays is exactly what the engine does.
//!
//! The three macro knobs (X, Y, Z) mean something different for every object;
//! [`macro_label`] and [`macro_display`] give their per-engine names.

/// Per-voice parameters. The discriminant is the stable wire id.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Param {
    /// Sound object (see [`ENGINE_NAMES`]).
    Engine = 0,
    /// Reference pitch (each object says what it means).
    Pitch,
    /// Fine tune.
    Fine,
    /// How long the object's event lasts.
    Time,
    /// Object macro 1.
    MacroX,
    /// Object macro 2.
    MacroY,
    /// Object macro 3.
    MacroZ,
    /// Pitch bend time.
    BendTime,
    /// Pitch bend depth, bipolar (centre = off, up = starts high).
    BendDepth,
    /// Per-hit random offsets of pitch and macros.
    Whim,
    /// Frequency shifter amount, bipolar.
    ShiftHz,
    /// Frequency shifter feedback (barber-pole echoes).
    ShiftSpiral,
    /// Frequency shifter feedback delay.
    ShiftLag,
    /// Tone filter cutoff.
    Cutoff,
    /// Tone filter resonance.
    Reso,
    /// Tone filter type: LP / BP / HP.
    FilterType,
    /// The medium the sound is heard through.
    Medium,
    /// How much of the medium.
    MediumDepth,
    /// Probability that this channel answers the other channel's hits.
    ReplyOdds,
    /// How long this channel waits before answering.
    ReplyLag,
    /// Bipolar: replies calm down (quieter, lower) or escalate (louder, higher).
    ReplyMood,
    /// Channel output level.
    Level,
    /// Trigger sensitivity.
    Sense,
    /// Stereo position.
    Pan,
    /// Reverb send level (post channel output level).
    ReverbMix,
    /// Reverb decay time (≈ RT60).
    ReverbDecay,
    /// Reverb damping / brightness.
    ReverbTone,
    /// Reverb pre-delay.
    ReverbPredelay,
}

pub const PARAM_COUNT: usize = 28;

pub const ALL_PARAMS: [Param; PARAM_COUNT] = [
    Param::Engine,
    Param::Pitch,
    Param::Fine,
    Param::Time,
    Param::MacroX,
    Param::MacroY,
    Param::MacroZ,
    Param::BendTime,
    Param::BendDepth,
    Param::Whim,
    Param::ShiftHz,
    Param::ShiftSpiral,
    Param::ShiftLag,
    Param::Cutoff,
    Param::Reso,
    Param::FilterType,
    Param::Medium,
    Param::MediumDepth,
    Param::ReplyOdds,
    Param::ReplyLag,
    Param::ReplyMood,
    Param::Level,
    Param::Sense,
    Param::Pan,
    Param::ReverbMix,
    Param::ReverbDecay,
    Param::ReverbTone,
    Param::ReverbPredelay,
];

pub const ENGINE_BOUNCE: u32 = 0;
pub const ENGINE_COIN: u32 = 1;
pub const ENGINE_CREAK: u32 = 2;
pub const ENGINE_GLOSSA: u32 = 3;
pub const ENGINE_POUR: u32 = 4;
pub const ENGINE_AVIARY: u32 = 5;
pub const ENGINE_MUSICBOX: u32 = 6;
pub const ENGINE_ICE: u32 = 7;
pub const ENGINE_INSPIRAL: u32 = 8;
pub const ENGINE_AUTOMATA: u32 = 9;
pub const ENGINE_PARADOX: u32 = 10;
pub const ENGINE_BALLOON: u32 = 11;
pub const ENGINE_COUNT: usize = 12;

pub const ENGINE_NAMES: &[&str] = &[
    "BOUNCE", "COIN", "CREAK", "GLOSSA", "POUR", "AVIARY", "MUSICBOX", "ICE", "INSPIRAL",
    "AUTOMATA", "PARADOX", "BALLOON",
];

/// One-line description of each object, for the panel.
pub const ENGINE_BLURBS: [&str; ENGINE_COUNT] = [
    "Balls dropped on a struck drumhead, which can be shaken like a vibrating table",
    "A coin or pot lid spinning down to a finite-time singularity, then a final clack",
    "Stick-slip friction dragged across a body: door hinges, balloons, wet glass",
    "A tiny vocal tract that mutters a new nonsense word on every hit",
    "Water poured into a bottle: the air column shortens and the pitch rises",
    "A flock of synthetic creatures startled by the hit",
    "A wind-up music box whose pins were punched by number theory",
    "Flexural waves in a frozen lake: every echo is a falling laser chirp",
    "Two black holes spiral together: chirp, merger, ringdown",
    "A cellular automaton plays the harmonic series, one generation at a time",
    "Shepard-Risset illusion: a fall that never gets lower, a beat that never stops speeding up",
    "A balloon let go: rubber nozzle squeal under a collapsing pressure, flying around the room",
];

pub const FILTER_TYPE_NAMES: &[&str] = &["LP", "BP", "HP"];
pub const FILTER_LP: u32 = 0;
pub const FILTER_BP: u32 = 1;
pub const FILTER_HP: u32 = 2;

pub const MEDIUM_NAMES: &[&str] = &["AIR", "TIN CAN", "TUBE", "WATER", "WALL", "RADIO", "HORN"];
pub const MEDIUM_AIR: u32 = 0;
pub const MEDIUM_TIN_CAN: u32 = 1;
pub const MEDIUM_TUBE: u32 = 2;
pub const MEDIUM_WATER: u32 = 3;
pub const MEDIUM_WALL: u32 = 4;
pub const MEDIUM_RADIO: u32 = 5;
pub const MEDIUM_HORN: u32 = 6;

/// Knob positions at or above this pick a new random seed on every hit
/// (GLOSSA words, AVIARY species, MUSICBOX tunes, AUTOMATA rules).
pub const RND_THRESHOLD: f32 = 0.96;
/// Number of fixed seeds below the RND position.
pub const SEEDS: u32 = 48;

/// The sequences a MUSICBOX cylinder can be punched with.
pub const TUNE_NAMES: [&str; 5] = ["COLLATZ", "RECAMAN", "PI", "FIB", "NORGARD"];
/// MUSICBOX scales, in knob order.
pub const SCALE_NAMES: [&str; 6] = ["HARMONIC", "PENTA", "MAJOR", "HIJAZ", "WHOLE", "BOHLEN-P"];
/// COIN objects, in knob order.
pub const OBJECT_NAMES: [&str; 4] = ["COIN", "POT LID", "PLATE", "BOWL"];
/// CREAK bodies, in knob order.
pub const BODY_NAMES: [&str; 4] = ["DOOR", "BALLOON", "GLASS", "CELLO"];
/// CREAK gestures, in knob order.
pub const GESTURE_NAMES: [&str; 4] = ["SWELL", "SWIPE", "TREMBLE", "JUDDER"];
/// GLOSSA sources, in knob order.
pub const SOURCE_NAMES: [&str; 4] = ["POP", "VOICE", "JAW HARP", "WHISPER"];
/// AUTOMATA partial maps, in knob order.
pub const CELLMAP_NAMES: [&str; 5] = ["HARMONIC", "ODD", "BELL", "UNDERTONE", "PENTA"];
/// AUTOMATA rules, in knob order (chosen for long-lived, varied patterns).
pub const CA_RULES: [u8; 16] = [
    90, 30, 110, 150, 105, 45, 73, 54, 22, 126, 60, 18, 57, 62, 94, 169,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    Continuous,
    Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct ParamInfo {
    pub param: Param,
    /// Descriptive name.
    pub name: &'static str,
    /// Panel label.
    pub label: &'static str,
    pub default: f32,
    pub kind: ParamKind,
}

impl Param {
    pub fn id(self) -> u32 {
        self as u32
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_id(id: u32) -> Option<Param> {
        ALL_PARAMS.get(id as usize).copied()
    }

    pub fn info(self) -> ParamInfo {
        use ParamKind::*;
        let (name, label, default, kind) = match self {
            Param::Engine => ("Object", "OBJECT", 0.0, Choice(ENGINE_NAMES)),
            Param::Pitch => ("Pitch", "PITCH", 0.42, Continuous),
            Param::Fine => ("Fine tune", "FINE", 0.5, Continuous),
            Param::Time => ("Time", "TIME", 0.5, Continuous),
            Param::MacroX => ("Macro X", "X", 0.35, Continuous),
            Param::MacroY => ("Macro Y", "Y", 0.5, Continuous),
            Param::MacroZ => ("Macro Z", "Z", 0.3, Continuous),
            Param::BendTime => ("Bend time", "TIME", 0.5, Continuous),
            Param::BendDepth => ("Bend depth", "DEPTH", 0.5, Continuous),
            Param::Whim => ("Per-hit whim", "WHIM", 0.0, Continuous),
            Param::ShiftHz => ("Frequency shift", "SHIFT", 0.5, Continuous),
            Param::ShiftSpiral => ("Spiral feedback", "SPIRAL", 0.0, Continuous),
            Param::ShiftLag => ("Spiral lag", "LAG", 0.4, Continuous),
            Param::Cutoff => ("Tone cutoff", "CUTOFF", 1.0, Continuous),
            Param::Reso => ("Tone resonance", "RESO", 0.1, Continuous),
            Param::FilterType => (
                "Tone filter type",
                "TYPE",
                FILTER_LP as f32,
                Choice(FILTER_TYPE_NAMES),
            ),
            Param::Medium => (
                "Heard through",
                "THROUGH",
                MEDIUM_AIR as f32,
                Choice(MEDIUM_NAMES),
            ),
            Param::MediumDepth => ("Medium depth", "DEPTH", 0.6, Continuous),
            Param::ReplyOdds => ("Reply odds", "ODDS", 0.0, Continuous),
            Param::ReplyLag => ("Reply lag", "LAG", 0.45, Continuous),
            Param::ReplyMood => ("Reply mood", "MOOD", 0.5, Continuous),
            Param::Level => ("Output", "OUTPUT", 0.75, Continuous),
            Param::Sense => ("Sense", "SENSE", 0.5, Continuous),
            Param::Pan => ("Pan", "PAN", 0.5, Continuous),
            Param::ReverbMix => ("Reverb amount", "AMOUNT", 0.3, Continuous),
            Param::ReverbDecay => ("Reverb decay", "DECAY", 0.45, Continuous),
            Param::ReverbTone => ("Reverb tone", "TONE", 0.6, Continuous),
            Param::ReverbPredelay => ("Reverb pre-delay", "PRE", 0.1, Continuous),
        };
        ParamInfo {
            param: self,
            name,
            label,
            default,
            kind,
        }
    }

    pub fn default_value(self) -> f32 {
        self.info().default
    }

    /// Clamp/quantize a raw value to what this parameter accepts.
    pub fn sanitize(self, v: f32) -> f32 {
        let v = if v.is_finite() {
            v
        } else {
            self.default_value()
        };
        match self.info().kind {
            ParamKind::Continuous => v.clamp(0.0, 1.0),
            ParamKind::Choice(names) => v.round().clamp(0.0, (names.len() - 1) as f32),
        }
    }

    /// Bipolar controls are centred at 0.5 (the panel lights them from the middle).
    pub fn is_bipolar(self) -> bool {
        matches!(
            self,
            Param::Fine | Param::BendDepth | Param::ShiftHz | Param::ReplyMood | Param::Pan
        )
    }

    /// RND and MUTATE never touch these: the channel's level, trigger
    /// sensitivity and stereo placement. They can't be locked either, since
    /// they are never randomized in the first place.
    pub fn is_mutable(self) -> bool {
        !matches!(self, Param::Level | Param::Sense | Param::Pan)
    }

    /// The macro slot (0 = X, 1 = Y, 2 = Z) of a macro parameter.
    pub fn macro_slot(self) -> Option<usize> {
        match self {
            Param::MacroX => Some(0),
            Param::MacroY => Some(1),
            Param::MacroZ => Some(2),
            _ => None,
        }
    }

    /// Panel label, resolving macro knobs to the given engine's names.
    pub fn label_for(self, engine: u32) -> &'static str {
        match self.macro_slot() {
            Some(slot) => macro_label(engine, slot),
            None => self.info().label,
        }
    }

    /// Descriptive name, resolving macro knobs to the given engine's names.
    pub fn name_for(self, engine: u32) -> &'static str {
        match self.macro_slot() {
            Some(slot) => macro_name(engine, slot),
            None => self.info().name,
        }
    }

    /// Human readable value for the panel read-out. Macro knobs need the
    /// engine; use [`Param::display_for`] for those.
    pub fn display(self, v: f32) -> String {
        self.display_for(ENGINE_BOUNCE, v)
    }

    /// Human readable value, resolving macro knobs for the given engine.
    pub fn display_for(self, engine: u32, v: f32) -> String {
        if let Some(slot) = self.macro_slot() {
            return macro_display(engine, slot, v);
        }
        match self {
            Param::Engine | Param::FilterType | Param::Medium => match self.info().kind {
                ParamKind::Choice(names) => names[self.sanitize(v) as usize].to_string(),
                ParamKind::Continuous => unreachable!(),
            },
            Param::Pitch => fmt_hz(map::pitch_hz(v, 0.5)),
            Param::Fine => format!("{:+.0} ct", map::fine_semitones(v) * 100.0),
            Param::Time => fmt_secs(map::time_seconds(v)),
            Param::BendTime => fmt_secs(map::bend_seconds(v)),
            Param::BendDepth => {
                let o = map::bend_octaves(v);
                if o.abs() < 0.005 {
                    "OFF".into()
                } else {
                    format!("{:+.2} oct", o)
                }
            }
            Param::Whim | Param::ShiftSpiral | Param::MediumDepth => off_or_pct(v),
            Param::ShiftHz => {
                let hz = map::shift_hz(v);
                if hz == 0.0 {
                    "OFF".into()
                } else if hz.abs() >= 100.0 {
                    format!("{:+.0} Hz", hz)
                } else {
                    format!("{:+.1} Hz", hz)
                }
            }
            Param::ShiftLag | Param::ReplyLag => fmt_secs(if self == Param::ShiftLag {
                map::shift_lag_seconds(v)
            } else {
                map::reply_lag_seconds(v)
            }),
            Param::Cutoff => fmt_hz(map::cutoff_hz(v)),
            Param::Reso => format!("Q {:.1}", map::reso_q(v)),
            Param::ReplyOdds => {
                if v <= 0.0 {
                    "NEVER".into()
                } else if v >= 1.0 {
                    "ALWAYS".into()
                } else {
                    format!("{:.0}%", v * 100.0)
                }
            }
            Param::ReplyMood => {
                let m = map::bipolar(v);
                if m.abs() < 0.03 {
                    "CALM".into()
                } else if m < 0.0 {
                    format!("SULK {:.0}%", -m * 100.0)
                } else {
                    format!("RILED {:.0}%", m * 100.0)
                }
            }
            Param::Level => fmt_db(map::level_gain(v)),
            Param::Sense => format!("x{:.2}", map::sense_gain(v)),
            Param::Pan => {
                let p = (v - 0.5) * 200.0;
                if p.abs() < 1.0 {
                    "C".to_string()
                } else if p < 0.0 {
                    format!("L{:.0}", -p)
                } else {
                    format!("R{:.0}", p)
                }
            }
            Param::ReverbMix => fmt_db(map::reverb_gain(v)),
            Param::ReverbDecay => fmt_secs(map::reverb_seconds(v)),
            Param::ReverbTone => fmt_hz(map::reverb_tone_hz(v)),
            Param::ReverbPredelay => fmt_secs(map::reverb_predelay_seconds(v)),
            Param::MacroX | Param::MacroY | Param::MacroZ => unreachable!(),
        }
    }
}

fn off_or_pct(v: f32) -> String {
    if v <= 0.0 {
        "OFF".into()
    } else {
        format!("{:.0}%", v * 100.0)
    }
}

/// Short panel label of an object macro knob.
pub fn macro_label(engine: u32, slot: usize) -> &'static str {
    const LABELS: [[&str; 3]; ENGINE_COUNT] = [
        ["COUNT", "BOUNCE", "SHAKE"],
        ["WOBBLE", "RIM", "OBJECT"],
        ["PRESSURE", "GESTURE", "BODY"],
        ["WORD", "SOURCE", "MOUTH"],
        ["FILL", "FLOW", "FIZZ"],
        ["SPECIES", "FLOCK", "FRENZY"],
        ["TUNE", "SCALE", "WEAR"],
        ["DISTANCE", "ECHOES", "SHATTER"],
        ["MASS", "ECCENTRIC", "SPIN"],
        ["RULE", "RATE", "MAP"],
        ["GLIDE", "PULSE", "SPACING"],
        ["SIZE", "FLAP", "FLIGHT"],
    ];
    LABELS[(engine as usize).min(ENGINE_COUNT - 1)][slot.min(2)]
}

/// Descriptive name of an object macro knob.
pub fn macro_name(engine: u32, slot: usize) -> &'static str {
    const NAMES: [[&str; 3]; ENGINE_COUNT] = [
        [
            "Ball count (fewer, bigger … many, tiny)",
            "Bounciness (restitution)",
            "Table shake",
        ],
        [
            "Wobble (how sharp the singularity)",
            "Rim roughness",
            "Spinning object",
        ],
        ["Normal pressure", "Hand gesture", "Body"],
        ["Word", "Voice source", "Mouth size"],
        ["Fill (down: drain)", "Flow (stream … glug)", "Fizz"],
        ["Species", "Flock size", "Frenzy"],
        ["Tune (punched sequence)", "Scale", "Wear and rust"],
        ["Distance (chirp length)", "Echoes", "Shatter (cracks)"],
        ["Mass ratio", "Eccentricity", "Spin"],
        ["Rule", "Generation rate", "Cell → partial map"],
        [
            "Glide (endless fall … rise)",
            "Risset pulse",
            "Partial spacing",
        ],
        ["Balloon size", "Nozzle flap", "Flight"],
    ];
    NAMES[(engine as usize).min(ENGINE_COUNT - 1)][slot.min(2)]
}

/// Read-out of an object macro knob.
pub fn macro_display(engine: u32, slot: usize, v: f32) -> String {
    let pct = || format!("{:.0}%", v * 100.0);
    let pick = |names: &[&str]| names[map::choose(v, names.len())].to_string();
    match (engine, slot) {
        (ENGINE_BOUNCE, 0) => format!(
            "{} ball{}",
            map::ball_count(v),
            if map::ball_count(v) == 1 { "" } else { "s" }
        ),
        (ENGINE_BOUNCE, 1) => format!("e {:.2}", map::restitution(v)),
        (ENGINE_BOUNCE, 2) => {
            let g = map::shake_gamma(v);
            if g <= 0.0 {
                "STILL".into()
            } else {
                format!("Γ {:.2}", g)
            }
        }
        (ENGINE_COIN, 0) => format!("t^-{:.2}", map::wobble_exponent(v)),
        (ENGINE_COIN, 2) => morph_display(&OBJECT_NAMES, v),
        (ENGINE_CREAK, 1) => morph_display(&GESTURE_NAMES, v),
        (ENGINE_CREAK, 2) => morph_display(&BODY_NAMES, v),
        (ENGINE_GLOSSA, 0) | (ENGINE_AVIARY, 0) => match map::seed(v) {
            Some(s) => format!("#{:02}", s),
            None => "RND".into(),
        },
        (ENGINE_GLOSSA, 1) => morph_display(&SOURCE_NAMES, v),
        (ENGINE_GLOSSA, 2) => format!("{:.0} cm", map::tract_cm(v)),
        (ENGINE_POUR, 0) => {
            let o = map::fill_octaves(v);
            if o.abs() < 0.02 {
                "LEVEL".into()
            } else if o > 0.0 {
                format!("FILL {:+.1} oct", o)
            } else {
                format!("DRAIN {:+.1} oct", o)
            }
        }
        (ENGINE_AVIARY, 1) => format!(
            "{} bird{}",
            map::flock_size(v),
            if map::flock_size(v) == 1 { "" } else { "s" }
        ),
        (ENGINE_MUSICBOX, 0) => match map::tune(v) {
            Some((kind, n)) => match kind {
                0 => format!("COLLATZ {}", map::collatz_start(n)),
                1 => format!("RECAMAN +{}", n),
                2 => format!("PI @{}", n * 7),
                3 => format!("FIB %{}", 5 + n % 9),
                _ => format!("NORGARD @{}", n * 8 % 64),
            },
            None => "RND".into(),
        },
        (ENGINE_MUSICBOX, 1) => pick(&SCALE_NAMES),
        (ENGINE_ICE, 0) => fmt_secs(map::ice_distance_seconds(v)),
        (ENGINE_ICE, 1) => format!(
            "{} echo{}",
            map::ice_echoes(v),
            if map::ice_echoes(v) == 1 { "" } else { "es" }
        ),
        (ENGINE_INSPIRAL, 0) => format!("q {:.1}", map::mass_ratio(v)),
        (ENGINE_INSPIRAL, 1) => format!("e {:.2}", map::eccentricity(v)),
        (ENGINE_INSPIRAL, 2) => format!("χ {:.2}", map::spin(v)),
        (ENGINE_AUTOMATA, 0) => match map::rule(v) {
            Some(r) => format!("RULE {}", r),
            None => "RND".into(),
        },
        (ENGINE_AUTOMATA, 1) => format!("{:.1} gen/s", map::ca_rate(v)),
        (ENGINE_AUTOMATA, 2) => morph_display(&CELLMAP_NAMES, v),
        (ENGINE_PARADOX, 0) => {
            let g = map::glide_rate(v);
            if g.abs() < 0.01 {
                "HOVER".into()
            } else if g < 0.0 {
                format!("FALL {:.2}/s", -g)
            } else {
                format!("RISE {:.2}/s", g)
            }
        }
        (ENGINE_PARADOX, 2) => format!("x{:.3}", map::paradox_ratio(v)),
        (ENGINE_BALLOON, 0) => format!("λ {:.2}", map::balloon_stretch(v)),
        (ENGINE_BALLOON, 2) => {
            if v < 0.02 {
                "HELD".into()
            } else {
                pct()
            }
        }
        _ => pct(),
    }
}

/// "A>B 40%" for a knob that morphs through named states.
fn morph_display(names: &[&str], v: f32) -> String {
    let (i, f) = map::segment(v, names.len());
    if f < 0.04 {
        names[i].to_string()
    } else if f > 0.96 {
        names[i + 1].to_string()
    } else {
        let short = |s: &str| s.chars().take(4).collect::<String>();
        format!(
            "{}>{} {:.0}%",
            short(names[i]),
            short(names[i + 1]),
            f * 100.0
        )
    }
}

fn fmt_hz(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{:.2} kHz", hz / 1000.0)
    } else if hz >= 10.0 {
        format!("{:.0} Hz", hz)
    } else {
        format!("{:.2} Hz", hz)
    }
}

fn fmt_db(gain: f32) -> String {
    if gain <= 1e-4 {
        "-inf dB".to_string()
    } else {
        format!("{:.1} dB", 20.0 * gain.log10())
    }
}

fn fmt_secs(s: f32) -> String {
    if s >= 1.0 {
        format!("{:.2} s", s)
    } else if s >= 0.01 {
        format!("{:.0} ms", s * 1000.0)
    } else {
        format!("{:.1} ms", s * 1000.0)
    }
}

/// Knob position → physical unit mappings.
pub mod map {
    use super::{CA_RULES, RND_THRESHOLD, SEEDS};

    /// Lowest pitch at the bottom of the PITCH fader.
    pub const PITCH_MIN_HZ: f32 = 20.0;
    /// Span of the PITCH fader in octaves.
    pub const PITCH_OCTAVES: f32 = 8.0;

    #[inline]
    pub fn bipolar(v: f32) -> f32 {
        (v - 0.5) * 2.0
    }

    /// Knob position split into `n` equal choices.
    pub fn choose(v: f32, n: usize) -> usize {
        ((v.clamp(0.0, 1.0) * n as f32) as usize).min(n - 1)
    }

    /// Knob position morphing through `n` named states: (lower state, fraction to the next).
    pub fn segment(v: f32, n: usize) -> (usize, f32) {
        let s = v.clamp(0.0, 1.0) * (n - 1) as f32;
        let i = (s as usize).min(n - 2);
        (i, s - i as f32)
    }

    pub fn fine_semitones(fine: f32) -> f32 {
        bipolar(fine) * 2.0
    }

    pub fn pitch_octaves(pitch: f32, fine: f32) -> f32 {
        PITCH_MIN_HZ.log2() + pitch * PITCH_OCTAVES + fine_semitones(fine) / 12.0
    }

    pub fn pitch_hz(pitch: f32, fine: f32) -> f32 {
        pitch_octaves(pitch, fine).exp2()
    }

    /// How long the object's event lasts (a pour, a spin, a flight …).
    pub fn time_seconds(time: f32) -> f32 {
        0.04 * 250f32.powf(time)
    }

    /// Pitch bend time constant. Higher knob = shorter.
    pub fn bend_seconds(speed: f32) -> f32 {
        1.5 * (0.002f32 / 1.5).powf(speed)
    }

    /// Bipolar bend depth in octaves: above centre the hit starts high and
    /// falls back to the pitch, below it starts low and rises.
    pub fn bend_octaves(depth: f32) -> f32 {
        let x = bipolar(depth);
        4.0 * x * x.abs().sqrt()
    }

    /// Frequency shift in Hz, bipolar with a dead zone at the centre.
    pub fn shift_hz(v: f32) -> f32 {
        let x = bipolar(v);
        let a = x.abs();
        if a < 0.02 {
            0.0
        } else {
            let a = (a - 0.02) / 0.98;
            (0.3 * 8000f32.powf(a)).copysign(x)
        }
    }

    /// Feedback gain of the shifter's delay loop.
    pub fn shift_spiral(v: f32) -> f32 {
        0.97 * v.clamp(0.0, 1.0).sqrt()
    }

    pub fn shift_lag_seconds(v: f32) -> f32 {
        0.004 * 150f32.powf(v)
    }

    pub fn cutoff_hz(cutoff: f32) -> f32 {
        20.0 * 1000f32.powf(cutoff)
    }

    pub fn reso_q(reso: f32) -> f32 {
        0.5 * 40f32.powf(reso)
    }

    pub fn reply_lag_seconds(v: f32) -> f32 {
        0.02 * 100f32.powf(v)
    }

    /// Velocity factor and pitch step (octaves) applied to each reply.
    pub fn reply_mood(v: f32) -> (f32, f32) {
        let m = bipolar(v);
        (0.8 * (0.6 * m).exp2(), m * 0.25)
    }

    pub fn level_gain(level: f32) -> f32 {
        level * level
    }

    pub fn sense_gain(sense: f32) -> f32 {
        0.25 * 16f32.powf(sense)
    }

    pub fn reverb_gain(amount: f32) -> f32 {
        amount * amount
    }

    /// Approximate RT60 of the plate.
    pub fn reverb_seconds(decay: f32) -> f32 {
        0.3 * 40f32.powf(decay)
    }

    /// Damping lowpass cutoff (input bandwidth and tank damping).
    pub fn reverb_tone_hz(tone: f32) -> f32 {
        1500.0 * 12f32.powf(tone)
    }

    pub fn reverb_predelay_seconds(predelay: f32) -> f32 {
        0.15 * predelay
    }

    // --- object macros ---------------------------------------------------

    /// A fixed seed, or `None` at the RND end of the knob (new seed per hit).
    pub fn seed(v: f32) -> Option<u32> {
        if v >= RND_THRESHOLD {
            None
        } else {
            Some(((v / RND_THRESHOLD) * SEEDS as f32) as u32)
        }
    }

    // BOUNCE
    pub fn ball_count(v: f32) -> usize {
        1 + (23.0 * v.clamp(0.0, 1.0).powf(1.6)).round() as usize
    }
    pub fn restitution(v: f32) -> f32 {
        0.25 + 0.7 * v.clamp(0.0, 1.0).sqrt()
    }
    /// Peak table acceleration in g (Γ). The ball leaves the table above 1.
    pub fn shake_gamma(v: f32) -> f32 {
        if v < 0.02 {
            0.0
        } else {
            4.5 * ((v - 0.02) / 0.98).powf(1.3)
        }
    }

    // COIN
    /// Exponent p of the wobble rate `Ω ∝ (t₀ − t)^-p`: smaller is a sharper
    /// singularity, so the knob turns it down.
    pub fn wobble_exponent(v: f32) -> f32 {
        1.6 - 1.25 * v.clamp(0.0, 1.0)
    }

    // GLOSSA
    pub fn tract_cm(v: f32) -> f32 {
        7.0 * (34.0f32 / 7.0).powf(v)
    }

    // POUR
    pub fn fill_octaves(v: f32) -> f32 {
        let x = bipolar(v);
        if x.abs() < 0.02 {
            0.0
        } else {
            3.2 * x * x.abs().sqrt()
        }
    }

    // AVIARY
    pub fn flock_size(v: f32) -> usize {
        1 + (11.0 * v.clamp(0.0, 1.0)).round() as usize
    }

    // MUSICBOX
    /// (sequence kind, parameter) or `None` for RND.
    pub fn tune(v: f32) -> Option<(u32, u32)> {
        seed(v).map(|s| (s % 5, s / 5 + (s % 5) * 3))
    }
    /// Starting numbers chosen for long, wandering hailstone paths.
    pub fn collatz_start(n: u32) -> u32 {
        const STARTS: [u32; 16] = [
            27, 7, 97, 41, 31, 63, 73, 54, 11, 39, 47, 71, 91, 103, 15, 9,
        ];
        STARTS[n as usize % STARTS.len()]
    }

    // ICE
    pub fn ice_distance_seconds(v: f32) -> f32 {
        0.008 * 60f32.powf(v)
    }
    pub fn ice_echoes(v: f32) -> usize {
        1 + (11.0 * v.clamp(0.0, 1.0)).round() as usize
    }

    // INSPIRAL
    pub fn mass_ratio(v: f32) -> f32 {
        1.0 + 11.0 * v * v
    }
    pub fn eccentricity(v: f32) -> f32 {
        0.85 * v.clamp(0.0, 1.0)
    }
    pub fn spin(v: f32) -> f32 {
        0.98 * v.clamp(0.0, 1.0)
    }

    // AUTOMATA
    pub fn rule(v: f32) -> Option<u8> {
        if v >= RND_THRESHOLD {
            None
        } else {
            let i = ((v / RND_THRESHOLD) * CA_RULES.len() as f32) as usize;
            Some(CA_RULES[i.min(CA_RULES.len() - 1)])
        }
    }
    pub fn ca_rate(v: f32) -> f32 {
        2.0 * 150f32.powf(v)
    }

    // PARADOX
    /// Glide in spacing-steps per second, bipolar (negative = falls).
    pub fn glide_rate(v: f32) -> f32 {
        let x = bipolar(v);
        if x.abs() < 0.02 {
            0.0
        } else {
            3.0 * x * x.abs()
        }
    }
    pub fn paradox_ratio(v: f32) -> f32 {
        // fifth … octave … tritave, through the golden ratio on the way.
        (0.585 + v.clamp(0.0, 1.0) * (1.585 - 0.585)).exp2()
    }

    // BALLOON
    /// Initial stretch ratio of the rubber.
    pub fn balloon_stretch(v: f32) -> f32 {
        1.25 + 2.5 * v.clamp(0.0, 1.0)
    }
}

/// A complete voice patch: one value per [`Param`], indexed by `Param::index`.
pub type VoicePatch = [f32; PARAM_COUNT];

pub fn default_patch() -> VoicePatch {
    let mut p = [0.0; PARAM_COUNT];
    for param in ALL_PARAMS {
        p[param.index()] = param.default_value();
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip() {
        for (i, p) in ALL_PARAMS.iter().enumerate() {
            assert_eq!(p.index(), i);
            assert_eq!(Param::from_id(p.id()), Some(*p));
        }
        assert_eq!(Param::from_id(PARAM_COUNT as u32), None);
    }

    #[test]
    fn sanitize_quantizes_choices() {
        assert_eq!(Param::Engine.sanitize(2.4), 2.0);
        assert_eq!(Param::Engine.sanitize(99.0), (ENGINE_COUNT - 1) as f32);
        assert_eq!(Param::Pitch.sanitize(1.5), 1.0);
        assert_eq!(
            Param::Pitch.sanitize(f32::NAN),
            Param::Pitch.default_value()
        );
    }

    #[test]
    fn mapping_ranges() {
        assert!((map::pitch_hz(0.0, 0.5) - 20.0).abs() < 0.01);
        assert!((map::pitch_hz(1.0, 0.5) - 5120.0).abs() < 1.0);
        assert!((map::time_seconds(1.0) - 10.0).abs() < 1e-3);
        assert_eq!(map::shift_hz(0.5), 0.0);
        assert!(map::shift_hz(1.0) > 2000.0 && map::shift_hz(0.0) < -2000.0);
        assert_eq!(map::bend_octaves(0.5), 0.0);
        assert_eq!(map::seed(1.0), None);
        assert_eq!(map::seed(0.0), Some(0));
        assert_eq!(map::seed(0.959), Some(SEEDS - 1));
        assert_eq!(map::ball_count(0.0), 1);
        assert_eq!(map::ball_count(1.0), 24);
        assert_eq!(map::rule(1.0), None);
        assert_eq!(map::rule(0.0), Some(90));
        assert!((map::paradox_ratio(0.415) - 2.0).abs() < 0.01);
        assert_eq!(map::segment(1.0, 4), (2, 1.0));
        assert_eq!(map::choose(1.0, 6), 5);
    }

    #[test]
    fn display_is_sane() {
        assert_eq!(Param::Engine.display(3.0), "GLOSSA");
        assert_eq!(Param::Pan.display(0.5), "C");
        assert_eq!(Param::Pitch.display(1.0), "5.12 kHz");
        assert_eq!(Param::BendDepth.display(0.5), "OFF");
        assert_eq!(Param::ShiftHz.display(0.5), "OFF");
        assert_eq!(Param::ReplyOdds.display(0.0), "NEVER");
        assert_eq!(Param::ReplyMood.display(0.5), "CALM");
        assert_eq!(Param::Medium.display(5.0), "RADIO");
        assert_eq!(Param::MacroX.display_for(ENGINE_GLOSSA, 1.0), "RND");
        assert_eq!(Param::MacroZ.display_for(ENGINE_COIN, 0.0), "COIN");
        assert_eq!(Param::MacroZ.display_for(ENGINE_COIN, 1.0), "BOWL");
        assert_eq!(
            Param::MacroX.display_for(ENGINE_MUSICBOX, 0.0),
            "COLLATZ 27"
        );
        assert_eq!(Param::MacroX.label_for(ENGINE_BALLOON), "SIZE");
        assert_eq!(Param::Time.label_for(ENGINE_BALLOON), "TIME");
    }

    #[test]
    fn every_macro_has_a_label_and_display() {
        for e in 0..ENGINE_COUNT as u32 {
            for slot in 0..3 {
                assert!(!macro_label(e, slot).is_empty());
                assert!(!macro_name(e, slot).is_empty());
                for v in [0.0, 0.01, 0.33, 0.5, 0.97, 1.0] {
                    assert!(!macro_display(e, slot, v).is_empty());
                }
            }
        }
    }

    #[test]
    fn locks_cover_exactly_the_mutable_params() {
        let fixed: Vec<_> = ALL_PARAMS.iter().filter(|p| !p.is_mutable()).collect();
        assert_eq!(fixed, [&Param::Level, &Param::Sense, &Param::Pan]);
    }
}
