//! The panel: two cabinets, each with an object picker, a hit preview, the
//! object's macro knobs and the shared bend/shift/tone/medium/reply/output/
//! reverb sections.
//!
//! Ctrl-click (or ⌘-click) any control that RND and MUTATE would change to
//! lock it: it keeps its value until unlocked.

use dioxus::prelude::*;
use wunderkammer_dsp::params::{ENGINE_BLURBS, ENGINE_COUNT, ENGINE_NAMES, ParamKind, map};
use wunderkammer_dsp::presets::PRESETS;
use wunderkammer_dsp::{Param, VOICES, VoicePatch};

use crate::audio::AudioStatus;
use crate::state::{Drag, Synth, Target};

const STYLE: &str = include_str!("../assets/style.css");

/// Pixels of vertical drag for a knob's full travel.
const KNOB_SPAN_PX: f64 = 200.0;
/// Fader geometry; keep in sync with `.fader-track` / `.fader-cap` in the CSS.
const FADER_TRACK_PX: f64 = 150.0;
const FADER_CAP_PX: f64 = 24.0;
const FADER_LEDS: usize = 10;
/// Toggle switch geometry; keep in sync with `.switch-lever`.
const SWITCH_LEVER_PX: f64 = 14.0;
const SWITCH_LEVER_INSET_PX: f64 = 2.0;
/// Vertical space per switch position.
const SWITCH_STEP_PX: f64 = 15.0;
/// Trigger pad height; keep in sync with `.pad` in the CSS.
const PAD_PX: f64 = 96.0;
/// Hit preview: columns, render rate and the velocity it is drawn at.
const SCOPE_COLUMNS: usize = 180;
const SCOPE_FS: f32 = 16_000.0;
const SCOPE_VELOCITY: f32 = 0.8;
const SCOPE_MAX_S: f32 = 4.0;

/// Cabinet names.
const ROMAN: [&str; VOICES] = ["I", "II"];

/// Keyboard map: (key, channel, velocity).
const KEYMAP: &[(Code, usize, f32)] = &[
    (Code::KeyZ, 0, 0.3),
    (Code::KeyX, 0, 0.55),
    (Code::KeyC, 0, 0.8),
    (Code::KeyV, 0, 1.0),
    (Code::KeyN, 1, 0.3),
    (Code::KeyM, 1, 0.55),
    (Code::Comma, 1, 0.8),
    (Code::Period, 1, 1.0),
];

fn lock_modifier(m: Modifiers) -> bool {
    m.ctrl() || m.meta()
}

#[component]
pub fn App() -> Element {
    let synth = use_context_provider(Synth::new);
    let drag = synth.drag;

    let s_move = synth.clone();
    let s_up = synth.clone();
    let s_down = synth.clone();
    let s_key = synth.clone();
    let end_drag = move || {
        let mut drag = s_up.drag;
        if drag.peek().is_some() {
            drag.set(None);
            s_up.save();
        }
    };
    let end_drag_leave = end_drag.clone();
    let end_drag_up = end_drag;

    rsx! {
        style { {STYLE} }
        // Shared paint for the knobs.
        svg { class: "defs", width: "0", height: "0",
            defs {
                radialGradient { id: "wk-brass", cx: "38%", cy: "32%", r: "75%",
                    stop { offset: "0%", stop_color: "#f9e7b8" }
                    stop { offset: "38%", stop_color: "#c99a45" }
                    stop { offset: "100%", stop_color: "#5c3c14" }
                }
                radialGradient { id: "wk-ebony", cx: "40%", cy: "35%", r: "70%",
                    stop { offset: "0%", stop_color: "#3a2a22" }
                    stop { offset: "100%", stop_color: "#0b0706" }
                }
            }
        }
        div {
            class: if drag.read().is_some() { "root dragging" } else { "root" },
            tabindex: "0",
            autofocus: true,
            onpointerdown: move |_| s_down.audio.user_gesture(),
            // Ctrl-click is a lock here, never a context menu.
            oncontextmenu: move |e| e.prevent_default(),
            onpointermove: move |e: PointerEvent| {
                let Some(d) = *s_move.drag.peek() else { return };
                e.prevent_default();
                let y = e.client_coordinates().y;
                let fine = e.modifiers().shift();
                let mut drag = s_move.drag;
                if fine != d.fine {
                    // Re-anchor so toggling SHIFT mid-drag doesn't jump.
                    drag.set(Some(Drag { start_y: y, start_value: s_move.value(d.target), fine, ..d }));
                    return;
                }
                let scale = if fine { 0.1 } else { 1.0 };
                let v = d.start_value as f64 + (d.start_y - y) / d.span_px * scale;
                s_move.set(d.target, v as f32);
            },
            onpointerup: move |_| end_drag_up(),
            onpointerleave: move |_| end_drag_leave(),
            onkeydown: move |e: KeyboardEvent| {
                let m = e.modifiers();
                if e.is_auto_repeating() || m.ctrl() || m.alt() || m.meta() {
                    return;
                }
                let code = e.code();
                if code == Code::Space {
                    e.prevent_default();
                    for v in 0..VOICES {
                        s_key.trigger(v, 1.0);
                    }
                } else if let Some(&(_, voice, vel)) = KEYMAP.iter().find(|(c, _, _)| *c == code) {
                    // Also stops a focused <select> from type-ahead jumping presets.
                    e.prevent_default();
                    s_key.trigger(voice, vel);
                } else {
                    s_key.audio.user_gesture();
                }
            },
            Header {}
            main { class: "channels",
                for v in 0..VOICES {
                    Channel { key: "{v}", voice: v }
                }
            }
            footer { class: "footer",
                span { "Keys: " }
                kbd { "Z X C V" }
                span { " cabinet I (soft → hard) · " }
                kbd { "N M , ." }
                span { " cabinet II · " }
                kbd { "Space" }
                span { " both · Drag knobs vertically (" }
                kbd { "Shift" }
                span { " = fine), scroll to nudge, double-click to reset · " }
                kbd { "Ctrl" }
                span { "-click any control to lock it against RND and MUTATE · MUTATE at RND: every hit is a new curiosity." }
            }
        }
    }
}

#[component]
fn Header() -> Element {
    let synth = use_context::<Synth>();
    let status = synth.audio.status.read().clone();
    let (class, text) = match status {
        AudioStatus::NeedsGesture => (
            "status warn",
            "Click anywhere or press a key to open the cabinet".to_string(),
        ),
        AudioStatus::Starting => ("status warn", "Unlocking the cabinet…".to_string()),
        AudioStatus::Running {
            sample_rate,
            detail,
        } => (
            "status ok",
            format!("{detail} · {:.1} kHz", sample_rate as f32 / 1000.0),
        ),
        AudioStatus::Failed(e) => ("status err", format!("Audio unavailable: {e}")),
    };
    rsx! {
        header { class: "header",
            div { class: "brand",
                Logo {}
                div { class: "brand-text",
                    div { class: "brand-line",
                        span { class: "logo", "WUNDERKAMMER" }
                        span { class: "model", "WK-1" }
                    }
                    span { class: "tagline", "A Cabinet of Improbable Sound Objects" }
                }
            }
            div { class: "{class}",
                span { class: "status-dot" }
                span { "{text}" }
            }
            div { class: "master",
                Knob { target: Target::Master, size: "sm" }
            }
        }
    }
}

/// The brand mark: a nautilus shell (a logarithmic spiral with its chambers),
/// the classic specimen of every cabinet of curiosities.
#[component]
fn Logo() -> Element {
    let (shell, septa) = use_hook(|| nautilus(17.0, 17.0));
    rsx! {
        svg { class: "brand-mark", view_box: "0 0 34 34",
            path { class: "shell", d: "{shell}" }
            path { class: "septa", d: "{septa}" }
        }
    }
}

/// (spiral outline, chamber walls) of a nautilus centred at (cx, cy).
fn nautilus(cx: f64, cy: f64) -> (String, String) {
    // r = a·e^(bθ); b for a growth of ≈3.2× per turn, like the real shell.
    let b = 3.2f64.ln() / std::f64::consts::TAU;
    let a = 0.75;
    let turns = 2.65;
    let steps = 220;
    let point = |theta: f64| {
        let r = a * (b * theta).exp();
        (cx + r * theta.cos(), cy - r * theta.sin())
    };
    let mut shell = String::new();
    for i in 0..=steps {
        let theta = turns * std::f64::consts::TAU * i as f64 / steps as f64;
        let (x, y) = point(theta);
        shell += &format!("{}{x:.2} {y:.2}", if i == 0 { 'M' } else { 'L' });
    }
    // Chamber walls: curved septa from each whorl to the next one in.
    let mut septa = String::new();
    for k in 6..=22 {
        let theta = turns * std::f64::consts::TAU * k as f64 / 22.0;
        let (x0, y0) = point(theta);
        let (x1, y1) = point(theta - std::f64::consts::TAU);
        let (mx, my) = (
            (x0 + x1) / 2.0 + (y1 - y0) * 0.18,
            (y0 + y1) / 2.0 - (x1 - x0) * 0.18,
        );
        septa += &format!("M{x0:.2} {y0:.2}Q{mx:.2} {my:.2} {x1:.2} {y1:.2}");
    }
    (shell, septa)
}

#[component]
fn Channel(voice: usize) -> Element {
    let synth = use_context::<Synth>();
    let other = (voice + 1) % VOICES;
    let s_preset = synth.clone();
    let s_rand = synth.clone();
    let s_copy = synth.clone();
    let s_unlock = synth.clone();
    let locks = synth.lock_count(voice);
    let p = |param: Param| Target::Param(voice, param);
    rsx! {
        section { class: "channel ch{voice + 1}",
            div { class: "channel-head",
                h2 { "CABINET {ROMAN[voice]}" }
                select {
                    class: "preset",
                    title: "Load a factory curiosity",
                    onchange: move |e| {
                        if let Some(p) = e.value().parse::<usize>().ok().and_then(|i| PRESETS.get(i)) {
                            s_preset.load_patch(voice, &p.patch());
                        }
                    },
                    option { value: "", disabled: true, selected: true, "Curiosities…" }
                    option { value: "0", "{PRESETS[0].name}" }
                    for e in 0..ENGINE_COUNT as u32 {
                        optgroup { key: "{e}", label: "{ENGINE_NAMES[e as usize]}",
                            for (i, p) in PRESETS.iter().enumerate().skip(1).filter(|(_, p)| p.engine() == e) {
                                option { key: "{i}", value: "{i}", "{p.name}" }
                            }
                        }
                    }
                }
                button {
                    class: "btn",
                    title: "Randomize this cabinet (locked controls stay put)",
                    onclick: move |_| s_rand.randomize(voice),
                    "RND"
                }
                button {
                    class: "btn",
                    title: "Copy this cabinet to cabinet {ROMAN[other]}",
                    onclick: move |_| s_copy.copy_voice(voice, other),
                    "→ {ROMAN[other]}"
                }
                if locks > 0 {
                    button {
                        class: "btn unlock",
                        title: "Unlock every control in this cabinet",
                        onclick: move |_| s_unlock.unlock_all(voice),
                        LockIcon {}
                        "{locks}"
                    }
                }
                SpecimenLabel { voice }
            }
            div { class: "channel-body",
                div { class: "sec sec-engine",
                    div { class: "sec-title",
                        if synth.is_locked(voice, Param::Engine) { LockIcon {} }
                        "OBJECT"
                    }
                    EnginePicker { voice }
                }
                div { class: "sec sec-scope",
                    HitScope { voice }
                }
                div { class: "sec sec-voice",
                    div { class: "row faders",
                        div { class: "tune-group",
                            Knob { target: p(Param::Fine), size: "sm" }
                            Fader { voice, param: Param::Pitch }
                        }
                        Fader { voice, param: Param::Time }
                        div { class: "macros",
                            Knob { target: p(Param::MacroX), size: "xl" }
                            Knob { target: p(Param::MacroY), size: "xl" }
                            Knob { target: p(Param::MacroZ), size: "xl" }
                        }
                    }
                }
                div { class: "sec sec-out",
                    div { class: "row",
                        Knob { target: p(Param::Level), size: "sm" }
                        Knob { target: p(Param::Sense), size: "sm" }
                        Knob { target: p(Param::Pan), size: "sm" }
                    }
                    Pad { voice }
                    div { class: "row mutate-row",
                        Knob { target: Target::Mutate(voice), size: "sm" }
                    }
                }
                div { class: "sec sec-bend",
                    div { class: "sec-title", "BEND" }
                    div { class: "row",
                        Knob { target: p(Param::BendTime) }
                        Knob { target: p(Param::BendDepth) }
                        Knob { target: p(Param::Whim) }
                    }
                }
                div { class: "sec sec-shift",
                    div { class: "sec-title",
                        "SHIFT"
                        span { class: "sec-note", "barber-pole echoes" }
                    }
                    div { class: "row",
                        Knob { target: p(Param::ShiftHz) }
                        Knob { target: p(Param::ShiftSpiral) }
                        Knob { target: p(Param::ShiftLag) }
                    }
                }
                div { class: "sec sec-tone",
                    div { class: "sec-title", "TONE" }
                    div { class: "row",
                        Knob { target: p(Param::Cutoff) }
                        Knob { target: p(Param::Reso) }
                        Switch { voice, param: Param::FilterType }
                    }
                }
                div { class: "sec sec-through",
                    div { class: "sec-title", "HEARD THROUGH" }
                    div { class: "row",
                        Switch { voice, param: Param::Medium }
                        Knob { target: p(Param::MediumDepth) }
                    }
                }
                div { class: "sec sec-reply",
                    div { class: "sec-title",
                        "REPLY"
                        ReplyGlyph { voice }
                        span { class: "sec-note", "answers cabinet {ROMAN[other]}" }
                    }
                    div { class: "row",
                        Knob { target: p(Param::ReplyOdds) }
                        Knob { target: p(Param::ReplyLag) }
                        Knob { target: p(Param::ReplyMood) }
                    }
                }
                div { class: "sec sec-reverb",
                    div { class: "sec-title", "REVERB" }
                    div { class: "row",
                        Knob { target: p(Param::ReverbMix) }
                        Knob { target: p(Param::ReverbDecay) }
                        Knob { target: p(Param::ReverbTone) }
                        Knob { target: p(Param::ReverbPredelay) }
                    }
                }
            }
        }
    }
}

fn engine_of(patch: &VoicePatch) -> u32 {
    Param::Engine.sanitize(patch[Param::Engine.index()]) as u32
}

/// A specimen tag: the object's catalogue number and name, and the last
/// touched control.
#[component]
fn SpecimenLabel(voice: usize) -> Element {
    let synth = use_context::<Synth>();
    let state = synth.state.read();
    let patch = &state.voices[voice];
    let engine = engine_of(patch);
    let line2 = match synth.touched.read()[voice] {
        Some(p) => format!(
            "{} {}",
            p.name_for(engine).to_uppercase(),
            p.display_for(engine, patch[p.index()])
        ),
        None => format!(
            "PITCH {} · TIME {}",
            Param::Pitch.display(patch[Param::Pitch.index()]),
            Param::Time.display(patch[Param::Time.index()])
        ),
    };
    rsx! {
        div { class: "tag",
            span { class: "tag-hole" }
            div { class: "tag-text",
                div { class: "tag-line tag-name",
                    span { class: "tag-no", "No. {engine + 1:02}" }
                    "{ENGINE_NAMES[engine as usize]}"
                }
                div { class: "tag-line", "{line2}" }
            }
        }
    }
}

/// Twelve object buttons with glyphs. Ctrl-click any of them to lock the
/// object choice.
#[component]
fn EnginePicker(voice: usize) -> Element {
    let synth = use_context::<Synth>();
    let current = engine_of(&synth.state.read().voices[voice]) as usize;
    let locked = synth.is_locked(voice, Param::Engine);
    rsx! {
        div {
            class: if locked { "engines locked" } else { "engines" },
            role: "radiogroup",
            "aria-label": "Object",
            for (i, name) in ENGINE_NAMES.iter().enumerate() {
                EngineButton { key: "{i}", voice, index: i, name, selected: i == current }
            }
        }
        div { class: "engine-blurb", "{ENGINE_BLURBS[current]}" }
    }
}

#[component]
fn EngineButton(voice: usize, index: usize, name: &'static str, selected: bool) -> Element {
    let synth = use_context::<Synth>();
    rsx! {
        button {
            class: if selected { "engine sel" } else { "engine" },
            role: "radio",
            "aria-checked": "{selected}",
            title: "{ENGINE_BLURBS[index]} (Ctrl-click: lock the object)",
            onclick: move |e: MouseEvent| {
                if lock_modifier(e.modifiers()) {
                    synth.toggle_lock(voice, Param::Engine);
                    return;
                }
                synth.set_param(voice, Param::Engine, index as f32);
                synth.save();
            },
            svg { class: "engine-glyph", view_box: "0 0 24 24",
                path { d: "{engine_glyph(index)}" }
            }
            span { "{name}" }
        }
    }
}

/// Line-art glyph per object, in a 24×24 box.
fn engine_glyph(engine: usize) -> &'static str {
    match engine {
        // BOUNCE: a ball and its shrinking arcs.
        0 => {
            "M2 21h20M4 21Q6 5 8 21Q9.8 10 11.6 21Q13 14 14.4 21Q15.4 17 16.4 21Q17.1 19 17.8 21M5 4.5a1.5 1.5 0 1 0 0.01 0"
        }
        // COIN: a disc on its edge, whirling.
        1 => {
            "M4 17A8 3 0 1 0 20 17A8 3 0 1 0 4 17M5 10C8 6.5 16 6.5 19 10M7.5 13C10 11 14 11 16.5 13"
        }
        // CREAK: a door on its hinge, squeaking.
        2 => "M5 3v18M5 4h11v17H5M3 7h2M3 17h2M13 12h.01M19 8l2.5-1.5M19 12h3M19 16l2.5 1.5",
        // GLOSSA: lips.
        3 => {
            "M2 12C6 5.5 9 7.5 12 8.5C15 7.5 18 5.5 22 12C18 18.5 6 18.5 2 12ZM2 12C8 13.5 16 13.5 22 12"
        }
        // POUR: a bottle, half full.
        4 => "M10 2h4v5l3 3v12H7V10l3-3ZM7 15c1.7 1 3.3-1 5 0s3.3 1 5 0",
        // AVIARY: birds on the wing.
        5 => "M2 9Q6 4 11 10Q16 4 21 9M7 17Q9 14.5 11.5 17.5Q14 14.5 16 17",
        // MUSICBOX: comb teeth over a pinned cylinder.
        6 => {
            "M3 3h18M5 3v8M8 3v10M11 3v7M14 3v11M17 3v9M20 3v6M3 16h18v5H3ZM7 18.5h.01M10.5 18.5h.01M14 18.5h.01M17.5 18.5h.01"
        }
        // ICE: a crack.
        7 => "M2 4L9 11L7 15L12 22M9 11L16 9L22 13M16 9L18 2M7 15L3 17",
        // INSPIRAL: a closing spiral.
        8 => {
            "M12 12m-1 0a1 1 0 1 1 2 0a2.5 2.5 0 1 1-5 0a4 4 0 1 1 8 0a5.5 5.5 0 1 1-11 0a7 7 0 1 1 14 0"
        }
        // AUTOMATA: rule 90 growing a Sierpiński triangle.
        9 => {
            "M11 2h2v2h-2ZM9 5h2v2H9ZM13 5h2v2h-2ZM7 8h2v2H7ZM15 8h2v2h-2ZM5 11h2v2H5ZM9 11h2v2H9ZM13 11h2v2h-2ZM17 11h2v2h-2ZM3 14h2v2H3ZM19 14h2v2h-2ZM1 17h2v2H1ZM5 17h2v2H5ZM17 17h2v2h-2ZM21 17h2v2h-2Z"
        }
        // PARADOX: the staircase that never ends.
        10 => "M3 20h4v-4h4v-4h4v-4h4V4M19 4c3 4 3 12-3 17H3",
        // BALLOON: a balloon, its knot and a trailing string.
        _ => "M12 2.5a6 7.5 0 1 0 0.01 0ZM10.8 17.4l1.2 1.6 1.2-1.6M12 19c-2 1 2 2 0 3.5",
    }
}

/// Offline-rendered outline of one hit of the current patch.
#[component]
fn HitScope(voice: usize) -> Element {
    let synth = use_context::<Synth>();
    let hits = synth.hits.read()[voice];
    let state = synth.state;
    let outline = use_memo(move || {
        let mut patch = state.read().voices[voice];
        // The scope shows the object itself, not the channel's placement.
        patch[Param::ReverbMix.index()] = 0.0;
        let time = map::time_seconds(patch[Param::Time.index()]);
        let seconds = (time * 1.3 + 0.25).clamp(0.3, SCOPE_MAX_S);
        let cols = wunderkammer_dsp::preview::hit_outline(
            &patch,
            SCOPE_VELOCITY,
            SCOPE_FS,
            seconds,
            SCOPE_COLUMNS,
        );
        (cols, seconds)
    });
    let (cols, seconds) = &*outline.read();
    let peak = cols
        .iter()
        .fold(0.0f32, |a, &(lo, hi)| a.max(lo.abs()).max(hi.abs()));
    let scale = 1.0 / peak.max(1e-3);
    let (w, h) = (SCOPE_COLUMNS as f32, 60.0f32);
    let mut d = String::with_capacity(SCOPE_COLUMNS * 24);
    for (i, &(_, hi)) in cols.iter().enumerate() {
        let cmd = if i == 0 { 'M' } else { 'L' };
        d += &format!("{cmd}{i} {:.1}", h / 2.0 - hi * scale * h * 0.46);
    }
    for (i, &(lo, _)) in cols.iter().enumerate().rev() {
        d += &format!("L{i} {:.1}", h / 2.0 - lo * scale * h * 0.46);
    }
    d += "Z";
    let peak_db = if peak > 1e-5 {
        format!("{:+.0} dB", 20.0 * peak.log10())
    } else {
        "silent".into()
    };
    let time = if *seconds >= 1.0 {
        format!("{seconds:.1} s")
    } else {
        format!("{:.0} ms", seconds * 1000.0)
    };
    rsx! {
        div { class: "scope",
            div { class: "sec-title scope-title",
                "ONE HIT"
                span { class: "scope-meta", "{time} · peak {peak_db}" }
            }
            svg {
                class: "scope-svg",
                view_box: "0 0 {w} {h}",
                preserve_aspect_ratio: "none",
                line { class: "scope-axis", x1: "0", y1: "{h / 2.0}", x2: "{w}", y2: "{h / 2.0}" }
                // Re-keyed on every hit so the flash replays.
                for k in std::iter::once(hits) {
                    path { key: "{k}", class: if k > 0 { "scope-wave played" } else { "scope-wave" }, d: "{d}" }
                }
            }
        }
    }
}

/// Two speech bubbles by the REPLY title; they talk while ODDS is up.
#[component]
fn ReplyGlyph(voice: usize) -> Element {
    let synth = use_context::<Synth>();
    let odds = synth.state.read().voices[voice][Param::ReplyOdds.index()];
    let lag = map::reply_lag_seconds(synth.state.read().voices[voice][Param::ReplyLag.index()]);
    rsx! {
        svg {
            class: if odds > 0.0 { "reply-glyph on" } else { "reply-glyph" },
            view_box: "0 0 30 16",
            style: "--lag: {(lag * 2.0).max(0.3):.2}s",
            path { class: "bubble a", d: "M2 2h11a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H7l-3 3v-3H2a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Z" }
            path { class: "bubble b", d: "M17 5h11a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2h-2v3l-3-3h-6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z" }
        }
    }
}

#[component]
fn LockIcon() -> Element {
    rsx! {
        svg { class: "lock-icon", view_box: "0 0 10 12", "aria-label": "locked",
            path { d: "M2.5 5V3.5a2.5 2.5 0 0 1 5 0V5" }
            rect { x: "1", y: "5", width: "8", height: "6.2", rx: "1.2" }
        }
    }
}

fn polar(cx: f64, cy: f64, r: f64, deg: f64) -> (f64, f64) {
    let a = deg.to_radians();
    (cx + r * a.sin(), cy - r * a.cos())
}

fn arc_path(cx: f64, cy: f64, r: f64, from_deg: f64, to_deg: f64) -> String {
    let (x0, y0) = polar(cx, cy, r, from_deg);
    let (x1, y1) = polar(cx, cy, r, to_deg);
    let large = if (to_deg - from_deg).abs() > 180.0 {
        1
    } else {
        0
    };
    format!("M {x0:.2} {y0:.2} A {r} {r} 0 {large} 1 {x1:.2} {y1:.2}")
}

/// The lockable parameter behind a control, if any.
fn lockable(target: Target) -> Option<(usize, Param)> {
    match target {
        Target::Param(v, p) if p.is_mutable() => Some((v, p)),
        _ => None,
    }
}

fn is_locked(synth: &Synth, target: Target) -> bool {
    lockable(target).is_some_and(|(v, p)| synth.is_locked(v, p))
}

fn begin_drag(synth: &Synth, target: Target, e: &PointerEvent, span_px: f64) {
    e.prevent_default();
    synth.audio.user_gesture();
    if lock_modifier(e.modifiers()) {
        if let Some((v, p)) = lockable(target) {
            synth.toggle_lock(v, p);
        }
        return;
    }
    if let Target::Param(v, p) = target {
        let mut touched = synth.touched;
        touched.write()[v] = Some(p);
    }
    let mut drag = synth.drag;
    drag.set(Some(Drag {
        target,
        start_y: e.client_coordinates().y,
        start_value: synth.value(target),
        span_px,
        fine: e.modifiers().shift(),
    }));
}

fn nudge(synth: &Synth, target: Target, e: &WheelEvent) {
    e.prevent_default();
    let dy = e.delta().strip_units().y;
    if dy == 0.0 {
        return;
    }
    let step = if e.modifiers().shift() { 0.002 } else { 0.02 };
    let v = synth.value(target) - (dy.signum() as f32) * step;
    synth.set(target, v);
    synth.save();
}

/// (label, descriptive name) for a control, resolving macro knobs to the
/// cabinet's current object.
fn target_info(synth: &Synth, target: Target) -> (String, String) {
    match target {
        Target::Param(v, p) => {
            let engine = engine_of(&synth.state.read().voices[v]);
            (
                p.label_for(engine).to_string(),
                p.name_for(engine).to_string(),
            )
        }
        Target::Master => ("MASTER".into(), "Master volume".into()),
        Target::Mutate(_) => ("MUTATE".into(), "Mutate on every hit".into()),
    }
}

fn display(synth: &Synth, target: Target, value: f32) -> String {
    match target {
        Target::Param(v, p) => p.display_for(engine_of(&synth.state.read().voices[v]), value),
        Target::Master => Param::Level.display(value),
        Target::Mutate(_) => crate::state::mutate_display(value),
    }
}

fn hover_title(name: &str, text: &str, target: Target, locked: bool) -> String {
    match (lockable(target), locked) {
        (Some(_), true) => format!("{name}: {text} (locked: Ctrl-click to unlock)"),
        (Some(_), false) => format!("{name}: {text} (Ctrl-click to lock)"),
        (None, _) => format!("{name}: {text}"),
    }
}

#[component]
fn Knob(target: Target, #[props(default = "lg")] size: &'static str) -> Element {
    let synth = use_context::<Synth>();
    let value = synth.value(target);
    let (label, name) = target_info(&synth, target);
    let active = matches!(*synth.drag.read(), Some(d) if d.target == target);
    let locked = is_locked(&synth, target);
    let angle = -135.0 + 270.0 * value as f64;
    let (px, py) = polar(30.0, 30.0, 15.0, angle);
    let track = arc_path(30.0, 30.0, 26.0, -135.0, 135.0);
    // Bipolar controls light the arc from the centre.
    let bipolar = matches!(target, Target::Param(_, p) if p.is_bipolar());
    let (a0, a1) = if bipolar {
        (angle.min(0.0), angle.max(0.0))
    } else {
        (-135.0, angle)
    };
    let lit = if (a1 - a0).abs() > 0.5 {
        arc_path(30.0, 30.0, 26.0, a0, a1)
    } else {
        String::new()
    };
    let text = display(&synth, target, value);
    let title = hover_title(&name, &text, target, locked);
    let is_macro = matches!(target, Target::Param(_, p) if p.macro_slot().is_some());
    let s_down = synth.clone();
    let s_dbl = synth.clone();
    let s_wheel = synth.clone();
    rsx! {
        div {
            class: "knob knob-{size}",
            class: if active { "active" },
            class: if is_macro { "macro" },
            class: if locked { "locked" },
            div { class: "ctl-label",
                if locked { LockIcon {} }
                "{label}"
            }
            svg {
                class: "knob-svg",
                view_box: "0 0 60 60",
                role: "slider",
                "aria-label": "{name}",
                "aria-valuetext": "{text}",
                onpointerdown: move |e| begin_drag(&s_down, target, &e, KNOB_SPAN_PX),
                ondoubleclick: move |_| s_dbl.reset(target),
                onwheel: move |e| nudge(&s_wheel, target, &e),
                title { "{title}" }
                path { class: "knob-track", d: "{track}" }
                if !lit.is_empty() {
                    path { class: "knob-lit", d: "{lit}" }
                }
                circle { class: "knob-skirt", cx: "30", cy: "30", r: "21" }
                circle { class: "knob-body", cx: "30", cy: "30", r: "17" }
                line { class: "knob-pointer", x1: "30", y1: "30", x2: "{px:.2}", y2: "{py:.2}" }
            }
            div { class: "ctl-value", "{text}" }
        }
    }
}

#[component]
fn Fader(voice: usize, param: Param) -> Element {
    let synth = use_context::<Synth>();
    let target = Target::Param(voice, param);
    let value = synth.value(target);
    let info = param.info();
    let text = param.display(value);
    let active = matches!(*synth.drag.read(), Some(d) if d.target == target);
    let locked = is_locked(&synth, target);
    let title = hover_title(info.name, &text, target, locked);
    let travel = FADER_TRACK_PX - FADER_CAP_PX;
    let cap_top = (1.0 - value as f64) * travel;
    let s_down = synth.clone();
    let s_dbl = synth.clone();
    let s_wheel = synth.clone();
    rsx! {
        div { class: "fader", class: if active { "active" }, class: if locked { "locked" },
            div { class: "ctl-label",
                if locked { LockIcon {} }
                "{info.label}"
            }
            div {
                class: "fader-body",
                role: "slider",
                title: "{title}",
                "aria-label": "{info.name}",
                "aria-valuetext": "{text}",
                onpointerdown: move |e| begin_drag(&s_down, target, &e, travel),
                ondoubleclick: move |_| s_dbl.reset(target),
                onwheel: move |e| nudge(&s_wheel, target, &e),
                div { class: "fader-leds",
                    for i in (0..FADER_LEDS).rev() {
                        div {
                            key: "{i}",
                            class: if value * FADER_LEDS as f32 >= i as f32 + 0.5 { "led on" } else { "led" },
                        }
                    }
                }
                div { class: "fader-track",
                    div { class: "fader-slot" }
                    div { class: "fader-cap", style: "top: {cap_top:.1}px" }
                }
            }
            div { class: "ctl-value", "{text}" }
        }
    }
}

/// Multi-position slide switch for choice parameters; click a position to
/// select it, or the slot to step. Ctrl-click locks it.
#[component]
fn Switch(voice: usize, param: Param) -> Element {
    let synth = use_context::<Synth>();
    let ParamKind::Choice(names) = param.info().kind else {
        return rsx! {};
    };
    let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let current = synth.value(Target::Param(voice, param)) as usize;
    let locked = synth.is_locked(voice, param);
    let n = names.len();
    let slot_px = SWITCH_STEP_PX * n as f64 + 4.0;
    let lever = if n > 1 {
        current as f64 / (n - 1) as f64
    } else {
        0.0
    };
    let lever_px =
        SWITCH_LEVER_INSET_PX + lever * (slot_px - SWITCH_LEVER_PX - 2.0 * SWITCH_LEVER_INSET_PX);
    let title = hover_title(
        param.info().name,
        &names[current],
        Target::Param(voice, param),
        locked,
    );
    let s_cycle = synth.clone();
    rsx! {
        div { class: "switch", class: if locked { "locked" },
            div { class: "ctl-label",
                if locked { LockIcon {} }
                "{param.info().label}"
            }
            div { class: "switch-body",
                div {
                    class: "switch-slot",
                    style: "height: {slot_px}px",
                    title: "{title}",
                    onclick: move |e: MouseEvent| {
                        if lock_modifier(e.modifiers()) {
                            s_cycle.toggle_lock(voice, param);
                            return;
                        }
                        s_cycle.set_param(voice, param, ((current + 1) % n) as f32);
                        s_cycle.save();
                    },
                    div { class: "switch-lever", style: "top: {lever_px:.1}px" }
                }
                div { class: "switch-labels", style: "height: {slot_px}px",
                    for (i, name) in names.into_iter().enumerate() {
                        SwitchLabel { key: "{i}", voice, param, index: i, name, selected: i == current }
                    }
                }
            }
        }
    }
}

#[component]
fn SwitchLabel(voice: usize, param: Param, index: usize, name: String, selected: bool) -> Element {
    let synth = use_context::<Synth>();
    rsx! {
        div {
            class: if selected { "switch-label sel" } else { "switch-label" },
            onclick: move |e: MouseEvent| {
                if lock_modifier(e.modifiers()) {
                    synth.toggle_lock(voice, param);
                    return;
                }
                synth.set_param(voice, param, index as f32);
                synth.save();
            },
            "{name}"
        }
    }
}

#[component]
fn Pad(voice: usize) -> Element {
    let synth = use_context::<Synth>();
    let hits = synth.hits.read()[voice];
    rsx! {
        div {
            class: "pad",
            role: "button",
            title: "Disturb the specimen (hit higher for a harder hit)",
            onpointerdown: move |e: PointerEvent| {
                e.prevent_default();
                let y = e.element_coordinates().y;
                let velocity = (1.0 - y / PAD_PX).clamp(0.05, 1.0) as f32;
                synth.trigger(voice, velocity);
            },
            for h in std::iter::once(hits) {
                span { key: "{h}", class: if h > 0 { "led trig-led flash" } else { "led trig-led" } }
            }
            span { class: "pad-label", "DISTURB" }
            span { class: "pad-hint", "hard" }
            span { class: "pad-hint soft", "soft" }
        }
    }
}
