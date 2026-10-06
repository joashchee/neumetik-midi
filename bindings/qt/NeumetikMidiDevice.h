// Neumetik MIDI for Qt 6: a QIODevice that QAudioSink pulls MIDI music
// from. Header only. Copyright 2026 ansiapps. MIT licensed.
//
//   QAudioFormat format;
//   format.setSampleRate(48000);
//   format.setChannelCount(2);
//   format.setSampleFormat(QAudioFormat::Int16);
//
//   auto *music = new NeumetikMidiDevice(48000, this);
//   music->load(midiBytes);            // a QByteArray
//   nm_set_looping(music->player(), 1);
//   nm_play(music->player());
//   auto *sink = new QAudioSink(format, this);
//   sink->start(music);
//
// QAudioSink reads from the thread it was made on, so control the player
// from that thread too (in most apps, the GUI thread).
#pragma once

#include <QIODevice>
#include <QString>

#include "neumetik_midi.h"

class NeumetikMidiDevice : public QIODevice {
public:
    explicit NeumetikMidiDevice(int sampleRate = 48000, QObject *parent = nullptr)
        : QIODevice(parent), m_player(nm_create(static_cast<uint32_t>(sampleRate))) {
        open(QIODevice::ReadOnly);
    }

    ~NeumetikMidiDevice() override { nm_destroy(m_player); }

    // Loads a MIDI file; returns an empty string, or why it couldn't.
    QString load(const QByteArray &midi) {
        const int32_t code = nm_load(m_player, reinterpret_cast<const uint8_t *>(midi.constData()), static_cast<size_t>(midi.size()));
        return code == NM_OK ? QString() : QStringLiteral("Couldn't play that MIDI file: %1.").arg(QString::fromLatin1(nm_error_string(code)));
    }

    // The player, for every other nm_ call: nm_play, nm_fade_to, nm_note_on...
    nm_player *player() const { return m_player; }

    bool isSequential() const override { return true; }
    qint64 bytesAvailable() const override { return 4096 + QIODevice::bytesAvailable(); }

protected:
    // 16-bit stereo: four bytes a frame.
    qint64 readData(char *data, qint64 maxSize) override {
        const qint64 frames = maxSize / 4;
        nm_render_s16(m_player, reinterpret_cast<int16_t *>(data), static_cast<size_t>(frames));
        return frames * 4;
    }

    qint64 writeData(const char *, qint64) override { return -1; }

private:
    nm_player *m_player;
};
