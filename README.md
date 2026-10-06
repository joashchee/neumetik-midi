# Neumetik MIDI

**MIDI music for games, with no SoundFont.** A General MIDI and Roland GS
synthesizer where every instrument is synthesized as it plays (subtractive,
FM and plucked-string engines, and a recipe for every drum), so there are no
samples to download. It turns MIDI into samples, and your engine plays them.

- **Small:** 65 KB of WebAssembly, gzipped. Linked into a native program it
  adds about 400 KB.
- **Light:** under 1% of one CPU core for typical game music at 48 kHz, and
  about 1.2 MB of memory, allocated once. Rendering never allocates, locks
  or panics.
- **No dependencies:** no SoundFont, no audio API, nothing to install. You
  get float or 16-bit PCM, interleaved or not.
- **Made for games:** loops (from the file's markers or your own points),
  seeking, fades, tempo, channels muted and unmuted for music in layers, and
  notes played live over the music.
- **Everywhere:** Rust, a C API, WebAssembly, and bindings for JavaScript
  (Phaser and any Web Audio game), LÖVE, C# (MonoGame, FNA, Unity, Godot),
  Godot (GDScript), Qt and raylib.
- **Free:** MIT licensed.

Hear it, play it, compare it with a SoundFont and see the benchmarks on the
demo page (`demo/`; see [Building](#building)).

## Contents

- [Quick start](#quick-start)
- [Your engine](#your-engine)
- [How it works](#how-it-works) (players, loops, seeking, layers, playing live, threads, levels)
- [The C API](#the-c-api)
- [What it plays](#what-it-plays)
- [Size and speed](#size-and-speed)
- [Building](#building)
- [Limits](#limits)

## Quick start

### Rust

```toml
[dependencies]
neumetik-midi = "0.1"
```

```rust
use neumetik_midi::Player;

let mut music = Player::new(48_000);
music.load(&std::fs::read("overworld.mid")?)?;
music.set_looping(true);   // between the file's loop markers, or the whole song
music.play();

// In your audio callback (or whenever your engine wants more):
let mut buffer = [0f32; 1024];          // 512 frames, left and right interleaved
music.render_interleaved(&mut buffer);  // or render(&mut left, &mut right), render_i16
```

### C or C++

```c
#include "neumetik_midi.h"   /* link libneumetik_midi.a, or the .so / .dylib / .dll */

nm_player *music = nm_create(48000);
int32_t err = nm_load(music, midi_bytes, midi_size);
if (err != NM_OK) printf("%s\n", nm_error_string(err));
nm_set_looping(music, 1);
nm_play(music);

nm_render_f32(music, buffer, frames);   /* interleaved stereo; or nm_render_s16 */

nm_destroy(music);
```

`capi/examples/render_wav.c` plays a MIDI file into a WAV file using most of
the API.

### JavaScript

```js
import { createMusicNode } from './neumetik-midi.js';   // neumetik_midi.wasm beside it

const music = await createMusicNode(audioContext);       // an AudioWorklet
music.node.connect(audioContext.destination);
await music.load(await (await fetch('overworld.mid')).arrayBuffer());
music.setLooping(true);
music.play();
```

## Your engine

| Engine | Use | Where |
| --- | --- | --- |
| Any Rust engine (Bevy, macroquad, ggez, cpal) | the crate | this folder |
| C / C++ (SDL, SFML, miniaudio, your own) | `neumetik_midi.h` + the library | `capi/` |
| raylib | an `AudioStream` fed from the game loop | `bindings/raylib/raylib_music.c` |
| Phaser, PixiJS, three.js, plain Web Audio | `createMusicNode` (AudioWorklet) or `renderToAudioBuffer` | `bindings/js/` |
| LÖVE | `neumetik_midi.lua` (LuaJIT FFI, a `QueueableSource`) | `bindings/love/` |
| MonoGame, FNA, Unity, Godot C#, .NET | `NeumetikMidi.cs` (P/Invoke) | `bindings/csharp/` |
| Godot (GDScript) | the `NeumetikMusic` node, a GDExtension | `bindings/godot/` |
| Qt 6 | `NeumetikMidiDevice`, a `QIODevice` for `QAudioSink` | `bindings/qt/` |
| Anything with a C FFI (Python ctypes, Zig, Nim, Odin, Go) | the C API | `capi/include/neumetik_midi.h` |

The Rust crate, the C API, WebAssembly, the JavaScript binding and the LÖVE
binding are tested. The C#, Godot, Qt and raylib bindings are written to
those engines' APIs but haven't been built against them in CI yet.

### Phaser

```js
import { createMusicNode } from './neumetik-midi.js';

class Level extends Phaser.Scene {
  async create() {
    this.music = await createMusicNode(this.sound.context);
    this.music.node.connect(this.sound.masterVolumeNode);   // follows Phaser's volume and mute
    await this.music.load(await (await fetch('assets/overworld.mid')).arrayBuffer());
    this.music.setLooping(true);
    this.music.play();
  }
  bossAppears() {
    this.music.fadeTo(0, 1.5);
  }
}
```

Browsers start audio only after the player clicks or presses a key, and
Phaser resumes its context then. `createMusicNode` needs the page served
over HTTP (an AudioWorklet can't load from `file://`). For music as an
ordinary sound instead, `renderToAudioBuffer(context, midi, { loops: 2 })`
renders it whole.

### LÖVE

Put `neumetik_midi.lua` and the library for each platform
(`libneumetik_midi.dylib`, `.so`, `neumetik_midi.dll`) beside `main.lua`:

```lua
local NM = require("neumetik_midi")

function love.load()
  music = NM.new()                                   -- 44.1 kHz, a QueueableSource
  assert(music:load(love.filesystem.read("overworld.mid")))
  music:setLooping(true)
  music:play()
end

function love.update(dt)
  music:update()                                     -- keeps about 93 ms queued
end
```

`bindings/love/main.lua` is a whole example: pause, fades, muting the drums,
and notes played on the keyboard.

### MonoGame, Unity, Godot C#

See `bindings/csharp/README.md`. In short:

```csharp
var player = new NeumetikMidi.Player(44100);
player.Load(File.ReadAllBytes("overworld.mid"));
player.Looping = true;
player.Play();
player.Render(buffer);   // float[], short[] or byte[] (16-bit, for DynamicSoundEffectInstance)
```

### Godot (GDScript)

Build the GDExtension (`bindings/godot/README.md`), then:

```gdscript
@onready var music: NeumetikMusic = $Music

func _ready():
    music.load_file("res://music/overworld.mid")
    music.looping = true
    music.play()
```

### raylib

```c
AudioStream stream = LoadAudioStream(44100, 16, 2);
nm_player *music = nm_create(44100);
nm_load(music, data, size);
nm_play(music);
PlayAudioStream(stream);
/* each frame: */
while (IsAudioStreamProcessed(stream)) {
    nm_render_s16(music, buffer, 1024);
    UpdateAudioStream(stream, buffer, 1024);
}
```

Set `SetAudioStreamBufferSizeDefault(1024)` before `LoadAudioStream` so the
two agree. See `bindings/raylib/raylib_music.c`.

## How it works

### Three ways in

- **`Player`** plays a song: load, play, pause, stop, seek, loop, speed,
  volume, fades, mutes. Use it for a game's music.
- **`Synth`** is the synthesizer alone, driven by MIDI messages as they
  come. Every player has one (`player.synth()`, `player.synth_mut()`), so
  you can play notes over the music.
- **`Sequencer`** queues MIDI files back to back with no gap. Use it for
  music written as it plays.

`render(bytes, rate, tail, longest)` renders a whole file at once, and `wav`
turns samples into a WAV file.

### Loops

`set_looping(true)` loops between the song's loop points:

1. points you set with `set_loop_points(start, end)` (in seconds);
2. otherwise the file's own: a marker named `loopStart` and one named
   `loopEnd` (any case, with or without a space, `_` or `-`), or CC 111 at
   the loop's start (as RPG Maker writes it). A start with no end loops to
   the end of the song;
3. otherwise the whole song, from its start to its last End of Track.

At the loop's end, held notes are let go, every channel returns to the
instrument and controller settings it has at the loop's start, and playing
carries on from there with no gap. The reverb and the release of the last
notes ring on into the loop.

Tips for MIDI files that loop well:

- End the loop on a bar line: put the `loopEnd` marker there, or make the
  track's End of Track fall there.
- Most DAWs can export markers in a MIDI file. Name them `loopStart` and
  `loopEnd`.
- Set each channel's instrument and volume before the loop starts, or again
  at its start.

### Seeking

`seek(seconds)` silences what's sounding, then works out what every channel
plays at that moment (program changes, bank selects, controllers, pitch
bend, SysEx), so the music resumes with the right instruments. It doesn't
resume a note held across that moment: it waits for the next one.

### Layers, fades and speed

- `set_channel_muted(channel, true)` keeps a channel playing unheard, so it
  comes back in time when unmuted. Write music in layers (drums on 9, bass
  on 1, and so on) and bring them in and out as the game changes.
- `fade_to(volume, seconds)` fades smoothly. Fading to 0 keeps the song
  playing, silent.
- `set_speed(1.5)` plays half as fast again, from 0.25× to 4×, without
  changing the pitch.

### Playing live

`note_on`, `note_off`, `control_change`, `program_change`,
`set_instrument(channel, bank, program)` and `pitch_bend`, or raw MIDI bytes
with `send` (running status and SysEx included, so a MIDI keyboard's input
can go straight in). Over a song, use a channel the song doesn't use
(15 is usually free). Channels count from 0, and 9 is the drums.

```rust
let s = music.synth_mut();
s.set_instrument(15, 0, 11);   // a vibraphone
s.note_on(15, 84, 110);        // a sting over the music
```

### Threads and audio callbacks

A player has no locks of its own. Call it from one thread at a time. Most
engines feed audio from the game loop (LÖVE's `QueueableSource`, raylib's
`UpdateAudioStream`, Godot's `AudioStreamGenerator`, MonoGame's
`BufferNeeded`, Qt's `QAudioSink`), so there's nothing to do. If your audio
callback runs on its own thread (miniaudio, SDL's callback, cpal, Unity's
`OnAudioFilterRead`), guard the player with a mutex. The calls are short,
and rendering never blocks. The C# binding takes a lock for you. In the
browser, `createMusicNode` runs the player in the AudioWorklet and passes
your calls to it as messages.

`Song::from_midi` reads a file without a player, so a game can read its
songs while loading and hand them over with `load_song` later.

### Levels

The synthesizer leaves headroom: the demo songs peak between -11 and
-16 dBFS, and all 64 voices at once peak around -2.5 dBFS. Use `set_volume` (up to 4) if it's
quieter than your sound effects. `render_i16` clips at full scale.

### Determinism

The same MIDI at the same sample rate always makes the same samples on the
same platform. Its noise is seeded, not random.

## The C API

`capi/include/neumetik_midi.h` documents every function. In brief:

| | |
| --- | --- |
| Make | `nm_create(rate)`, `nm_destroy`, `nm_sample_rate` |
| Song | `nm_load(p, bytes, size)` → `NM_OK` or `NM_ERR_*`, `nm_error_string`, `nm_unload` |
| Play | `nm_play`, `nm_pause`, `nm_stop`, `nm_state`, `nm_is_finished` |
| Where | `nm_seek`, `nm_position`, `nm_duration` |
| Loop | `nm_set_looping`, `nm_is_looping`, `nm_set_loop_points(start, end)` (negative: the file's own), `nm_loop_start`, `nm_loop_end` |
| Shape | `nm_set_speed`, `nm_set_volume`, `nm_volume`, `nm_fade_to`, `nm_set_channel_muted` |
| Live | `nm_note_on`, `nm_note_off`, `nm_control_change`, `nm_program_change`, `nm_set_instrument`, `nm_pitch_bend`, `nm_send_midi`, `nm_all_notes_off`, `nm_all_sound_off`, `nm_reset` |
| Render | `nm_render_f32` (interleaved), `nm_render_s16`, `nm_render_f32_planar` |
| Look | `nm_active_voices`, `nm_instrument_name(p, channel, buffer, size)` |

Every function accepts a NULL player and does nothing. Out-of-range values
are clamped or ignored. The WebAssembly module exports the same functions,
plus `nm_alloc` and `nm_free` for passing bytes in. It needs no imports.

## What it plays

- Standard MIDI Files, formats 0, 1 and 2, and RIFF RMID. A damaged track
  plays as far as it reads.
- **Bank 0, General MIDI:** all 128 instruments.
- **Banks 1 to 63, Roland GS:** the SC-55's variation tones. A variation it
  doesn't have plays the GM tone, as the SC-55 did.
- **Banks 80, 81, 82:** classic synth sounds of the eighties, the nineties,
  and the 2000s on.
- **Drums** on channel 9 (MIDI's 10), or any channel GS makes a drum part.
  Every key 27 to 87 is covered. Kits by program: 0 Standard, 8 Room,
  16 Power, 24 Electronic, 25 Analog Boom, 26 Dance, 27 Rhythm Box,
  28 Analog Tick, 29 Digital Machine, 30 Analog Punch, 32 Jazz, 40 Brush,
  48 Orchestra, 56 SFX, 64 Studio Machine, 72 Hex Pads. XG's drum bank
  (127) works too.
- **Controllers:**

  | CC | Does |
  | --- | --- |
  | 0 | bank select |
  | 1 | modulation |
  | 6/38 | data entry |
  | 7 | volume |
  | 10 | pan |
  | 11 | expression |
  | 64 | sustain |
  | 71 | resonance |
  | 74 | brightness |
  | 91 | reverb |
  | 93 | chorus |
  | 100/101 | RPN: bend range, fine and coarse tuning |
  | 120 | all sound off |
  | 121 | reset controllers |
  | 123–127 | all notes off |

  Pitch bend is supported.
- **SysEx:** GM System On, GS reset, GS Use For Rhythm Part, master volume.
- 64 voices. When they run out, the quietest released voice is stolen
  first, then the quietest of all.

## Size and speed

Measured on Apple Silicon (aarch64, macOS) at 48 kHz in 512-frame buffers
by `cargo run --release --example bench`. `scripts/build.sh` measures
again, and the demo page shows the latest numbers along with a benchmark
run in your browser.

| | |
| --- | --- |
| WebAssembly | 191 KB, 65 KB gzipped |
| Added to a C program, linked statically (stripped) | about 409 KB |
| Shared library | 426 KB |
| Memory, one player | 1.15 MB (64 voices, chorus, reverb) |
| Typical game music | 0.25–0.7% of one core (140–400× real time) |
| All 64 voices busy | 4.9% of one core (21× real time) |

The native figure includes the parts of Rust's standard library that get
linked in.

## Building

You need Rust (1.75 or newer). `rustup target add wasm32-unknown-unknown`
for the WebAssembly build.

```sh
cargo test                                         # the crate
cargo run --release --example make_demos -- songs  # the demo songs, as MIDI files
cargo run --release --example to_wav -- song.mid song.wav --loops 2
cargo run --release --example live                 # live.wav: notes, bends, drums, no file
cargo run --release --example bench -- songs       # how fast, how loud

scripts/build.sh                                   # everything: dist/ and the demo page
python3 -m http.server -d demo 8000                # then open http://localhost:8000
```

`dist/` gets the header, the static and shared libraries for the machine
you built on, the WebAssembly module and JavaScript, the LÖVE and C#
bindings, and the demo songs. For other platforms, build the C API with
their targets:

```sh
cargo build --release --manifest-path capi/Cargo.toml --target x86_64-pc-windows-msvc   # neumetik_midi.dll / .lib
cargo build --release --manifest-path capi/Cargo.toml --target x86_64-unknown-linux-gnu
cargo build --release --manifest-path capi/Cargo.toml --target aarch64-apple-ios        # a static library for iOS
cargo build --release --manifest-path capi/Cargo.toml --target aarch64-linux-android    # with the NDK's linker set up
```

On Windows, linking the static library also needs `ws2_32 userenv ntdll bcrypt advapi32`.
On Linux, it needs `pthread dl m`.

The demo page's A/B comparison plays SoundFonts through
[rustysynth](https://github.com/sinshu/rustysynth) (MIT), built from
`demo/ab-soundfont/`. That code is only for the comparison and isn't part
of Neumetik MIDI.

## Limits

- No SoundFonts or DLS: this is the synthesizer you use instead of them.
  For sampled realism, use a SoundFont player.
- General MIDI and GS, not GM2 or XG voice banks (XG's drum bank aside).
  No NRPN sound editing, no MIDI 2.0.
- One song per player. For two at once (a crossfade, say), use two players
  and mix them.
- The sound is its own: close to a Sound Canvas in spirit, not a copy of
  one.

## License

MIT: see `LICENSE`. The demo songs were written for this project and are
free to use under the same license. "Für Elise" is Beethoven's, in the
public domain.
