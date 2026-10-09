//! Real-time factor of the engine with both channels hitting every object,
//! through a busy chain (SHIFT spiral, a THROUGH medium and the reverb).
//!
//! ```sh
//! cargo run -p wunderkammer-dsp --release --example bench
//! ```

use std::time::Instant;
use wunderkammer_dsp::params::{ENGINE_NAMES, MEDIUM_HORN};
use wunderkammer_dsp::{Engine, Param};

fn main() {
    let sr = 48_000.0;
    let seconds = 20.0;
    let mut worst = f64::MAX;
    for (engine, name) in ENGINE_NAMES.iter().enumerate() {
        let mut e = Engine::new(sr);
        for v in 0..2 {
            e.set_param(v, Param::Engine, engine as f32);
            e.set_param(v, Param::ReverbMix, 0.5);
            e.set_param(v, Param::Time, 0.75);
            // Heavy corners: many balls, big flocks, long tracts …
            e.set_param(v, Param::MacroX, 1.0);
            e.set_param(v, Param::MacroY, 0.8);
            e.set_param(v, Param::MacroZ, 1.0);
            e.set_param(v, Param::ShiftHz, 0.7);
            e.set_param(v, Param::ShiftSpiral, 0.5);
            e.set_param(v, Param::Medium, MEDIUM_HORN as f32);
        }
        e.snap_params();
        let mut l = [0.0f32; 128];
        let mut r = [0.0f32; 128];
        let blocks = (sr * seconds / 128.0) as usize;
        let start = Instant::now();
        for b in 0..blocks {
            if b % 150 == 0 {
                e.trigger(0, 1.0);
                e.trigger(1, 0.7);
            }
            e.render(&mut l, &mut r);
        }
        let rtf = seconds as f64 / start.elapsed().as_secs_f64();
        worst = worst.min(rtf);
        println!("{:<9} {rtf:>6.0}x real time (both channels)", name);
    }
    println!("worst case: {worst:.0}x");
}
