// VERBATIM copy of .oracle/internal/c/parts/audio/out/audio.cpp lines 111-165 (synth types).
// Test-only: compiled by tests/diff_audio.cpp. Do not edit.
enum struct SoundType
{
    None,   // No sound or internal sound whose buffer is managed by the QBPE audio engine
    Static, // Static sounds that are completely managed by miniaudio
    Raw     // Raw sound stream that is managed by the QBPE audio engine
};

struct SampleFrameBlockQueue;

enum struct SynthWaveformKind
{
    Triangle,
    Sine,
    Square,
    Saw,
    WhiteNoise,
    PinkNoise,
    BrownNoise,
    LfsrNoise,
    Custom
};

struct SynthEnvelopeSettings
{
    double attack;
    double decay;
    double sustain;
    double release;

    SynthEnvelopeSettings()
    {
        attack = 0.01;
        decay = 0.05;
        sustain = 0.80;
        release = 0.06;
    }
};

struct SynthVoice
{
    ma_sound maSound;
    SampleFrameBlockQueue *rawQueue;
    SynthEnvelopeSettings envelope;
    SynthWaveformKind waveform;
    std::vector<float> customWave;
    uint32_t lfsr;

    SynthVoice()
    {
        ZERO_VARIABLE(maSound);
        rawQueue = nullptr;
        waveform = SynthWaveformKind::Triangle;
        lfsr = 0xACE1u;
    }
};
