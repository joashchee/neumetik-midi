// Neumetik MIDI for Godot 4. Copyright 2026 ansiapps. MIT licensed.
#include "neumetik_music.h"

#include <godot_cpp/classes/audio_stream_generator.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/core/class_db.hpp>

using namespace godot;

void NeumetikMusic::_bind_methods() {
    ClassDB::bind_method(D_METHOD("load_file", "path"), &NeumetikMusic::load_file);
    ClassDB::bind_method(D_METHOD("load_bytes", "bytes"), &NeumetikMusic::load_bytes);
    ClassDB::bind_method(D_METHOD("play"), &NeumetikMusic::play);
    ClassDB::bind_method(D_METHOD("pause"), &NeumetikMusic::pause);
    ClassDB::bind_method(D_METHOD("stop"), &NeumetikMusic::stop);
    ClassDB::bind_method(D_METHOD("is_playing"), &NeumetikMusic::is_playing);
    ClassDB::bind_method(D_METHOD("is_finished"), &NeumetikMusic::is_finished);
    ClassDB::bind_method(D_METHOD("seek", "seconds"), &NeumetikMusic::seek);
    ClassDB::bind_method(D_METHOD("get_position"), &NeumetikMusic::get_position);
    ClassDB::bind_method(D_METHOD("get_duration"), &NeumetikMusic::get_duration);
    ClassDB::bind_method(D_METHOD("set_looping", "looping"), &NeumetikMusic::set_looping);
    ClassDB::bind_method(D_METHOD("is_looping"), &NeumetikMusic::is_looping);
    ClassDB::bind_method(D_METHOD("set_loop_points", "start", "end"), &NeumetikMusic::set_loop_points);
    ClassDB::bind_method(D_METHOD("set_speed", "speed"), &NeumetikMusic::set_speed);
    ClassDB::bind_method(D_METHOD("set_volume", "volume"), &NeumetikMusic::set_volume);
    ClassDB::bind_method(D_METHOD("get_volume"), &NeumetikMusic::get_volume);
    ClassDB::bind_method(D_METHOD("fade_to", "volume", "seconds"), &NeumetikMusic::fade_to);
    ClassDB::bind_method(D_METHOD("set_channel_muted", "channel", "muted"), &NeumetikMusic::set_channel_muted);
    ClassDB::bind_method(D_METHOD("note_on", "channel", "key", "velocity"), &NeumetikMusic::note_on);
    ClassDB::bind_method(D_METHOD("note_off", "channel", "key"), &NeumetikMusic::note_off);
    ClassDB::bind_method(D_METHOD("control_change", "channel", "controller", "value"), &NeumetikMusic::control_change);
    ClassDB::bind_method(D_METHOD("set_instrument", "channel", "bank", "program"), &NeumetikMusic::set_instrument);
    ClassDB::bind_method(D_METHOD("pitch_bend", "channel", "value"), &NeumetikMusic::pitch_bend);
    ClassDB::bind_method(D_METHOD("all_notes_off"), &NeumetikMusic::all_notes_off);
    ClassDB::bind_method(D_METHOD("get_active_voices"), &NeumetikMusic::get_active_voices);
    ClassDB::bind_method(D_METHOD("get_instrument_name", "channel"), &NeumetikMusic::get_instrument_name);
    ClassDB::bind_method(D_METHOD("set_bus", "name"), &NeumetikMusic::set_bus);
    ClassDB::bind_method(D_METHOD("get_bus"), &NeumetikMusic::get_bus);

    ADD_PROPERTY(PropertyInfo(Variant::BOOL, "looping"), "set_looping", "is_looping");
    ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "volume", PROPERTY_HINT_RANGE, "0,4,0.01"), "set_volume", "get_volume");
    ADD_PROPERTY(PropertyInfo(Variant::STRING_NAME, "bus"), "set_bus", "get_bus");
}

NeumetikMusic::NeumetikMusic() {
    player = nm_create(mix_rate);
}

NeumetikMusic::~NeumetikMusic() {
    nm_destroy(player);
}

void NeumetikMusic::_ready() {
    if (Engine::get_singleton()->is_editor_hint()) {
        return;
    }
    Ref<AudioStreamGenerator> generator;
    generator.instantiate();
    generator->set_mix_rate(mix_rate);
    generator->set_buffer_length(0.1f);
    output = memnew(AudioStreamPlayer);
    output->set_stream(generator);
    output->set_bus(bus);
    add_child(output);
    output->play();
    playback = output->get_stream_playback();
}

void NeumetikMusic::_process(double) {
    if (playback.is_null()) {
        return;
    }
    int available = playback->get_frames_available();
    while (available > 0) {
        int n = available < 1024 ? available : 1024;
        nm_render_f32(player, buffer, (size_t)n);
        frames.resize(n);
        Vector2 *w = frames.ptrw();
        for (int i = 0; i < n; i++) {
            w[i] = Vector2(buffer[2 * i], buffer[2 * i + 1]);
        }
        playback->push_buffer(frames);
        available -= n;
    }
}

String NeumetikMusic::load_file(const String &path) {
    PackedByteArray bytes = FileAccess::get_file_as_bytes(path);
    if (bytes.is_empty()) {
        return "Couldn't read " + path + ".";
    }
    return load_bytes(bytes);
}

String NeumetikMusic::load_bytes(const PackedByteArray &bytes) {
    int32_t code = nm_load(player, bytes.ptr(), (size_t)bytes.size());
    return code == NM_OK ? String() : String("Couldn't play that MIDI file: ") + nm_error_string(code) + ".";
}

void NeumetikMusic::play() { nm_play(player); }
void NeumetikMusic::pause() { nm_pause(player); }
void NeumetikMusic::stop() { nm_stop(player); }
bool NeumetikMusic::is_playing() const { return nm_state(player) == NM_PLAYING; }
bool NeumetikMusic::is_finished() const { return nm_is_finished(player) != 0; }
void NeumetikMusic::seek(double seconds) { nm_seek(player, seconds); }
double NeumetikMusic::get_position() const { return nm_position(player); }
double NeumetikMusic::get_duration() const { return nm_duration(player); }
void NeumetikMusic::set_looping(bool looping) { nm_set_looping(player, looping ? 1 : 0); }
bool NeumetikMusic::is_looping() const { return nm_is_looping(player) != 0; }
void NeumetikMusic::set_loop_points(double start, double end) { nm_set_loop_points(player, start, end); }
void NeumetikMusic::set_speed(double speed) { nm_set_speed(player, speed); }
void NeumetikMusic::set_volume(float volume) { nm_set_volume(player, volume); }
float NeumetikMusic::get_volume() const { return nm_volume(player); }
void NeumetikMusic::fade_to(float volume, float seconds) { nm_fade_to(player, volume, seconds); }
void NeumetikMusic::set_channel_muted(int channel, bool muted) { nm_set_channel_muted(player, channel, muted ? 1 : 0); }
void NeumetikMusic::note_on(int channel, int key, int velocity) { nm_note_on(player, channel, key, velocity); }
void NeumetikMusic::note_off(int channel, int key) { nm_note_off(player, channel, key); }
void NeumetikMusic::control_change(int channel, int controller, int value) { nm_control_change(player, channel, controller, value); }
void NeumetikMusic::set_instrument(int channel, int bank, int program) { nm_set_instrument(player, channel, bank, program); }
void NeumetikMusic::pitch_bend(int channel, int value) { nm_pitch_bend(player, channel, value); }
void NeumetikMusic::all_notes_off() { nm_all_notes_off(player); }
int NeumetikMusic::get_active_voices() const { return nm_active_voices(player); }

String NeumetikMusic::get_instrument_name(int channel) const {
    char name[64];
    nm_instrument_name(player, channel, name, sizeof name);
    return String(name);
}

void NeumetikMusic::set_bus(const StringName &name) {
    bus = name;
    if (output) {
        output->set_bus(name);
    }
}

StringName NeumetikMusic::get_bus() const { return bus; }
