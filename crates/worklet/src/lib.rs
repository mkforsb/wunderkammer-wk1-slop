//! C ABI over the engine for the AudioWorkletProcessor (`worklet.js`).
//!
//! The module imports nothing, so the worklet can instantiate it with an empty
//! import object. Each worklet node owns one engine handle.

use wunderkammer_dsp::{Engine, Param};

/// Frames per render call; matches the WebAudio render quantum.
pub const BLOCK: usize = 128;

pub struct Worklet {
    engine: Engine,
    left: [f32; BLOCK],
    right: [f32; BLOCK],
}

/// Voice index that addresses global parameters in [`wk_set_param`].
pub const GLOBAL: u32 = 255;
/// Global parameter id: master volume.
pub const GLOBAL_MASTER: u32 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn wk_new(sample_rate: f32) -> *mut Worklet {
    Box::into_raw(Box::new(Worklet {
        engine: Engine::new(sample_rate),
        left: [0.0; BLOCK],
        right: [0.0; BLOCK],
    }))
}

/// # Safety
/// `w` must come from [`wk_new`] and not have been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_free(w: *mut Worklet) {
    if !w.is_null() {
        drop(unsafe { Box::from_raw(w) });
    }
}

/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_set_param(w: *mut Worklet, voice: u32, id: u32, value: f32) {
    let w = unsafe { &mut *w };
    if voice == GLOBAL {
        if id == GLOBAL_MASTER {
            w.engine.set_master(value);
        }
    } else if let Some(p) = Param::from_id(id) {
        w.engine.set_param(voice as usize, p, value);
    }
}

/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_trigger(w: *mut Worklet, voice: u32, velocity: f32) {
    unsafe { &mut *w }.engine.trigger(voice as usize, velocity);
}

/// Jump one voice's smoothed parameters to their targets (patch changes
/// that must land exactly on the next trigger).
///
/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_snap_voice(w: *mut Worklet, voice: u32) {
    unsafe { &mut *w }.engine.snap_voice(voice as usize);
}

/// Jump smoothed parameters to their targets (after the initial patch sync).
///
/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_snap(w: *mut Worklet) {
    unsafe { &mut *w }.engine.snap_params();
}

/// Render `frames` (≤ [`BLOCK`]) into the buffers returned by [`wk_left`]/[`wk_right`].
///
/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_render(w: *mut Worklet, frames: u32) {
    let w = unsafe { &mut *w };
    let n = (frames as usize).min(BLOCK);
    w.engine.render(&mut w.left[..n], &mut w.right[..n]);
}

/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_left(w: *mut Worklet) -> *const f32 {
    unsafe { (*w).left.as_ptr() }
}

/// # Safety
/// `w` must come from [`wk_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wk_right(w: *mut Worklet) -> *const f32 {
    unsafe { (*w).right.as_ptr() }
}
