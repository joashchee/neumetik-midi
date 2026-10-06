//! Writes the demo songs the demo page, the benchmarks and the binding
//! examples play: `cargo run --example make_demos -- <folder>`.
//!
//! - `overworld.mid`: an upbeat game loop, with `loopStart` and `loopEnd`
//!   markers after a two-bar intro, in layers (drums, bass, chords, lead)
//!   to mute and unmute.
//! - `dungeon.mid`: slow and minor; strings swelling (CC 11), a filter
//!   opening (CC 74), a flute bending into its notes.
//! - `fur-elise.mid`: the opening of Beethoven's "Für Elise" (public
//!   domain), piano with the sustain pedal.
//! - `gm-tour.mid`: a phrase on 25 instruments across General MIDI and
//!   the classic synth banks, then a beat on eight drum kits.
//!
//! All written here, note by note: free to use, as the crate is (MIT).

use std::path::PathBuf;

/// Ticks a quarter note.
const Q: u32 = 480;
const E: u32 = Q / 2;
const S: u32 = Q / 4;
const BAR: u32 = Q * 4;

/// One track's events, by tick, in any order.
#[derive(Default)]
struct Track {
    events: Vec<(u32, u8, Vec<u8>)>,
}

impl Track {
    /// Note-offs sort before anything else on their tick.
    fn at(&mut self, tick: u32, bytes: &[u8]) {
        let order = if bytes[0] & 0xf0 == 0x80 { 0 } else { 1 };
        self.events.push((tick, order, bytes.to_vec()));
    }

    fn note(&mut self, ch: u8, tick: u32, len: u32, key: u8, vel: u8) {
        self.at(tick, &[0x90 | ch, key, vel]);
        self.at(tick + len, &[0x80 | ch, key, 0]);
    }

    fn program(&mut self, ch: u8, tick: u32, bank: u8, program: u8) {
        self.at(tick, &[0xb0 | ch, 0, bank]);
        self.at(tick, &[0xc0 | ch, program]);
    }

    fn cc(&mut self, ch: u8, tick: u32, cc: u8, value: u8) {
        self.at(tick, &[0xb0 | ch, cc, value]);
    }

    fn meta(&mut self, tick: u32, kind: u8, data: &[u8]) {
        let mut bytes = vec![0xff, kind];
        bytes.extend(vlq(data.len() as u32));
        bytes.extend_from_slice(data);
        self.events.push((tick, 1, bytes));
    }

    fn marker(&mut self, tick: u32, text: &str) {
        self.meta(tick, 0x06, text.as_bytes());
    }

    fn tempo(&mut self, tick: u32, bpm: u32) {
        let us = 60_000_000 / bpm;
        self.meta(tick, 0x51, &[(us >> 16) as u8, (us >> 8) as u8, us as u8]);
    }

    fn bytes(mut self, end: u32) -> Vec<u8> {
        self.events.sort_by_key(|(tick, order, _)| (*tick, *order));
        let mut out = Vec::new();
        let mut last = 0;
        for (tick, _, bytes) in &self.events {
            out.extend(vlq(tick - last));
            out.extend_from_slice(bytes);
            last = *tick;
        }
        out.extend(vlq(end.saturating_sub(last)));
        out.extend_from_slice(&[0xff, 0x2f, 0]);
        out
    }
}

fn vlq(mut n: u32) -> Vec<u8> {
    let mut v = vec![(n & 0x7f) as u8];
    n >>= 7;
    while n > 0 {
        v.insert(0, (n & 0x7f) as u8 | 0x80);
        n >>= 7;
    }
    v
}

/// A format 1 file of `tracks`, all ending at `end`.
fn file(tracks: Vec<Track>, end: u32) -> Vec<u8> {
    let mut out = b"MThd".to_vec();
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
    out.extend_from_slice(&(Q as u16).to_be_bytes());
    for t in tracks {
        let body = t.bytes(end);
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
    }
    out
}

/// Drum keys (General MIDI).
const KICK: u8 = 36;
const SNARE: u8 = 38;
const HAT: u8 = 42;
const OPEN_HAT: u8 = 46;
const CRASH: u8 = 49;

fn overworld() -> Vec<u8> {
    let mut meta = Track::default();
    meta.tempo(0, 140);
    // Two bars of intro, then eight that loop.
    meta.marker(2 * BAR, "loopStart");
    meta.marker(10 * BAR, "loopEnd");
    let (mut drums, mut bass, mut chords, mut lead) = (Track::default(), Track::default(), Track::default(), Track::default());
    bass.program(1, 0, 0, 38); // Synth Bass 1
    chords.program(2, 0, 0, 4); // Electric Piano 1
    chords.cc(2, 0, 91, 70);
    lead.program(3, 0, 0, 80); // Square Lead
    lead.cc(3, 0, 91, 50);
    lead.cc(3, 0, 93, 40);
    // C, G, Am, F, C, G, F, G; the intro on C and G.
    let roots: [u8; 10] = [48, 43, 48, 43, 45, 41, 48, 43, 41, 43];
    let thirds: [u8; 10] = [4, 4, 4, 4, 3, 4, 4, 4, 4, 4];
    for (bar, (&root, &third)) in roots.iter().zip(&thirds).enumerate() {
        let t0 = bar as u32 * BAR;
        for beat in 0..8 {
            let t = t0 + beat * E;
            drums.note(9, t, E, HAT, if beat % 2 == 0 { 90 } else { 60 });
            if beat % 4 == 0 {
                drums.note(9, t, E, KICK, 110);
            }
            if beat % 4 == 2 {
                drums.note(9, t, E, SNARE, 100);
            }
            if beat == 7 && bar % 4 == 3 {
                drums.note(9, t, E, OPEN_HAT, 80);
            }
            let key = if beat % 2 == 1 { root } else { root - 12 };
            bass.note(1, t, E - 20, key, if beat % 2 == 0 { 105 } else { 85 });
        }
        if bar < 2 {
            continue;
        }
        if bar == 2 {
            drums.note(9, t0, Q * 2, CRASH, 100);
        }
        // The chord, broken in sixteenths.
        let chord = [root + 12, root + 12 + third, root + 19, root + 24];
        for step in 0..16 {
            let k = chord[[0, 1, 2, 3, 2, 1][step as usize % 6]];
            chords.note(2, t0 + step * S, S, k, if step % 4 == 0 { 80 } else { 58 });
        }
    }
    // The tune: (key, eighths), 0 a rest; eight bars of eight eighths.
    let tune: [&[(u8, u32)]; 8] = [
        &[(76, 2), (79, 2), (84, 3), (83, 1)],
        &[(81, 2), (79, 2), (74, 3), (0, 1)],
        &[(72, 2), (76, 2), (81, 2), (79, 1), (76, 1)],
        &[(77, 3), (76, 1), (74, 4)],
        &[(76, 2), (79, 2), (84, 2), (86, 1), (88, 1)],
        &[(86, 2), (83, 2), (79, 3), (0, 1)],
        &[(81, 1), (79, 1), (77, 1), (76, 1), (74, 2), (77, 2)],
        &[(79, 6), (0, 2)],
    ];
    for (i, phrase) in tune.iter().enumerate() {
        let mut t = (2 + i as u32) * BAR;
        for &(key, eighths) in phrase.iter() {
            if key != 0 {
                lead.note(3, t, eighths * E - 30, key, 96);
            }
            t += eighths * E;
        }
    }
    file(vec![meta, drums, bass, chords, lead], 10 * BAR)
}

fn dungeon() -> Vec<u8> {
    let mut meta = Track::default();
    meta.tempo(0, 72);
    let (mut strings, mut harp, mut pad, mut flute, mut drums) = (Track::default(), Track::default(), Track::default(), Track::default(), Track::default());
    strings.program(0, 0, 0, 48); // String Ensemble 1
    strings.cc(0, 0, 91, 90);
    harp.program(1, 0, 0, 46); // Orchestral Harp
    harp.cc(1, 0, 91, 80);
    pad.program(2, 0, 0, 89); // Warm Pad
    pad.cc(2, 0, 7, 80);
    flute.program(3, 0, 0, 73); // Flute
    flute.cc(3, 0, 91, 90);
    drums.program(9, 0, 0, 48); // the orchestra kit
    // Dm, Bb, Gm, A: two bars each.
    let chords: [[u8; 3]; 4] = [[50, 53, 57], [46, 50, 53], [43, 46, 50], [45, 49, 52]];
    for (i, chord) in chords.iter().enumerate() {
        let t0 = i as u32 * 2 * BAR;
        for &k in chord {
            strings.note(0, t0, 2 * BAR - 10, k + 12, 70);
            pad.note(2, t0, 2 * BAR - 10, k, 60);
        }
        // Each chord swells and the pad's filter opens.
        for step in 0..=16u32 {
            let rise = (step * 127 / 16) as u8;
            strings.cc(0, t0 + step * BAR / 8, 11, 50 + rise * 77 / 127);
            pad.cc(2, t0 + step * BAR / 8, 74, 30 + rise * 70 / 127);
        }
        let arp = [chord[0] + 12, chord[1] + 12, chord[2] + 12, chord[0] + 24];
        for step in 0..16u32 {
            harp.note(1, t0 + step * E, Q, arp[[0, 1, 2, 3, 2, 1, 0, 1][step as usize % 8]], 70);
        }
        // Timpani on the bar.
        drums.note(9, t0, Q, 41, 90);
        drums.note(9, t0 + BAR, Q, 41, 70);
    }
    // The flute's tune, bending up into its first note of each phrase.
    let tune: [(u8, u32); 12] = [(74, 3), (77, 1), (76, 2), (74, 2), (70, 4), (72, 2), (74, 2), (67, 6), (69, 2), (73, 4), (76, 2), (74, 2)];
    let mut t = 2 * BAR;
    for (i, &(key, quarters)) in tune.iter().enumerate() {
        if i % 4 == 0 {
            // From a semitone below to in tune over an eighth.
            for s in 0..=8u32 {
                let value = -4096 + (4096 * s / 8) as i32;
                let raw = (value + 8192) as u16;
                flute.at(t + s * E / 8, &[0xe3, (raw & 0x7f) as u8, (raw >> 7) as u8]);
            }
        }
        flute.note(3, t, quarters * Q - 40, key, 88);
        t += quarters * Q;
    }
    file(vec![meta, strings, harp, pad, flute, drums], 8 * BAR)
}

fn fur_elise() -> Vec<u8> {
    let mut meta = Track::default();
    meta.tempo(0, 66);
    let mut piano = Track::default();
    piano.program(0, 0, 0, 0);
    piano.cc(0, 0, 91, 50);
    // In sixteenths: the right hand's keys and lengths, 0 a rest.
    const E5: u8 = 76;
    const DS5: u8 = 75;
    const B4: u8 = 71;
    const D5: u8 = 74;
    const C5: u8 = 72;
    const A4: u8 = 69;
    const C4: u8 = 60;
    const E4: u8 = 64;
    const GS4: u8 = 68;
    let turn: &[(u8, u32)] = &[(E5, 1), (DS5, 1), (E5, 1), (B4, 1), (D5, 1), (C5, 1)];
    let mut right: Vec<(u8, u32)> = vec![(E5, 1), (DS5, 1)];
    let to_a = [(A4, 2), (C4, 1), (E4, 1), (A4, 1)];
    let to_b = [(B4, 2), (E4, 1), (GS4, 1), (B4, 1)];
    let to_c = [(C5, 2), (E4, 1), (E5, 1), (DS5, 1)];
    let to_b_home = [(B4, 2), (E4, 1), (C5, 1), (B4, 1)];
    for _ in 0..2 {
        right.extend_from_slice(turn);
        right.extend_from_slice(&to_a);
        right.push((0, 1));
        right.extend_from_slice(&to_b);
        right.push((0, 1));
        right.extend_from_slice(&to_c);
        right.push((0, 1));
        right.extend_from_slice(turn);
        right.extend_from_slice(&to_a);
        right.push((0, 1));
        right.extend_from_slice(&to_b_home);
        right.push((0, 1));
        right.extend_from_slice(&[(A4, 4)]);
        right.push((0, 2));
    }
    let mut t = 0;
    let mut sixteenths = Vec::new();
    for &(key, len) in &right {
        sixteenths.push(t);
        if key != 0 {
            piano.note(0, t * S, len * S + S / 2, key, 72);
        }
        t += len;
    }
    // The left hand under bars 2 to 8 (and again; each time through is
    // 48 sixteenths, after the two of the upbeat): a broken chord at the
    // start of each, the pedal down with it.
    let left: [[u8; 3]; 7] = [[45, 52, 57], [40, 52, 56], [45, 52, 57], [0, 0, 0], [45, 52, 57], [40, 52, 56], [45, 52, 57]];
    for half in 0..2u32 {
        let start = 8 + half * 48;
        for (bar, chord) in left.iter().enumerate() {
            if chord[0] == 0 {
                continue;
            }
            let t0 = (start + bar as u32 * 6) * S;
            piano.cc(0, t0, 64, 0);
            piano.cc(0, t0 + 10, 64, 127);
            for (i, &k) in chord.iter().enumerate() {
                piano.note(0, t0 + i as u32 * S, S * 3, k, 58);
            }
        }
    }
    piano.cc(0, t * S, 64, 0);
    file(vec![meta, piano], t * S + BAR)
}

fn gm_tour() -> Vec<u8> {
    let mut meta = Track::default();
    meta.tempo(0, 120);
    let mut t_ = Track::default();
    // (bank, program): a sample of every family, then the classic banks.
    let tour: [(u8, u8); 25] = [
        (0, 0), (0, 4), (0, 11), (0, 16), (0, 24), (0, 29), (0, 33), (0, 40), (0, 48), (0, 52), (0, 56), (0, 61), (0, 65),
        (0, 71), (0, 73), (0, 80), (0, 81), (0, 88), (0, 95), (0, 105), (0, 108), (0, 114), (80, 0), (81, 2), (82, 0),
    ];
    let mut t = 0;
    for (bank, program) in tour {
        let low = if matches!((bank, program), (0, 33) | (82, 0)) { 36 } else { 60 };
        t_.program(0, t, bank, program);
        for (i, k) in [0u8, 4, 7, 12].into_iter().enumerate() {
            t_.note(0, t + i as u32 * E, E, low + k, 100);
        }
        for k in [0u8, 4, 7] {
            t_.note(0, t + 2 * Q, 2 * Q - 60, low + k, 90);
        }
        t += BAR + Q;
    }
    // A bar on each of eight kits: Standard, Room, Power, Electronic, the
    // 808-style, Jazz, Brush, Orchestra.
    for kit in [0u8, 8, 16, 24, 25, 32, 40, 48] {
        t_.at(t, &[0xc9, kit]);
        for step in 0..16u32 {
            let at = t + step * S;
            t_.note(9, at, S, HAT, 80);
            if step % 8 == 0 {
                t_.note(9, at, S, KICK, 115);
            }
            if step % 8 == 4 {
                t_.note(9, at, S, SNARE, 105);
            }
            if step == 14 {
                t_.note(9, at, S, 45, 100);
            }
        }
        t += BAR;
    }
    file(vec![meta, t_], t + BAR)
}

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "demo/songs".into()));
    std::fs::create_dir_all(&dir).expect("couldn't make the folder");
    for (name, bytes) in [("overworld.mid", overworld()), ("dungeon.mid", dungeon()), ("fur-elise.mid", fur_elise()), ("gm-tour.mid", gm_tour())] {
        let song = neumetik_midi::Song::from_midi(&bytes).expect("a demo song reads");
        std::fs::write(dir.join(name), &bytes).expect("couldn't write a song");
        println!("{name}: {:.1} s, {} bytes", song.duration(), bytes.len());
    }
}
