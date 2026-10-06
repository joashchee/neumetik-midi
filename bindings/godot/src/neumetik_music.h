// Neumetik MIDI for Godot 4: a NeumetikMusic node that plays MIDI files.
// Copyright 2026 ansiapps. MIT licensed.
#pragma once

#include <godot_cpp/classes/audio_stream_generator_playback.hpp>
#include <godot_cpp/classes/audio_stream_player.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/variant/packed_vector2_array.hpp>

#include "neumetik_midi.h"

namespace godot {

// Plays a MIDI file through its own AudioStreamPlayer, fed from
// _process. Add it to a scene, then from GDScript:
//
//   $Music.load_file("res://music/overworld.mid")
//   $Music.looping = true
//   $Music.play()
class NeumetikMusic : public Node {
    GDCLASS(NeumetikMusic, Node)

    nm_player *player = nullptr;
    AudioStreamPlayer *output = nullptr;
    Ref<AudioStreamGeneratorPlayback> playback;
    PackedVector2Array frames;
    float buffer[2 * 1024];
    int mix_rate = 44100;
    StringName bus = "Master";

protected:
    static void _bind_methods();

public:
    NeumetikMusic();
    ~NeumetikMusic();

    void _ready() override;
    void _process(double delta) override;

    // The song. Returns an empty string, or why it couldn't be read.
    String load_file(const String &path);
    String load_bytes(const PackedByteArray &bytes);

    void play();
    void pause();
    void stop();
    bool is_playing() const;
    bool is_finished() const;
    void seek(double seconds);
    double get_position() const;
    double get_duration() const;

    void set_looping(bool looping);
    bool is_looping() const;
    void set_loop_points(double start, double end);
    void set_speed(double speed);
    void set_volume(float volume);
    float get_volume() const;
    void fade_to(float volume, float seconds);
    void set_channel_muted(int channel, bool muted);

    void note_on(int channel, int key, int velocity);
    void note_off(int channel, int key);
    void control_change(int channel, int controller, int value);
    void set_instrument(int channel, int bank, int program);
    void pitch_bend(int channel, int value);
    void all_notes_off();

    int get_active_voices() const;
    String get_instrument_name(int channel) const;

    void set_bus(const StringName &name);
    StringName get_bus() const;
};

} // namespace godot
