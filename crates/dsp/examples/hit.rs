//! Render hits of one channel with arbitrary settings, for tuning by ear
//! (and by eye: see `tools/spectrogram.py`).
//!
//! ```sh
//! cargo run --release -p wunderkammer-dsp --example hit -- out.wav \
//!     engine=BALLOON pitch=0.5 x=0.7 y=0.2 z=0.8 time=0.6 hits=2 seconds=3
//! ```
//!
//! Keys are parameter names (case-insensitive `Param` names, or `x`/`y`/`z`/
//! `engine` by name), plus `hits`, `seconds` (per hit), `velocity`, `preset`,
//! and `ch2.<param>`/`reply` to involve the second channel.

use wunderkammer_dsp::params::ENGINE_NAMES;
use wunderkammer_dsp::presets::PRESETS;
use wunderkammer_dsp::{ALL_PARAMS, Engine, Param};

const SR: u32 = 48_000;

fn write_wav(path: &str, left: &[f32], right: &[f32]) -> std::io::Result<()> {
    use std::io::Write;
    let frames = left.len() as u32;
    let data_len = frames * 4;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&SR.to_le_bytes())?;
    f.write_all(&(SR * 4).to_le_bytes())?;
    f.write_all(&4u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for (l, r) in left.iter().zip(right) {
        for s in [l, r] {
            f.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())?;
        }
    }
    f.flush()
}

fn param_named(name: &str) -> Option<Param> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "x" => return Some(Param::MacroX),
        "y" => return Some(Param::MacroY),
        "z" => return Some(Param::MacroZ),
        _ => {}
    }
    ALL_PARAMS
        .iter()
        .copied()
        .find(|p| format!("{p:?}").to_ascii_lowercase() == lower)
}

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("output path");
    let mut engine = Engine::new(SR as f32);
    for v in 0..2 {
        engine.set_param(v, Param::ReverbMix, 0.0);
    }
    let (mut hits, mut seconds, mut velocity) = (1usize, 2.0f32, 1.0f32);
    for arg in args {
        let (key, value) = arg.split_once('=').expect("key=value");
        let (voice, key) = match key.strip_prefix("ch2.") {
            Some(k) => (1, k),
            None => (0, key),
        };
        match key {
            "hits" => hits = value.parse().unwrap(),
            "seconds" => seconds = value.parse().unwrap(),
            "velocity" => velocity = value.parse().unwrap(),
            "preset" => {
                let p = PRESETS
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(value))
                    .expect("no such preset");
                engine.load_patch(voice, &p.patch());
            }
            _ => {
                let p = param_named(key).unwrap_or_else(|| panic!("unknown param {key}"));
                let v = if p == Param::Engine {
                    ENGINE_NAMES
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(value))
                        .map(|i| i as f32)
                        .unwrap_or_else(|| value.parse().unwrap())
                } else {
                    value.parse().unwrap()
                };
                engine.set_param(voice, p, v);
            }
        }
    }
    engine.snap_params();
    let mut left = Vec::new();
    let mut right = Vec::new();
    for _ in 0..hits {
        engine.trigger(0, velocity);
        let (l, r) = engine.render_vec((SR as f32 * seconds) as usize);
        left.extend(l);
        right.extend(r);
    }
    let peak = left
        .iter()
        .chain(&right)
        .fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt();
    let win = SR as usize / 10;
    let env: Vec<String> = left
        .chunks(win)
        .map(|c| {
            let r = rms(c);
            if r < 1e-5 {
                "  -".into()
            } else {
                format!("{:>3.0}", 20.0 * r.log10())
            }
        })
        .collect();
    println!("peak {peak:.3}  rms {:.4}  -> {out}", rms(&left));
    println!("dB/100ms: {}", env.join(" "));
    write_wav(&out, &left, &right)
}
