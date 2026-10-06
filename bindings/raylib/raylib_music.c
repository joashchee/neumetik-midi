/*
 * MIDI music in a raylib game through Neumetik MIDI: an AudioStream fed
 * from the game loop. Copyright 2026 ansiapps. MIT licensed.
 *
 *   cc raylib_music.c -I../../capi/include ../../capi/target/release/libneumetik_midi.a \
 *      $(pkg-config --libs --cflags raylib) -o raylib_music
 *   ./raylib_music overworld.mid
 *
 * Space pauses, F fades, M mutes the drums, Z X C play notes over it.
 */
#include <stdio.h>

#include "neumetik_midi.h"
#include "raylib.h"

#define RATE 44100
#define FRAMES 1024

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "overworld.mid";
    InitWindow(640, 200, "Neumetik MIDI + raylib");
    InitAudioDevice();
    SetAudioStreamBufferSizeDefault(FRAMES);
    AudioStream stream = LoadAudioStream(RATE, 16, 2);

    int size = 0;
    unsigned char *midi = LoadFileData(path, &size);
    nm_player *music = nm_create(RATE);
    int32_t err = nm_load(music, midi, (size_t)size);
    UnloadFileData(midi);
    const char *problem = err == NM_OK ? NULL : nm_error_string(err);

    nm_set_looping(music, 1);
    nm_play(music);
    nm_set_instrument(music, 15, 0, 11); /* a vibraphone for the notes played over it */
    PlayAudioStream(stream);

    short buffer[FRAMES * 2];
    int faded = 0, muted = 0;
    SetTargetFPS(60);
    while (!WindowShouldClose()) {
        /* Feed the stream whenever it has room: the game loop is the audio's
         * only thread, so no locking is needed. */
        while (IsAudioStreamProcessed(stream)) {
            nm_render_s16(music, buffer, FRAMES);
            UpdateAudioStream(stream, buffer, FRAMES);
        }
        if (IsKeyPressed(KEY_SPACE)) {
            if (nm_state(music) == NM_PLAYING) nm_pause(music); else nm_play(music);
        }
        if (IsKeyPressed(KEY_F)) { faded = !faded; nm_fade_to(music, faded ? 0.0f : 1.0f, 1.5f); }
        if (IsKeyPressed(KEY_M)) { muted = !muted; nm_set_channel_muted(music, 9, muted); }
        const int keys[3] = { KEY_Z, KEY_X, KEY_C }, notes[3] = { 72, 76, 79 };
        for (int i = 0; i < 3; i++) {
            if (IsKeyPressed(keys[i])) nm_note_on(music, 15, notes[i], 110);
            if (IsKeyReleased(keys[i])) nm_note_off(music, 15, notes[i]);
        }

        BeginDrawing();
        ClearBackground(RAYWHITE);
        if (problem) {
            DrawText(TextFormat("%s: %s", path, problem), 20, 20, 20, MAROON);
        } else {
            DrawText(TextFormat("%.1f / %.1f s   voices %d", nm_position(music), nm_duration(music), nm_active_voices(music)), 20, 20, 20, DARKGRAY);
            DrawText("Space pause  F fade  M mute drums  Z X C notes", 20, 60, 20, GRAY);
        }
        EndDrawing();
    }
    nm_destroy(music);
    UnloadAudioStream(stream);
    CloseAudioDevice();
    CloseWindow();
    return 0;
}
