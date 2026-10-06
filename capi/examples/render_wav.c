/*
 * Plays a MIDI file into a WAV file through Neumetik MIDI's C API: the
 * whole API in one small program (load, loop, seek, fade, live notes,
 * render).
 *
 *   cc -O2 -Iinclude examples/render_wav.c target/release/libneumetik_midi.a -o render_wav
 *   ./render_wav song.mid out.wav
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "neumetik_midi.h"

#define RATE 48000
#define FRAMES 512

static void put32(FILE *f, uint32_t v) { fputc(v & 255, f); fputc(v >> 8 & 255, f); fputc(v >> 16 & 255, f); fputc(v >> 24, f); }
static void put16(FILE *f, uint16_t v) { fputc(v & 255, f); fputc(v >> 8, f); }

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s song.mid out.wav\n", argv[0]);
        return 2;
    }
    FILE *in = fopen(argv[1], "rb");
    if (!in) { perror(argv[1]); return 1; }
    fseek(in, 0, SEEK_END);
    long size = ftell(in);
    fseek(in, 0, SEEK_SET);
    uint8_t *midi = malloc(size);
    if (fread(midi, 1, size, in) != (size_t)size) { perror(argv[1]); return 1; }
    fclose(in);

    nm_player *p = nm_create(RATE);
    int32_t err = nm_load(p, midi, size);
    free(midi); /* the player keeps its own copy */
    if (err != NM_OK) {
        fprintf(stderr, "%s: %s\n", argv[1], nm_error_string(err));
        return 1;
    }
    char name[64];
    nm_instrument_name(p, 0, name, sizeof name);
    printf("%.1f s, loop %.2f to %.2f s\n", nm_duration(p), nm_loop_start(p), nm_loop_end(p));

    /* Twice through the loop, then fade out over two seconds. */
    nm_set_looping(p, 1);
    nm_play(p);
    double length = nm_loop_end(p) + (nm_loop_end(p) - nm_loop_start(p));
    size_t total = (size_t)((length + 2.0) * RATE);

    FILE *out = fopen(argv[2], "wb");
    if (!out) { perror(argv[2]); return 1; }
    uint32_t bytes = (uint32_t)(total / FRAMES * FRAMES * 4);
    fwrite("RIFF", 1, 4, out); put32(out, 36 + bytes); fwrite("WAVEfmt ", 1, 8, out);
    put32(out, 16); put16(out, 1); put16(out, 2); put32(out, RATE); put32(out, RATE * 4); put16(out, 4); put16(out, 16);
    fwrite("data", 1, 4, out); put32(out, bytes);

    int16_t buffer[FRAMES * 2];
    int faded = 0, peak_voices = 0;
    for (size_t done = 0; done + FRAMES <= total; done += FRAMES) {
        if (!faded && done >= (size_t)(length * RATE)) {
            nm_fade_to(p, 0.0f, 2.0f);
            /* A sting over the music: a bell on a free channel. */
            nm_set_instrument(p, 15, 0, 14);
            nm_note_on(p, 15, 84, 110);
            faded = 1;
        }
        nm_render_s16(p, buffer, FRAMES);
        for (int i = 0; i < FRAMES * 2; i++) {
            put16(out, (uint16_t)buffer[i]);
        }
        if (nm_active_voices(p) > peak_voices) peak_voices = nm_active_voices(p);
    }
    fclose(out);
    printf("wrote %s: %.1f s, first channel %s, %d voices at most\n", argv[2], (double)total / RATE, name, peak_voices);
    nm_destroy(p);
    return 0;
}
