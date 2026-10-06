# Changelog

## 0.1.0 (2026-10-06)

The first release: Neumetik's MIDI synthesizer as a crate of its own,
MIT licensed, for games.

- **`Player`**: a song with play, pause, stop and seek (every channel's
  instrument and controllers set as the song has them there), looping
  between the file's `loopStart`/`loopEnd` markers (or CC 111) or points
  you set, speed without a change of pitch, volume and fades, channels
  muted for music in layers, and `is_finished`.
- **`Synth`, played live**: `note_on`, `note_off`, `control_change`,
  `program_change`, `set_instrument`, `pitch_bend`, raw MIDI bytes with
  `send`, `all_notes_off`, `all_sound_off`, `reset`, and looking in with
  `active_voices`, `instrument_name` and `memory_bytes`.
- **Output** as left and right buffers, interleaved floats, or
  interleaved 16-bit samples, from `Player`, `Synth` and `Sequencer`;
  `wav` writes a WAV file.
- **`Error`** for a MIDI file that won't read, in place of a `String`
  (it still turns into one with `?`).
- **The C API** (`capi/`, `neumetik_midi.h`): static and shared
  libraries, and the same functions in WebAssembly.
- **Bindings**: JavaScript (with an AudioWorklet node, for Phaser and any
  Web Audio game), LÖVE (LuaJIT FFI), C# (MonoGame, FNA, Unity, Godot C#),
  a Godot 4 GDExtension, Qt (a `QIODevice`) and raylib.
- **Fixed**: the sequencer freed SysEx data while rendering; it now keeps
  it in a pool it trims when a file is queued. Nothing renders through an
  allocation or a free.
- **Hardened**: sample rates are clamped to 8 to 192 kHz, out-of-range
  channels, keys and values are ignored or clamped, and NaN volumes,
  speeds and times are refused, so nothing a caller passes panics.
- A demo page (`demo/`): the songs, live play, an A/B against any
  SoundFont, benchmarks and sizes, and how it compares with the other
  options.
