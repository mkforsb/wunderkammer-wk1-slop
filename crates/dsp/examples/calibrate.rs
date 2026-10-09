//! Loudness survey: every object across pitch, TIME and macro settings, so
//! the objects can be balanced against each other.
//!
//! ```sh
//! cargo run --release -p wunderkammer-dsp --example calibrate
//! ```

use wunderkammer_dsp::params::{ENGINE_COUNT, ENGINE_NAMES};
use wunderkammer_dsp::{Param, voice::Voice};

fn main() {
    let fs = 48_000.0;
    // With an object name, list every setting for that object.
    let detail = std::env::args()
        .nth(1)
        .and_then(|n| ENGINE_NAMES.iter().position(|e| e.eq_ignore_ascii_case(&n)));
    println!(
        "{:<9} {:>7} {:>7} {:>7} {:>7}   (peak and rms of the loudest 100 ms, dBFS)",
        "object", "peak lo", "peak hi", "rms med", "rms max"
    );
    for engine in (0..ENGINE_COUNT).filter(|&e| detail.is_none_or(|d| d == e)) {
        let mut peaks = Vec::new();
        let mut rmss = Vec::new();
        for pitch in [0.25, 0.45, 0.65] {
            for time in [0.2, 0.5] {
                for m in [0.0, 0.33, 0.66, 1.0] {
                    let mut v = Voice::new(fs, 1);
                    v.set_param(Param::Engine, engine as f32);
                    v.set_param(Param::Pitch, pitch);
                    v.set_param(Param::Time, time);
                    v.set_param(Param::MacroX, m);
                    v.set_param(Param::MacroY, 1.0 - m);
                    v.set_param(Param::MacroZ, (m + 0.5) % 1.0);
                    v.snap_params();
                    v.trigger(0.9);
                    let n = (fs * (map_time(time) + 1.0)) as usize;
                    let out: Vec<f32> = (0..n).map(|_| v.tick()).collect();
                    let peak = out.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
                    let rms = out
                        .chunks(4_800)
                        .map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt())
                        .fold(0.0f32, f32::max);
                    if detail.is_some() {
                        println!(
                            "  pitch {pitch:.2} time {time:.1} xyz {m:.2} {:.2} {:.2}: peak {:>6.1} rms {:>6.1}",
                            1.0 - m,
                            (m + 0.5) % 1.0,
                            20.0 * peak.max(1e-6).log10(),
                            20.0 * rms.max(1e-6).log10()
                        );
                    }
                    peaks.push(peak);
                    rmss.push(rms);
                }
            }
        }
        peaks.sort_by(|a, b| a.partial_cmp(b).unwrap());
        rmss.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let db = |x: f32| 20.0 * x.max(1e-6).log10();
        println!(
            "{:<9} {:>7.1} {:>7.1} {:>7.1} {:>7.1}",
            ENGINE_NAMES[engine],
            db(peaks[0]),
            db(*peaks.last().unwrap()),
            db(rmss[rmss.len() / 2]),
            db(*rmss.last().unwrap())
        );
    }
}

fn map_time(t: f32) -> f32 {
    wunderkammer_dsp::params::map::time_seconds(t)
}
