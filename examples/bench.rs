//! How fast Neumetik MIDI plays, and how loud: each MIDI file in a folder
//! played through a `Player` at 48 kHz in 512-frame buffers, as a game
//! would, then every voice busy at once.
//!
//! `cargo run --release --example bench -- demo/songs [--json out.json]`

use std::time::Instant;

use neumetik_midi::{Player, Synth, POLYPHONY};

const RATE: u32 = 48_000;
const BUFFER: usize = 512;

struct Row {
    name: String,
    seconds: f64,
    took: f64,
    voices: usize,
    peak: f32,
}

fn measure(name: &str, bytes: &[u8]) -> Option<Row> {
    let mut p = Player::new(RATE);
    p.load(bytes).ok()?;
    p.play();
    let mut buf = [0f32; BUFFER * 2];
    let (mut frames, mut voices, mut peak) = (0usize, 0, 0f32);
    let start = Instant::now();
    while p.is_playing() {
        p.render_interleaved(&mut buf);
        frames += BUFFER;
        voices = voices.max(p.synth().active_voices());
        peak = buf.iter().fold(peak, |m, s| m.max(s.abs()));
    }
    let took = start.elapsed().as_secs_f64();
    Some(Row { name: name.into(), seconds: frames as f64 / f64::from(RATE), took, voices, peak })
}

/// Every voice busy: 64 notes held on four channels (strings, piano, saw
/// lead, bass), and a drum hit every beat.
fn stress() -> Row {
    let mut s = Synth::new(RATE);
    for (ch, program) in [(0u8, 48u8), (1, 0), (2, 81), (3, 33)] {
        s.program_change(ch, program);
        for k in 0..16 {
            s.note_on(ch, 40 + k * 2, 90);
        }
    }
    let mut buf = [0f32; BUFFER * 2];
    let blocks = RATE as usize * 10 / BUFFER;
    let (mut voices, mut peak) = (0, 0f32);
    let start = Instant::now();
    for b in 0..blocks {
        if b % 47 == 0 {
            s.note_on(9, 36, 100);
            // Keep every voice sounding: a note let go comes back.
            for ch in 0..4u8 {
                for k in 0..16 {
                    s.note_on(ch, 40 + k * 2, 90);
                }
            }
        }
        s.render_interleaved(&mut buf);
        voices = voices.max(s.active_voices());
        peak = buf.iter().fold(peak, |m, x| m.max(x.abs()));
    }
    let took = start.elapsed().as_secs_f64();
    Row { name: format!("stress: {POLYPHONY} voices"), seconds: (blocks * BUFFER) as f64 / f64::from(RATE), took, voices, peak }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = args.first().cloned().unwrap_or_else(|| "demo/songs".into());
    let json = args.iter().position(|a| a == "--json").and_then(|i| args.get(i + 1)).cloned();

    let mut files: Vec<_> = std::fs::read_dir(&dir).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mid")));
    files.sort();
    let mut rows: Vec<Row> = files
        .iter()
        .filter_map(|f| measure(&f.file_name()?.to_string_lossy(), &std::fs::read(f).ok()?))
        .collect();
    rows.push(stress());

    let memory = Synth::new(RATE).memory_bytes();
    println!("Neumetik MIDI {} at {} Hz, {}-frame buffers; synth memory {:.2} MB", neumetik_midi::VERSION, RATE, BUFFER, memory as f64 / 1e6);
    println!("{:<24} {:>8} {:>9} {:>11} {:>7} {:>7}", "song", "seconds", "x real", "% one core", "voices", "peak");
    for r in &rows {
        let x = r.seconds / r.took;
        println!("{:<24} {:>8.1} {:>9.0} {:>11.2} {:>7} {:>7.2}", r.name, r.seconds, x, 100.0 / x, r.voices, r.peak);
    }

    if let Some(path) = json {
        let items: Vec<String> = rows
            .iter()
            .map(|r| {
                let x = r.seconds / r.took;
                format!(
                    "{{\"name\":\"{}\",\"seconds\":{:.2},\"realtime\":{:.1},\"cpu_percent\":{:.3},\"voices\":{},\"peak\":{:.3}}}",
                    r.name, r.seconds, x, 100.0 / x, r.voices, r.peak
                )
            })
            .collect();
        let out = format!(
            "{{\"version\":\"{}\",\"rate\":{},\"buffer\":{},\"memory_bytes\":{},\"arch\":\"{}\",\"os\":\"{}\",\"songs\":[{}]}}\n",
            neumetik_midi::VERSION,
            RATE,
            BUFFER,
            memory,
            std::env::consts::ARCH,
            std::env::consts::OS,
            items.join(",")
        );
        std::fs::write(&path, out).expect("couldn't write the JSON");
    }
}
