// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! Neumetik MIDI's C API, `include/neumetik_midi.h`: a player handle and
//! plain functions over it, for C and C++ and any language with a C FFI
//! (Lua, C#, Python, GDExtension), and for WebAssembly.
//!
//! Every function takes a null handle quietly (doing nothing, returning 0)
//! and never panics.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_char;

use midi::{PlayState, Player};

/// The player behind an `nm_player*`.
pub struct NmPlayer(Player);

fn get<'a>(p: *mut NmPlayer) -> Option<&'a mut Player> {
    // SAFETY: the caller hands back a pointer nm_create made, as the
    // header says, or null.
    unsafe { p.as_mut().map(|p| &mut p.0) }
}

fn get_ref<'a>(p: *const NmPlayer) -> Option<&'a Player> {
    // SAFETY: as in `get`.
    unsafe { p.as_ref().map(|p| &p.0) }
}

fn bytes<'a>(data: *const u8, size: usize) -> &'a [u8] {
    if data.is_null() || size == 0 {
        &[]
    } else {
        // SAFETY: the caller says `data` holds `size` bytes.
        unsafe { std::slice::from_raw_parts(data, size) }
    }
}

fn floats<'a>(data: *mut f32, len: usize) -> Option<&'a mut [f32]> {
    // SAFETY: the caller says `data` holds `len` floats.
    (!data.is_null()).then(|| unsafe { std::slice::from_raw_parts_mut(data, len) })
}

const VERSION: u32 = 1_00;

#[no_mangle]
pub extern "C" fn nm_version() -> u32 {
    VERSION
}

#[no_mangle]
pub extern "C" fn nm_create(sample_rate: u32) -> *mut NmPlayer {
    Box::into_raw(Box::new(NmPlayer(Player::new(sample_rate))))
}

#[no_mangle]
pub unsafe extern "C" fn nm_destroy(p: *mut NmPlayer) {
    if !p.is_null() {
        // SAFETY: made by nm_create, and the caller won't use it again.
        drop(unsafe { Box::from_raw(p) });
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_sample_rate(p: *const NmPlayer) -> u32 {
    get_ref(p).map_or(0, |p| p.synth().sample_rate())
}

#[no_mangle]
pub unsafe extern "C" fn nm_load(p: *mut NmPlayer, data: *const u8, size: usize) -> i32 {
    let Some(p) = get(p) else { return -5 };
    match p.load(bytes(data, size)) {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "C" fn nm_error_string(code: i32) -> *const c_char {
    let s: &'static [u8] = match code {
        0 => b"no error\0",
        -1 => b"it isn't a MIDI file\0",
        -2 => b"it ends too soon\0",
        -3 => b"its timing is zero\0",
        -4 => b"it has no music in it\0",
        -5 => b"no player\0",
        _ => b"unknown error\0",
    };
    s.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn nm_unload(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.unload();
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_play(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.play();
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_pause(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.pause();
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_stop(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.stop();
    }
}

/// 0 stopped, 1 playing, 2 paused.
#[no_mangle]
pub unsafe extern "C" fn nm_state(p: *const NmPlayer) -> i32 {
    match get_ref(p).map(Player::state) {
        Some(PlayState::Playing) => 1,
        Some(PlayState::Paused) => 2,
        _ => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_is_finished(p: *const NmPlayer) -> i32 {
    get_ref(p).map_or(0, |p| i32::from(p.is_finished()))
}

#[no_mangle]
pub unsafe extern "C" fn nm_seek(p: *mut NmPlayer, seconds: f64) {
    if let Some(p) = get(p) {
        p.seek(seconds);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_position(p: *const NmPlayer) -> f64 {
    get_ref(p).map_or(0.0, Player::position)
}

#[no_mangle]
pub unsafe extern "C" fn nm_duration(p: *const NmPlayer) -> f64 {
    get_ref(p).map_or(0.0, Player::duration)
}

#[no_mangle]
pub unsafe extern "C" fn nm_set_looping(p: *mut NmPlayer, looping: i32) {
    if let Some(p) = get(p) {
        p.set_looping(looping != 0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_is_looping(p: *const NmPlayer) -> i32 {
    get_ref(p).map_or(0, |p| i32::from(p.is_looping()))
}

/// A negative start or end goes back to the file's own (or the song's).
#[no_mangle]
pub unsafe extern "C" fn nm_set_loop_points(p: *mut NmPlayer, start: f64, end: f64) {
    if let Some(p) = get(p) {
        p.set_loop_points((start >= 0.0).then_some(start), (end >= 0.0).then_some(end));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_loop_start(p: *const NmPlayer) -> f64 {
    get_ref(p).map_or(0.0, |p| p.loop_points().0)
}

#[no_mangle]
pub unsafe extern "C" fn nm_loop_end(p: *const NmPlayer) -> f64 {
    get_ref(p).map_or(0.0, |p| p.loop_points().1)
}

#[no_mangle]
pub unsafe extern "C" fn nm_set_speed(p: *mut NmPlayer, speed: f64) {
    if let Some(p) = get(p) {
        p.set_speed(speed);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_set_volume(p: *mut NmPlayer, volume: f32) {
    if let Some(p) = get(p) {
        p.set_volume(volume);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_volume(p: *const NmPlayer) -> f32 {
    get_ref(p).map_or(0.0, Player::volume)
}

#[no_mangle]
pub unsafe extern "C" fn nm_fade_to(p: *mut NmPlayer, volume: f32, seconds: f32) {
    if let Some(p) = get(p) {
        p.fade_to(volume, seconds);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_set_channel_muted(p: *mut NmPlayer, channel: i32, muted: i32) {
    if let (Some(p), Ok(ch)) = (get(p), u8::try_from(channel)) {
        p.set_channel_muted(ch, muted != 0);
    }
}

fn byte(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

#[no_mangle]
pub unsafe extern "C" fn nm_note_on(p: *mut NmPlayer, channel: i32, key: i32, velocity: i32) {
    if let Some(p) = get(p) {
        p.synth_mut().note_on(byte(channel), byte(key), byte(velocity));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_note_off(p: *mut NmPlayer, channel: i32, key: i32) {
    if let Some(p) = get(p) {
        p.synth_mut().note_off(byte(channel), byte(key));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_control_change(p: *mut NmPlayer, channel: i32, controller: i32, value: i32) {
    if let Some(p) = get(p) {
        p.synth_mut().control_change(byte(channel), byte(controller), byte(value));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_program_change(p: *mut NmPlayer, channel: i32, program: i32) {
    if let Some(p) = get(p) {
        p.synth_mut().program_change(byte(channel), byte(program));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_set_instrument(p: *mut NmPlayer, channel: i32, bank: i32, program: i32) {
    if let Some(p) = get(p) {
        p.synth_mut().set_instrument(byte(channel), byte(bank), byte(program));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_pitch_bend(p: *mut NmPlayer, channel: i32, value: i32) {
    if let Some(p) = get(p) {
        p.synth_mut().pitch_bend(byte(channel), value.clamp(-8192, 8191) as i16);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_send_midi(p: *mut NmPlayer, data: *const u8, size: usize) {
    if let Some(p) = get(p) {
        p.synth_mut().send(bytes(data, size));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_all_notes_off(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.synth_mut().all_notes_off();
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_all_sound_off(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.synth_mut().all_sound_off();
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_reset(p: *mut NmPlayer) {
    if let Some(p) = get(p) {
        p.synth_mut().reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_active_voices(p: *const NmPlayer) -> i32 {
    get_ref(p).map_or(0, |p| p.synth().active_voices() as i32)
}

/// Copies the instrument's name into `buffer` (cut to fit, always ended
/// with a NUL) and returns its full length, as snprintf does.
#[no_mangle]
pub unsafe extern "C" fn nm_instrument_name(p: *const NmPlayer, channel: i32, buffer: *mut c_char, size: usize) -> i32 {
    let Some(p) = get_ref(p) else { return 0 };
    let name = u8::try_from(channel).map_or("", |ch| p.synth().instrument_name(ch));
    if !buffer.is_null() && size > 0 {
        let n = name.len().min(size - 1);
        // SAFETY: the caller says `buffer` holds `size` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(name.as_ptr(), buffer.cast::<u8>(), n);
            *buffer.add(n) = 0;
        }
    }
    name.len() as i32
}

#[no_mangle]
pub unsafe extern "C" fn nm_render_f32(p: *mut NmPlayer, out: *mut f32, frames: usize) {
    let Some(out) = floats(out, frames.saturating_mul(2)) else { return };
    match get(p) {
        Some(p) => p.render_interleaved(out),
        None => out.fill(0.0),
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_render_s16(p: *mut NmPlayer, out: *mut i16, frames: usize) {
    if out.is_null() {
        return;
    }
    // SAFETY: the caller says `out` holds `frames` stereo frames.
    let out = unsafe { std::slice::from_raw_parts_mut(out, frames.saturating_mul(2)) };
    match get(p) {
        Some(p) => p.render_i16(out),
        None => out.fill(0),
    }
}

#[no_mangle]
pub unsafe extern "C" fn nm_render_f32_planar(p: *mut NmPlayer, left: *mut f32, right: *mut f32, frames: usize) {
    let (Some(l), Some(r)) = (floats(left, frames), floats(right, frames)) else { return };
    match get(p) {
        Some(p) => p.render(l, r),
        None => {
            l.fill(0.0);
            r.fill(0.0);
        }
    }
}

/// WebAssembly has no malloc for JavaScript to call: these lend it memory
/// for MIDI files and sample buffers.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn nm_alloc(size: usize) -> *mut u8 {
    let mut v = std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(size.max(1)));
    v.as_mut_ptr()
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub unsafe extern "C" fn nm_free(ptr: *mut u8, size: usize) {
    if !ptr.is_null() {
        // SAFETY: made by nm_alloc with this size.
        drop(unsafe { Vec::from_raw_parts(ptr, 0, size.max(1)) });
    }
}
