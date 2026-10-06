//! Playing live, no MIDI file: notes, an instrument change, a pitch bend
//! and the sustain pedal, as a game would for sound effects or music it
//! makes up as it goes. Writes `live.wav`:
//! `cargo run --release --example live`.

use neumetik_midi::Synth;

const RATE: u32 = 48_000;

/// Renders `seconds` more into `out`.
fn wait(synth: &mut Synth, out: &mut Vec<f32>, seconds: f32) {
    let start = out.len();
    out.resize(start + (seconds * RATE as f32) as usize * 2, 0.0);
    synth.render_interleaved(&mut out[start..]);
}

fn main() -> std::io::Result<()> {
    let mut synth = Synth::new(RATE);
    let mut out = Vec::new();

    // A coin: two quick notes on a vibraphone.
    synth.set_instrument(0, 0, 11);
    synth.note_on(0, 83, 110);
    wait(&mut synth, &mut out, 0.07);
    synth.note_off(0, 83);
    synth.note_on(0, 88, 110);
    wait(&mut synth, &mut out, 0.6);
    synth.note_off(0, 88);

    // A power-up: a saw lead from the nineties bank, bending up an octave
    // (the bend range set to 12 semitones by RPN 0).
    synth.set_instrument(1, 81, 0);
    for (cc, value) in [(101, 0), (100, 0), (6, 12)] {
        synth.control_change(1, cc, value);
    }
    synth.note_on(1, 60, 100);
    for step in 0..=40 {
        synth.pitch_bend(1, step * 8191 / 40);
        wait(&mut synth, &mut out, 0.01);
    }
    synth.note_off(1, 60);
    wait(&mut synth, &mut out, 0.5);

    // A chord held by the pedal, from raw MIDI bytes (running status).
    synth.send(&[0xc2, 0, 0xb2, 64, 127, 0x92, 60, 90, 64, 90, 67, 90]);
    wait(&mut synth, &mut out, 0.2);
    synth.send(&[0x82, 60, 0, 64, 0, 67, 0]);
    wait(&mut synth, &mut out, 1.0);
    synth.control_change(2, 64, 0);

    // A drum fill on channel 9.
    for (i, key) in [38, 38, 45, 43, 41, 49].into_iter().enumerate() {
        synth.note_on(9, key, 100 + i as u8 * 4);
        wait(&mut synth, &mut out, 0.1);
    }
    wait(&mut synth, &mut out, 1.5);

    std::fs::write("live.wav", neumetik_midi::wav(&out, RATE))?;
    println!("live.wav: {:.1} s, {} voices still sounding", out.len() as f32 / 2.0 / RATE as f32, synth.active_voices());
    Ok(())
}
