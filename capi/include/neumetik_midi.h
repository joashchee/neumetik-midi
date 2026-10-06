/*
 * Neumetik MIDI: General MIDI and GS music for games, no SoundFont.
 * Copyright 2026 ansiapps. MIT licensed: see LICENSE.
 *
 * Link libneumetik_midi (static: libneumetik_midi.a / neumetik_midi.lib;
 * shared: .so / .dylib / .dll). It plays no sound itself: it turns MIDI
 * into samples, and your engine plays them.
 *
 *     nm_player *p = nm_create(48000);
 *     if (nm_load(p, midi_bytes, midi_size) != 0) { ...error... }
 *     nm_set_looping(p, 1);
 *     nm_play(p);
 *     // whenever the audio stream wants more:
 *     nm_render_f32(p, buffer, frames);   // or nm_render_s16
 *     ...
 *     nm_destroy(p);
 *
 * Threads: a player is not locked. Call it from one thread at a time,
 * or guard it with your own mutex if your audio callback runs on another
 * thread. Rendering never allocates, locks or blocks.
 *
 * Every function accepts a NULL player and does nothing (returning 0).
 * Channels are 0 to 15 (9 is the drums); keys, velocities, controllers
 * and programs are 0 to 127. Values out of range are clamped or ignored.
 */
#ifndef NEUMETIK_MIDI_H
#define NEUMETIK_MIDI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct nm_player nm_player;

/* Error codes nm_load returns (0 is success). */
#define NM_OK               0
#define NM_ERR_NOT_MIDI    -1  /* it isn't a MIDI file */
#define NM_ERR_TRUNCATED   -2  /* it ends too soon */
#define NM_ERR_BAD_TIMING  -3  /* its timing is zero */
#define NM_ERR_NO_MUSIC    -4  /* it has no music in it */
#define NM_ERR_NO_PLAYER   -5  /* the player was NULL */

/* What nm_state returns. */
#define NM_STOPPED 0
#define NM_PLAYING 1
#define NM_PAUSED  2

/* The API version: 100 for 1.00. */
uint32_t nm_version(void);

/* ---- Creating ---------------------------------------------------- */

/* A player at sample_rate frames a second (8000 to 192000; others are
 * clamped). Allocates about 1 MB, once. */
nm_player *nm_create(uint32_t sample_rate);
void nm_destroy(nm_player *p);
uint32_t nm_sample_rate(const nm_player *p);

/* ---- The song ---------------------------------------------------- */

/* Reads a Standard MIDI File (format 0, 1, 2 or RIFF RMID) and loads it,
 * stopped at its start. The bytes are copied; free them when it returns.
 * Returns NM_OK or a negative NM_ERR_ code. */
int32_t nm_load(nm_player *p, const uint8_t *data, size_t size);
/* A message for an error code, in English. Never NULL; don't free it. */
const char *nm_error_string(int32_t code);
void nm_unload(nm_player *p);

/* ---- Playing ----------------------------------------------------- */

/* Plays from where it's paused or stopped (from the start if it played
 * to its end). */
void nm_play(nm_player *p);
/* Holds it where it is, silent; nm_play carries on. */
void nm_pause(nm_player *p);
/* Stops, silences it, and goes back to the start. */
void nm_stop(nm_player *p);
/* NM_STOPPED, NM_PLAYING or NM_PAUSED. */
int32_t nm_state(const nm_player *p);
/* 1 once it has played to its end (not looping). */
int32_t nm_is_finished(const nm_player *p);

/* Goes to a time in seconds, every instrument and controller set as the
 * song has them there. */
void nm_seek(nm_player *p, double seconds);
double nm_position(const nm_player *p);
double nm_duration(const nm_player *p);

/* Loops between the loop points: the file's own (a "loopStart" and
 * "loopEnd" marker, or CC 111), else the whole song. */
void nm_set_looping(nm_player *p, int32_t looping);
int32_t nm_is_looping(const nm_player *p);
/* Sets the loop points in seconds; a negative value goes back to the
 * file's own (or the song's start or end). */
void nm_set_loop_points(nm_player *p, double start, double end);
double nm_loop_start(const nm_player *p);
double nm_loop_end(const nm_player *p);

/* Tempo: 1 as written, 0.25 to 4. The pitch doesn't change. */
void nm_set_speed(nm_player *p, double speed);
/* Volume: 0 to 1 (up to 4). */
void nm_set_volume(nm_player *p, float volume);
float nm_volume(const nm_player *p);
/* Fades the volume to `volume` over `seconds`. Fading to 0 keeps it
 * playing, silent. */
void nm_fade_to(nm_player *p, float volume, float seconds);
/* Mutes a channel; it plays on unheard, for music in layers. */
void nm_set_channel_muted(nm_player *p, int32_t channel, int32_t muted);

/* ---- Playing live, over the song or alone -------------------------- */

void nm_note_on(nm_player *p, int32_t channel, int32_t key, int32_t velocity);
void nm_note_off(nm_player *p, int32_t channel, int32_t key);
void nm_control_change(nm_player *p, int32_t channel, int32_t controller, int32_t value);
void nm_program_change(nm_player *p, int32_t channel, int32_t program);
/* Bank and program at once: bank 0 is General MIDI, 1 to 63 the GS
 * variations, 80 to 82 classic synths. On a drum channel, the kit. */
void nm_set_instrument(nm_player *p, int32_t channel, int32_t bank, int32_t program);
/* -8192 to 8191, 0 in tune; two semitones each way unless changed. */
void nm_pitch_bend(nm_player *p, int32_t channel, int32_t value);
/* Raw MIDI bytes: one message or many, running status allowed, SysEx
 * from F0 to F7. */
void nm_send_midi(nm_player *p, const uint8_t *data, size_t size);
/* Lets every note go. */
void nm_all_notes_off(nm_player *p);
/* Silences everything at once. */
void nm_all_sound_off(nm_player *p);
/* Every channel back to General MIDI's defaults, and silence. */
void nm_reset(nm_player *p);

/* ---- Rendering --------------------------------------------------- */

/* Interleaved stereo (left, right, left...): `frames` frames, so
 * 2 * frames samples. */
void nm_render_f32(nm_player *p, float *out, size_t frames);
void nm_render_s16(nm_player *p, int16_t *out, size_t frames);
/* Left and right in buffers of their own, `frames` samples each. */
void nm_render_f32_planar(nm_player *p, float *left, float *right, size_t frames);

/* ---- Looking in -------------------------------------------------- */

/* Voices sounding now (at most 64). */
int32_t nm_active_voices(const nm_player *p);
/* Copies the name of what a channel plays (an instrument, or a drum kit)
 * into buffer, cut to fit and NUL-ended; returns its full length. */
int32_t nm_instrument_name(const nm_player *p, int32_t channel, char *buffer, size_t size);

#ifdef __cplusplus
}
#endif

#endif /* NEUMETIK_MIDI_H */
