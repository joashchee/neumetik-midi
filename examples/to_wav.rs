//! A MIDI file to a WAV file: `cargo run --release --example to_wav -- song.mid song.wav`.
//! With `--loops N`, it plays the loop N more times and fades out.

use neumetik_midi::Player;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        eprintln!("usage: to_wav song.mid song.wav [--loops N]");
        std::process::exit(2);
    };
    let loops: u32 = args.iter().position(|a| a == "--loops").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(0);

    const RATE: u32 = 48_000;
    let mut player = Player::new(RATE);
    player.load(&std::fs::read(input)?)?;
    let (start, end) = player.loop_points();
    player.set_looping(loops > 0);
    player.play();

    // As long as the song (and its loops), then a fade or a tail.
    let music = if loops > 0 { end + (end - start) * f64::from(loops) } else { player.duration() };
    let frames = ((music + 3.0) * f64::from(RATE)) as usize;
    let mut samples = vec![0f32; frames * 2];
    let fade_at = (music * f64::from(RATE)) as usize * 2;
    let (before, after) = samples.split_at_mut(fade_at.min(frames * 2));
    player.render_interleaved(before);
    if loops > 0 {
        player.fade_to(0.0, 3.0);
    }
    player.render_interleaved(after);

    std::fs::write(output, neumetik_midi::wav(&samples, RATE))?;
    println!("{output}: {:.1} s", frames as f64 / f64::from(RATE));
    Ok(())
}
