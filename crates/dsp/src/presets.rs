//! Factory voice presets, grouped by object.

use crate::params::{
    ENGINE_AUTOMATA, ENGINE_AVIARY, ENGINE_BALLOON, ENGINE_BOUNCE, ENGINE_COIN, ENGINE_CREAK,
    ENGINE_GLOSSA, ENGINE_ICE, ENGINE_INSPIRAL, ENGINE_MUSICBOX, ENGINE_PARADOX, ENGINE_POUR,
    FILTER_BP, FILTER_HP, MEDIUM_HORN, MEDIUM_RADIO, MEDIUM_TIN_CAN, MEDIUM_TUBE, MEDIUM_WALL,
    MEDIUM_WATER, Param, VoicePatch, default_patch,
};

pub struct Preset {
    pub name: &'static str,
    pub values: &'static [(Param, f32)],
}

impl Preset {
    /// Full patch: defaults overlaid with this preset's values.
    pub fn patch(&self) -> VoicePatch {
        let mut p = default_patch();
        for &(param, v) in self.values {
            p[param.index()] = param.sanitize(v);
        }
        p
    }

    /// The object this preset uses (for grouping in the panel).
    pub fn engine(&self) -> u32 {
        self.patch()[Param::Engine.index()] as u32
    }
}

const BOUNCE: f32 = ENGINE_BOUNCE as f32;
const COIN: f32 = ENGINE_COIN as f32;
const CREAK: f32 = ENGINE_CREAK as f32;
const GLOSSA: f32 = ENGINE_GLOSSA as f32;
const POUR: f32 = ENGINE_POUR as f32;
const AVIARY: f32 = ENGINE_AVIARY as f32;
const MUSICBOX: f32 = ENGINE_MUSICBOX as f32;
const ICE: f32 = ENGINE_ICE as f32;
const INSPIRAL: f32 = ENGINE_INSPIRAL as f32;
const AUTOMATA: f32 = ENGINE_AUTOMATA as f32;
const PARADOX: f32 = ENGINE_PARADOX as f32;
const BALLOON: f32 = ENGINE_BALLOON as f32;

/// Knob position of fixed seed `n` (GLOSSA words, AVIARY species, MUSICBOX tunes).
const fn seed(n: u32) -> f32 {
    (n as f32 + 0.5) / crate::params::SEEDS as f32 * crate::params::RND_THRESHOLD
}

/// Knob position of rule `i` in [`crate::params::CA_RULES`].
const fn rule(i: u32) -> f32 {
    (i as f32 + 0.5) / 16.0 * crate::params::RND_THRESHOLD
}

const RND: f32 = 1.0;

use Param::*;

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Init",
        values: &[],
    },
    // --- BOUNCE ----------------------------------------------------------
    Preset {
        name: "Ping-Pong Accelerando",
        values: &[
            (Engine, BOUNCE),
            (Pitch, 0.45),
            (MacroX, 0.0),
            (MacroY, 0.8),
            (MacroZ, 0.0),
            (Time, 0.5),
            (ReverbMix, 0.25),
        ],
    },
    Preset {
        name: "Popcorn Snare",
        values: &[
            (Engine, BOUNCE),
            (Pitch, 0.38),
            (MacroX, 0.75),
            (MacroY, 0.55),
            (MacroZ, 0.0),
            (Time, 0.35),
            (Cutoff, 0.9),
        ],
    },
    Preset {
        name: "Marbles in a Spin Cycle",
        values: &[
            (Engine, BOUNCE),
            (Pitch, 0.42),
            (MacroX, 0.25),
            (MacroY, 0.6),
            (MacroZ, 0.55),
            (Time, 0.75),
            (ReverbMix, 0.2),
        ],
    },
    Preset {
        name: "Period Doubling",
        values: &[
            (Engine, BOUNCE),
            (Pitch, 0.5),
            (MacroX, 0.0),
            (MacroY, 0.88),
            (MacroZ, 0.75),
            (Time, 0.8),
            (ShiftHz, 0.53),
            (ShiftSpiral, 0.3),
            (ShiftLag, 0.45),
        ],
    },
    Preset {
        name: "Neighbours' Ping-Pong",
        values: &[
            (Engine, BOUNCE),
            (Pitch, 0.45),
            (MacroX, 0.08),
            (MacroY, 0.8),
            (Time, 0.5),
            (Medium, MEDIUM_WALL as f32),
            (MediumDepth, 0.7),
            (ReverbMix, 0.35),
            (ReverbDecay, 0.3),
        ],
    },
    // --- COIN ------------------------------------------------------------
    Preset {
        name: "Euler's Disk",
        values: &[
            (Engine, COIN),
            (Pitch, 0.72),
            (MacroX, 0.75),
            (MacroY, 0.15),
            (MacroZ, 0.0),
            (Time, 0.68),
            (ReverbMix, 0.2),
        ],
    },
    Preset {
        name: "Pot Lid Finale",
        values: &[
            (Engine, COIN),
            (Pitch, 0.42),
            (MacroX, 0.55),
            (MacroY, 0.5),
            (MacroZ, 0.33),
            (Time, 0.65),
            (ReverbMix, 0.4),
        ],
    },
    Preset {
        name: "Singing Bowl Spin",
        values: &[
            (Engine, COIN),
            (Pitch, 0.5),
            (MacroX, 0.3),
            (MacroY, 0.25),
            (MacroZ, 1.0),
            (Time, 0.62),
            (ReverbMix, 0.45),
            (ReverbDecay, 0.7),
        ],
    },
    Preset {
        name: "Barber-Pole Penny",
        values: &[
            (Engine, COIN),
            (Pitch, 0.68),
            (MacroX, 0.7),
            (MacroY, 0.3),
            (MacroZ, 0.1),
            (Time, 0.6),
            (ShiftHz, 0.62),
            (ShiftSpiral, 0.75),
            (ShiftLag, 0.35),
            (ReverbMix, 0.25),
        ],
    },
    // --- CREAK -----------------------------------------------------------
    Preset {
        name: "Haunted Hinge",
        values: &[
            (Engine, CREAK),
            (Pitch, 0.42),
            (MacroX, 0.85),
            (MacroY, 0.0),
            (MacroZ, 0.0),
            (Time, 0.7),
            (ReverbMix, 0.5),
            (ReverbDecay, 0.8),
            (ReverbTone, 0.3),
        ],
    },
    Preset {
        name: "Balloon Squeak",
        values: &[
            (Engine, CREAK),
            (Pitch, 0.52),
            (MacroX, 0.6),
            (MacroY, 0.33),
            (MacroZ, 0.33),
            (Time, 0.5),
        ],
    },
    Preset {
        name: "Wet Finger on Glass",
        values: &[
            (Engine, CREAK),
            (Pitch, 0.62),
            (MacroX, 0.45),
            (MacroY, 0.0),
            (MacroZ, 0.66),
            (Time, 0.75),
            (ReverbMix, 0.4),
            (ReverbDecay, 0.6),
        ],
    },
    Preset {
        name: "Juddering Cello",
        values: &[
            (Engine, CREAK),
            (Pitch, 0.3),
            (MacroX, 0.7),
            (MacroY, 1.0),
            (MacroZ, 1.0),
            (Time, 0.62),
            (ReverbMix, 0.3),
        ],
    },
    // --- GLOSSA ----------------------------------------------------------
    Preset {
        name: "Talking Drum",
        values: &[
            (Engine, GLOSSA),
            (Pitch, 0.35),
            (MacroX, seed(5)),
            (MacroY, 0.0),
            (MacroZ, 0.45),
            (Time, 0.45),
        ],
    },
    Preset {
        name: "Mumbling Imp",
        values: &[
            (Engine, GLOSSA),
            (Pitch, 0.55),
            (MacroX, RND),
            (MacroY, 0.33),
            (MacroZ, 0.1),
            (Time, 0.48),
            (Whim, 0.3),
        ],
    },
    Preset {
        name: "The Ogre Has a Question",
        values: &[
            (Engine, GLOSSA),
            (Pitch, 0.22),
            (MacroX, seed(14)),
            (MacroY, 0.33),
            (MacroZ, 0.95),
            (Time, 0.6),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Jaw Harp Boing",
        values: &[
            (Engine, GLOSSA),
            (Pitch, 0.3),
            (MacroX, seed(26)),
            (MacroY, 0.66),
            (MacroZ, 0.5),
            (Time, 0.55),
        ],
    },
    Preset {
        name: "Ghost on the Wireless",
        values: &[
            (Engine, GLOSSA),
            (Pitch, 0.4),
            (MacroX, RND),
            (MacroY, 1.0),
            (MacroZ, 0.5),
            (Time, 0.6),
            (Medium, MEDIUM_RADIO as f32),
            (MediumDepth, 0.8),
            (ReverbMix, 0.5),
            (ReverbDecay, 0.8),
        ],
    },
    Preset {
        name: "Tin-Can Gossip",
        values: &[
            (Engine, GLOSSA),
            (Pitch, 0.42),
            (MacroX, RND),
            (MacroY, 0.4),
            (MacroZ, 0.3),
            (Time, 0.5),
            (Medium, MEDIUM_TIN_CAN as f32),
            (MediumDepth, 0.8),
            (ReplyOdds, 0.85),
            (ReplyLag, 0.55),
            (ReplyMood, 0.62),
        ],
    },
    // --- POUR ------------------------------------------------------------
    Preset {
        name: "Filling a Bottle",
        values: &[
            (Engine, POUR),
            (Pitch, 0.36),
            (MacroX, 0.95),
            (MacroY, 0.3),
            (MacroZ, 0.0),
            (Time, 0.66),
        ],
    },
    Preset {
        name: "Champagne",
        values: &[
            (Engine, POUR),
            (Pitch, 0.45),
            (MacroX, 0.85),
            (MacroY, 0.45),
            (MacroZ, 0.85),
            (Time, 0.6),
            (ReverbMix, 0.25),
        ],
    },
    Preset {
        name: "Bathtub Drain",
        values: &[
            (Engine, POUR),
            (Pitch, 0.5),
            (MacroX, 0.05),
            (MacroY, 0.85),
            (MacroZ, 0.0),
            (Time, 0.65),
            (ReverbMix, 0.3),
            (ReverbDecay, 0.3),
        ],
    },
    Preset {
        name: "Glug Down the Tube",
        values: &[
            (Engine, POUR),
            (Pitch, 0.42),
            (MacroX, 0.2),
            (MacroY, 1.0),
            (MacroZ, 0.2),
            (Time, 0.55),
            (Medium, MEDIUM_TUBE as f32),
            (MediumDepth, 0.6),
        ],
    },
    // --- AVIARY ----------------------------------------------------------
    Preset {
        name: "Startled Sparrows",
        values: &[
            (Engine, AVIARY),
            (Pitch, 0.42),
            (MacroX, seed(11)),
            (MacroY, 0.6),
            (MacroZ, 0.6),
            (Time, 0.65),
            (ReverbMix, 0.25),
        ],
    },
    Preset {
        name: "Dawn Chorus",
        values: &[
            (Engine, AVIARY),
            (Pitch, 0.38),
            (MacroX, RND),
            (MacroY, 1.0),
            (MacroZ, 0.4),
            (Time, 0.8),
            (ReverbMix, 0.4),
            (ReverbDecay, 0.7),
        ],
    },
    Preset {
        name: "Pond at Night",
        values: &[
            (Engine, AVIARY),
            (Pitch, 0.3),
            (MacroX, seed(2)),
            (MacroY, 0.75),
            (MacroZ, 0.3),
            (Time, 0.85),
            (ReverbMix, 0.35),
            (ReverbDecay, 0.65),
            (ReverbTone, 0.35),
        ],
    },
    Preset {
        name: "Gramophone Aviary",
        values: &[
            (Engine, AVIARY),
            (Pitch, 0.36),
            (MacroX, seed(30)),
            (MacroY, 0.45),
            (MacroZ, 0.5),
            (Time, 0.7),
            (Medium, MEDIUM_HORN as f32),
            (MediumDepth, 0.85),
        ],
    },
    // --- MUSICBOX --------------------------------------------------------
    Preset {
        name: "Collatz Lullaby",
        values: &[
            (Engine, MUSICBOX),
            (Pitch, 0.62),
            (MacroX, seed(0)),
            (MacroY, 0.0),
            (MacroZ, 0.05),
            (Time, 0.85),
            (ReverbMix, 0.4),
            (ReverbDecay, 0.6),
        ],
    },
    Preset {
        name: "Recaman Waltz",
        values: &[
            (Engine, MUSICBOX),
            (Pitch, 0.6),
            (MacroX, seed(1)),
            (MacroY, 0.25),
            (MacroZ, 0.15),
            (Time, 0.8),
            (ReverbMix, 0.35),
        ],
    },
    Preset {
        name: "Rusty Pi",
        values: &[
            (Engine, MUSICBOX),
            (Pitch, 0.58),
            (MacroX, seed(2)),
            (MacroY, 0.4),
            (MacroZ, 0.85),
            (Time, 0.75),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Bohlen-Pierce Infinity",
        values: &[
            (Engine, MUSICBOX),
            (Pitch, 0.55),
            (MacroX, seed(4)),
            (MacroY, 1.0),
            (MacroZ, 0.2),
            (Time, 0.8),
            (ReverbMix, 0.35),
        ],
    },
    Preset {
        name: "Numbers Station",
        values: &[
            (Engine, MUSICBOX),
            (Pitch, 0.52),
            (MacroX, seed(7)),
            (MacroY, 0.5),
            (MacroZ, 0.4),
            (Time, 0.85),
            (Medium, MEDIUM_RADIO as f32),
            (MediumDepth, 0.9),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Sunken Music Box",
        values: &[
            (Engine, MUSICBOX),
            (Pitch, 0.64),
            (MacroX, seed(10)),
            (MacroY, 0.25),
            (MacroZ, 0.3),
            (Time, 0.8),
            (Medium, MEDIUM_WATER as f32),
            (MediumDepth, 0.75),
            (ReverbMix, 0.4),
            (ReverbTone, 0.2),
        ],
    },
    // --- ICE -------------------------------------------------------------
    Preset {
        name: "Thin Ice",
        values: &[
            (Engine, ICE),
            (Pitch, 0.72),
            (MacroX, 0.45),
            (MacroY, 0.5),
            (MacroZ, 0.35),
            (Time, 0.7),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Guy-Wire Blaster",
        values: &[
            (Engine, ICE),
            (Pitch, 0.78),
            (MacroX, 0.2),
            (MacroY, 0.0),
            (MacroZ, 0.0),
            (Time, 0.5),
            (Cutoff, 0.85),
        ],
    },
    Preset {
        name: "Frozen Lake at Dusk",
        values: &[
            (Engine, ICE),
            (Pitch, 0.68),
            (MacroX, 0.75),
            (MacroY, 0.9),
            (MacroZ, 0.7),
            (Time, 0.85),
            (ReverbMix, 0.5),
            (ReverbDecay, 0.85),
        ],
    },
    // --- INSPIRAL --------------------------------------------------------
    Preset {
        name: "GW150914",
        values: &[
            (Engine, INSPIRAL),
            (Pitch, 0.48),
            (MacroX, 0.1),
            (MacroY, 0.0),
            (MacroZ, 0.35),
            (Time, 0.6),
            (ReverbMix, 0.25),
        ],
    },
    Preset {
        name: "Eccentric Binary",
        values: &[
            (Engine, INSPIRAL),
            (Pitch, 0.52),
            (MacroX, 0.4),
            (MacroY, 0.95),
            (MacroZ, 0.2),
            (Time, 0.68),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Kerr Bell",
        values: &[
            (Engine, INSPIRAL),
            (Pitch, 0.6),
            (MacroX, 0.8),
            (MacroY, 0.3),
            (MacroZ, 0.98),
            (Time, 0.55),
            (ReverbMix, 0.4),
            (ReverbDecay, 0.65),
        ],
    },
    // --- AUTOMATA --------------------------------------------------------
    Preset {
        name: "Sierpinski Arpeggio",
        values: &[
            (Engine, AUTOMATA),
            (Pitch, 0.3),
            (MacroX, rule(0)),
            (MacroY, 0.4),
            (MacroZ, 0.0),
            (Time, 0.8),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Rule 30 Buzz",
        values: &[
            (Engine, AUTOMATA),
            (Pitch, 0.32),
            (MacroX, rule(1)),
            (MacroY, 0.85),
            (MacroZ, 0.5),
            (Time, 0.6),
        ],
    },
    Preset {
        name: "Pentatonic Petri Dish",
        values: &[
            (Engine, AUTOMATA),
            (Pitch, 0.38),
            (MacroX, rule(6)),
            (MacroY, 0.45),
            (MacroZ, 1.0),
            (Time, 0.8),
            (ReverbMix, 0.4),
        ],
    },
    // --- PARADOX ---------------------------------------------------------
    Preset {
        name: "Endless Fall",
        values: &[
            (Engine, PARADOX),
            (Pitch, 0.45),
            (MacroX, 0.0),
            (MacroY, 0.0),
            (MacroZ, 0.415),
            (Time, 0.85),
            (ReverbMix, 0.4),
        ],
    },
    Preset {
        name: "Risset's Staircase",
        values: &[
            (Engine, PARADOX),
            (Pitch, 0.5),
            (MacroX, 0.15),
            (MacroY, 0.85),
            (MacroZ, 0.415),
            (Time, 0.9),
            (ReverbMix, 0.3),
        ],
    },
    Preset {
        name: "Golden Ascent",
        values: &[
            (Engine, PARADOX),
            (Pitch, 0.55),
            (MacroX, 0.9),
            (MacroY, 0.2),
            (MacroZ, 0.11),
            (Time, 0.8),
            (ReverbMix, 0.35),
            (FilterType, FILTER_BP as f32),
            (Cutoff, 0.6),
            (Reso, 0.15),
        ],
    },
    // --- BALLOON ---------------------------------------------------------
    Preset {
        name: "Let It Go",
        values: &[
            (Engine, BALLOON),
            (Pitch, 0.6),
            (MacroX, 0.75),
            (MacroY, 0.35),
            (MacroZ, 0.85),
            (Time, 0.75),
            (ReverbMix, 0.25),
            (ReverbDecay, 0.3),
        ],
    },
    Preset {
        name: "Whoopee",
        values: &[
            (Engine, BALLOON),
            (Pitch, 0.35),
            (MacroX, 0.3),
            (MacroY, 0.85),
            (MacroZ, 0.0),
            (Time, 0.55),
        ],
    },
    Preset {
        name: "Squealer",
        values: &[
            (Engine, BALLOON),
            (Pitch, 0.7),
            (MacroX, 0.9),
            (MacroY, 0.1),
            (MacroZ, 0.3),
            (Time, 0.7),
            (FilterType, FILTER_HP as f32),
            (Cutoff, 0.25),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ALL_PARAMS;

    #[test]
    fn names_are_unique_and_patches_sane() {
        for (i, p) in PRESETS.iter().enumerate() {
            assert!(
                PRESETS[i + 1..].iter().all(|q| q.name != p.name),
                "{}",
                p.name
            );
            let patch = p.patch();
            for param in ALL_PARAMS {
                assert_eq!(patch[param.index()], param.sanitize(patch[param.index()]));
            }
        }
    }

    #[test]
    fn every_object_has_presets() {
        for e in 0..crate::params::ENGINE_COUNT as u32 {
            assert!(
                PRESETS.iter().filter(|p| p.engine() == e).count() >= 3,
                "{e}"
            );
        }
    }

    #[test]
    fn every_preset_makes_sound() {
        use crate::voice::Voice;
        let fs = 24_000.0;
        for p in PRESETS {
            let mut v = Voice::new(fs, 1);
            let patch = p.patch();
            for param in ALL_PARAMS {
                v.set_param(param, patch[param.index()]);
            }
            v.snap_params();
            v.trigger(0.9);
            let out: Vec<f32> = (0..fs as usize * 3).map(|_| v.tick()).collect();
            let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(out.iter().all(|x| x.is_finite()), "{}", p.name);
            assert!(peak > 0.02 && peak < 3.0, "{}: {peak}", p.name);
        }
    }
}
