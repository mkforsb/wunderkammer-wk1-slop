//! Wunderkammer DSP: a two-channel cabinet of improbable sound objects.
//!
//! Each channel hosts one of twelve simulations (balls on a drumhead, a
//! spinning coin, stick-slip friction, a talking tube, a filling bottle, a
//! flock of birds, a number-theory music box, dispersive ice, merging black
//! holes, a cellular automaton, auditory illusions and a deflating balloon),
//! followed by a frequency-shifter spiral, a tone filter, the medium it is
//! heard through and a per-channel plate reverb. The channels can answer
//! each other's hits.
//!
//! The crate has no dependencies and no platform code so it can be compiled to
//! a standalone wasm module for an AudioWorklet as well as used natively.

// The objects loop over several parallel per-mode/per-cell arrays at once;
// index loops read better there than zipped iterators.
#![allow(clippy::needless_range_loop)]

pub mod engine;
pub mod engines;
pub mod fx;
pub mod params;
pub mod presets;
pub mod preview;
pub mod reverb;
pub mod util;
pub mod voice;

pub use engine::{Engine, VOICES};
pub use params::{ALL_PARAMS, PARAM_COUNT, Param, ParamKind, VoicePatch};
