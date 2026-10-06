// Copyright 2026 ansiapps. MIT licensed.

//! For the demo page's A/B comparison: a SoundFont read, and a MIDI file
//! rendered through it whole, by rustysynth. JavaScript passes bytes in
//! through `sf_alloc` and reads the samples out at `sf_left`/`sf_right`.

use std::sync::Arc;

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

static mut FONT: Option<Arc<SoundFont>> = None;
static mut LEFT: Vec<f32> = Vec::new();
static mut RIGHT: Vec<f32> = Vec::new();

#[no_mangle]
pub extern "C" fn sf_alloc(size: usize) -> *mut u8 {
    let mut v = std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(size.max(1)));
    v.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn sf_free(ptr: *mut u8, size: usize) {
    drop(Vec::from_raw_parts(ptr, 0, size.max(1)));
}

/// Reads a SoundFont: 0, or -1 if it isn't one.
#[no_mangle]
pub unsafe extern "C" fn sf_load_font(ptr: *const u8, size: usize) -> i32 {
    let mut bytes = std::slice::from_raw_parts(ptr, size);
    match SoundFont::new(&mut bytes) {
        Ok(font) => {
            FONT = Some(Arc::new(font));
            0
        }
        Err(_) => -1,
    }
}

/// Renders a MIDI file through the SoundFont, `tail` seconds past its end
/// (`longest` at most): the frames rendered, or -1 with no SoundFont, -2
/// if the MIDI file won't read.
#[no_mangle]
pub unsafe extern "C" fn sf_render(ptr: *const u8, size: usize, rate: i32, tail: f64, longest: f64) -> i32 {
    #[allow(static_mut_refs)]
    let Some(font) = FONT.clone() else { return -1 };
    let mut bytes = std::slice::from_raw_parts(ptr, size);
    let Ok(midi) = MidiFile::new(&mut bytes) else { return -2 };
    let midi = Arc::new(midi);
    let settings = SynthesizerSettings::new(rate);
    let Ok(synth) = Synthesizer::new(&font, &settings) else { return -1 };
    let mut seq = MidiFileSequencer::new(synth);
    seq.play(&midi, false);
    let frames = ((midi.get_length() + tail).min(longest) * f64::from(rate)) as usize;
    #[allow(static_mut_refs)]
    {
        LEFT = vec![0.0; frames];
        RIGHT = vec![0.0; frames];
        seq.render(&mut LEFT[..], &mut RIGHT[..]);
    }
    frames as i32
}

#[no_mangle]
pub unsafe extern "C" fn sf_left() -> *const f32 {
    #[allow(static_mut_refs)]
    LEFT.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn sf_right() -> *const f32 {
    #[allow(static_mut_refs)]
    RIGHT.as_ptr()
}
