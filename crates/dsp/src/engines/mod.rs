//! The twelve sound objects.
//!
//! Every object sees the same small interface. [`Ctx`] carries the per-block
//! control values (pitch after bend and whim, the three macros, TIME and how
//! long ago the channel was hit), `control` recomputes coefficients once per
//! control block, `trigger` hits the object and `tick` renders one sample.
//!
//! Unlike a drum voice, there is no shared amplitude envelope: each object is
//! a little simulation that decides for itself how a hit unfolds. A balloon
//! deflates, a coin spins down, water fills a bottle. Hitting an object that
//! is still busy usually *adds* to what it is doing (another ball, another
//! puff of air) rather than restarting it.

pub mod automata;
pub mod aviary;
pub mod balloon;
pub mod bounce;
pub mod coin;
pub mod creak;
pub mod glossa;
pub mod ice;
pub mod inspiral;
pub mod musicbox;
pub mod paradox;
pub mod pour;

use crate::util::Noise;

/// Control values for one block.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    /// Rate `tick` is called at.
    pub fs: f32,
    /// Pitch in Hz, after bend, whim and reply offsets.
    pub hz: f32,
    /// Object macros, `0..=1`, after whim.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// TIME in seconds: how long the object's event lasts.
    pub time_s: f32,
    /// Velocity of the latest hit after SENSE, `0..=1`.
    pub velocity: f32,
}

impl Ctx {
    pub fn new(fs: f32) -> Self {
        Self {
            fs,
            hz: 200.0,
            x: 0.5,
            y: 0.5,
            z: 0.5,
            time_s: 0.5,
            velocity: 1.0,
        }
    }
}

/// Output trim per object in dB, from `examples/calibrate.rs`, so that
/// switching objects doesn't jump in level.
const TRIM_DB: [f32; 12] = [
    -5.0, -8.0, 2.0, 1.0, 0.0, 5.0, -3.0, -5.0, 3.0, 6.0, 7.0, 1.0,
];

/// Linear output trim of an object.
pub fn trim(engine: u32) -> f32 {
    (TRIM_DB[(engine as usize).min(TRIM_DB.len() - 1)] / 20.0 * core::f32::consts::LOG2_10).exp2()
}

macro_rules! engines {
    ($($idx:literal => $field:ident : $ty:ty),* $(,)?) => {
        /// All objects, each with its own state. Only the selected one runs.
        #[derive(Clone, Debug)]
        pub struct Engines {
            $(pub $field: $ty,)*
        }

        impl Engines {
            pub fn new(fs: f32) -> Self {
                Self { $($field: <$ty>::new(fs),)* }
            }

            pub fn reset(&mut self, engine: u32) {
                match engine {
                    $($idx => self.$field.reset(),)*
                    _ => {}
                }
            }

            pub fn control(&mut self, engine: u32, c: &Ctx) {
                match engine {
                    $($idx => self.$field.control(c),)*
                    _ => {}
                }
            }

            pub fn trigger(&mut self, engine: u32, c: &Ctx, rng: &mut Noise) {
                match engine {
                    $($idx => self.$field.trigger(c, rng),)*
                    _ => {}
                }
            }

            #[inline]
            pub fn tick(&mut self, engine: u32, c: &Ctx, rng: &mut Noise) -> f32 {
                match engine {
                    $($idx => self.$field.tick(c, rng),)*
                    _ => 0.0,
                }
            }
        }
    };
}

engines! {
    0 => bounce: bounce::Bounce,
    1 => coin: coin::Coin,
    2 => creak: creak::Creak,
    3 => glossa: glossa::Glossa,
    4 => pour: pour::Pour,
    5 => aviary: aviary::Aviary,
    6 => musicbox: musicbox::MusicBox,
    7 => ice: ice::Ice,
    8 => inspiral: inspiral::Inspiral,
    9 => automata: automata::Automata,
    10 => paradox: paradox::Paradox,
    11 => balloon: balloon::Balloon,
}
