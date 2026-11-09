// VERBATIM copy of .oracle/internal/c/parts/audio/out/audio.cpp lines 728-1941 (synth, SOUND, PLAY, _VOICE, _ADSR, _WAVE).
// Test-only: compiled by tests/diff_audio.cpp. Do not edit.
static double ClampUnit(double value)
{
    if (value < 0.0)
        return 0.0;
    if (value > 1.0)
        return 1.0;
    return value;
}

static std::string TrimCopy(const std::string &value)
{
    size_t first = 0;
    size_t last = value.size();

    while (first < value.size() && std::isspace(static_cast<unsigned char>(value[first])))
        ++first;
    while (last > first && std::isspace(static_cast<unsigned char>(value[last - 1])))
        --last;

    return value.substr(first, last - first);
}

static std::string UpperCopy(const std::string &value)
{
    std::string result = value;
    for (size_t i = 0; i < result.size(); ++i)
        result[i] = static_cast<char>(std::toupper(static_cast<unsigned char>(result[i])));
    return result;
}

static int32_t ResolveConfigVoiceIndex(int32_t requestedVoice, int32_t voicePassedMask, bool &applyAll)
{
    applyAll = false;

    if (voicePassedMask)
    {
        if (requestedVoice < 1 || requestedVoice > QBNEX_SYNTH_VOICE_COUNT)
            return -1;
        return requestedVoice - 1;
    }

    if (audioEngine.synthVoicePinned)
        return audioEngine.currentSynthVoice;

    applyAll = true;
    return 0;
}

static int32_t ResolvePlaybackVoiceIndex()
{
    if (audioEngine.synthVoicePinned)
        return audioEngine.currentSynthVoice;

    int32_t voiceIndex = audioEngine.nextSynthVoice;
    audioEngine.nextSynthVoice = (audioEngine.nextSynthVoice + 1) % QBNEX_SYNTH_VOICE_COUNT;
    audioEngine.currentSynthVoice = voiceIndex;
    return voiceIndex;
}

static float EnvelopeAmplitude(const SynthEnvelopeSettings &envelope, double totalLength, double timePosition)
{
    double attack = std::max(0.0, envelope.attack);
    double decay = std::max(0.0, envelope.decay);
    double sustain = ClampUnit(envelope.sustain);
    double release = std::max(0.0, envelope.release);
    double sustainDuration = 0.0;
    double shapedTotal = attack + decay + release;

    if (totalLength <= 0.0)
        return 0.0f;

    if (shapedTotal > totalLength && shapedTotal > 0.0)
    {
        double scale = totalLength / shapedTotal;
        attack *= scale;
        decay *= scale;
        release *= scale;
    }
    else
    {
        sustainDuration = totalLength - shapedTotal;
    }

    if (attack > 0.0 && timePosition < attack)
        return static_cast<float>(timePosition / attack);

    timePosition -= attack;
    if (decay > 0.0 && timePosition < decay)
        return static_cast<float>(1.0 - (1.0 - sustain) * (timePosition / decay));

    timePosition -= decay;
    if (timePosition < sustainDuration)
        return static_cast<float>(sustain);

    timePosition -= sustainDuration;
    if (release > 0.0 && timePosition < release)
        return static_cast<float>(sustain * (1.0 - (timePosition / release)));

    return 0.0f;
}

static float SampleCustomWave(const std::vector<float> &customWave, double phase)
{
    if (customWave.empty())
        return 0.0f;
    if (customWave.size() == 1)
        return customWave[0];

    double wrapped = phase - std::floor(phase);
    double scaledIndex = wrapped * customWave.size();
    size_t index0 = static_cast<size_t>(scaledIndex) % customWave.size();
    size_t index1 = (index0 + 1) % customWave.size();
    double fraction = scaledIndex - std::floor(scaledIndex);

    return static_cast<float>(customWave[index0] + (customWave[index1] - customWave[index0]) * fraction);
}

static float NextLfsrNoise(uint32_t &lfsr)
{
    uint32_t bit = ((lfsr >> 0) ^ (lfsr >> 2) ^ (lfsr >> 3) ^ (lfsr >> 5)) & 1u;
    lfsr = (lfsr >> 1) | (bit << 15);
    return (lfsr & 1u) ? 1.0f : -1.0f;
}

static bool ParseCustomWaveSpec(const std::string &spec, std::vector<float> &outSamples)
{
    std::string samplesText = spec;
    size_t colon = samplesText.find(':');
    if (colon != std::string::npos)
        samplesText = samplesText.substr(colon + 1);

    outSamples.clear();

    size_t start = 0;
    while (start < samplesText.size())
    {
        size_t end = start;
        while (end < samplesText.size() && samplesText[end] != ',' && samplesText[end] != ';')
            ++end;

        std::string token = TrimCopy(samplesText.substr(start, end - start));
        if (!token.empty())
        {
            char *parseEnd = NULL;
            double parsedValue = std::strtod(token.c_str(), &parseEnd);
            if (!parseEnd || *parseEnd != 0)
                return false;

            if (parsedValue > 1.0 || parsedValue < -1.0)
                parsedValue /= 32768.0;

            if (parsedValue > 1.0)
                parsedValue = 1.0;
            if (parsedValue < -1.0)
                parsedValue = -1.0;

            outSamples.push_back(static_cast<float>(parsedValue));
        }

        start = end + 1;
    }

    return !outSamples.empty();
}

static bool ApplyWaveSpecToVoice(SynthVoice &voice, const std::string &spec)
{
    std::string normalized = UpperCopy(TrimCopy(spec));

    if (normalized == "TRIANGLE")
    {
        voice.waveform = SynthWaveformKind::Triangle;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "SINE")
    {
        voice.waveform = SynthWaveformKind::Sine;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "SQUARE")
    {
        voice.waveform = SynthWaveformKind::Square;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "SAW" || normalized == "SAWTOOTH")
    {
        voice.waveform = SynthWaveformKind::Saw;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "NOISE" || normalized == "WHITE" || normalized == "WHITE NOISE")
    {
        voice.waveform = SynthWaveformKind::WhiteNoise;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "PINK" || normalized == "PINK NOISE")
    {
        voice.waveform = SynthWaveformKind::PinkNoise;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "BROWN" || normalized == "BROWNIAN" || normalized == "BROWN NOISE" || normalized == "BROWNIAN NOISE")
    {
        voice.waveform = SynthWaveformKind::BrownNoise;
        std::vector<float>().swap(voice.customWave);
        return true;
    }
    if (normalized == "LFSR" || normalized == "LFSR NOISE")
    {
        voice.waveform = SynthWaveformKind::LfsrNoise;
        std::vector<float>().swap(voice.customWave);
        if (!voice.lfsr)
            voice.lfsr = 0xACE1u;
        return true;
    }

    if (normalized.rfind("CUSTOM", 0) == 0 || normalized.find(',') != std::string::npos || normalized.find(';') != std::string::npos)
    {
        std::vector<float> parsedWave;
        if (!ParseCustomWaveSpec(spec, parsedWave))
            return false;
        voice.waveform = SynthWaveformKind::Custom;
        voice.customWave.swap(parsedWave);
        return true;
    }

    return false;
}

static float SampleSynthWave(SynthVoice &voice, double phase, float &brownState, float &pink0, float &pink1, float &pink2)
{
    switch (voice.waveform)
    {
    case SynthWaveformKind::Sine:
        return static_cast<float>(std::sin(phase * QBNEX_PI * 2.0));
    case SynthWaveformKind::Square:
        return phase < 0.5 ? 1.0f : -1.0f;
    case SynthWaveformKind::Saw:
        return static_cast<float>(phase * 2.0 - 1.0);
    case SynthWaveformKind::WhiteNoise:
        return static_cast<float>((static_cast<double>(std::rand()) / static_cast<double>(RAND_MAX)) * 2.0 - 1.0);
    case SynthWaveformKind::PinkNoise:
    {
        float white = static_cast<float>((static_cast<double>(std::rand()) / static_cast<double>(RAND_MAX)) * 2.0 - 1.0);
        pink0 = 0.99765f * pink0 + white * 0.0990460f;
        pink1 = 0.96300f * pink1 + white * 0.2965164f;
        pink2 = 0.57000f * pink2 + white * 1.0526913f;
        float pink = pink0 + pink1 + pink2 + white * 0.1848f;
        if (pink > 1.0f)
            pink = 1.0f;
        if (pink < -1.0f)
            pink = -1.0f;
        return pink;
    }
    case SynthWaveformKind::BrownNoise:
    {
        float white = static_cast<float>((static_cast<double>(std::rand()) / static_cast<double>(RAND_MAX)) * 2.0 - 1.0);
        brownState += white * 0.08f;
        if (brownState > 1.0f)
            brownState = 1.0f;
        if (brownState < -1.0f)
            brownState = -1.0f;
        return brownState;
    }
    case SynthWaveformKind::LfsrNoise:
        return NextLfsrNoise(voice.lfsr);
    case SynthWaveformKind::Custom:
        return SampleCustomWave(voice.customWave, phase);
    case SynthWaveformKind::Triangle:
    default:
        return static_cast<float>(1.0 - 4.0 * std::fabs(phase - 0.5));
    }
}

/// <summary>
/// This creates 16-bit signed stereo data. The sound buffer is allocated and then returned.
/// Do we really need stereo for Play(), Sound() and Beep()?
/// </summary>
/// <param name="frequency">The sound frequency</param>
/// <param name="length">The duration of the sound in seconds</param>
/// <param name="volume">The volume of the sound (0.0 - 1.0)</param>
/// <param name="soundwave_bytes">A pointer to an integer that will receive the buffer size in bytes. This cannot be NULL</param>
/// <returns></returns>
static ma_uint8 *GenerateWaveform(SynthVoice &voice, double frequency, double length, double volume, ma_int32 *soundwave_bytes)
{
    ma_uint8 *data;
    ma_int32 i;
    ma_int16 x;
    ma_int16 *sp;
    double samples;
    ma_int32 samplesi;
    double volume_multiplier;
    float brownState = 0.0f;
    double phase = 0.0;
    double phaseIncrement = 0.0;
    float pink0 = 0.0f;
    float pink1 = 0.0f;
    float pink2 = 0.0f;

    // calculate total number of samples required
    samples = length * audioEngine.sampleRate;
    samplesi = samples;
    if (!samplesi)
        samplesi = 1;

    *soundwave_bytes = samplesi * SAMPLE_FRAME_SIZE(ma_int16, 2);

    // Frequency equal to or above 20000 will produce silence
    // This is per QuickBASIC 4.5 behavior
    if (frequency < 20000)
    {
        data = (ma_uint8 *)malloc(*soundwave_bytes);
    }
    else
    {
        data = (ma_uint8 *)calloc(*soundwave_bytes, sizeof(ma_uint8));
        return data;
    }

    if (!data)
        return nullptr;

    sp = (ma_int16 *)data;

    volume_multiplier = volume * 32767.0;
    if (audioEngine.sampleRate > 0)
        phaseIncrement = frequency / audioEngine.sampleRate;

    for (i = 0; i < samplesi; i++)
    {
        double timePosition = static_cast<double>(i) / audioEngine.sampleRate;
        float envelope = EnvelopeAmplitude(voice.envelope, length, timePosition);
        float waveform = 0.0f;

        if (frequency > 0.0 || voice.waveform == SynthWaveformKind::WhiteNoise || voice.waveform == SynthWaveformKind::PinkNoise ||
            voice.waveform == SynthWaveformKind::BrownNoise || voice.waveform == SynthWaveformKind::LfsrNoise)
        {
            waveform = SampleSynthWave(voice, phase - std::floor(phase), brownState, pink0, pink1, pink2);
        }

        x = static_cast<ma_int16>(waveform * envelope * volume_multiplier);
        *sp++ = x;
        *sp++ = x;
        phase += phaseIncrement;
    }

    return data;
}

/// <summary>
/// Returns the of a sound buffer in bytes.
/// </summary>
/// <param name="length">Length in seconds</param>
/// <returns>Length in bytes</returns>
static ma_int32 WaveformBufferSize(double length)
{
    ma_int32 samples;

    samples = (ma_int32)(length * audioEngine.sampleRate);
    if (!samples)
        samples = 1;

    return samples * SAMPLE_FRAME_SIZE(ma_int16, 2);
}

/// <summary>
/// This sends a buffer to a raw queue for playback.
/// Buffer required in 16-bit stereo at native frequency.
/// The buffer is freed.
/// </summary>
/// <param name="data">Sound buffer</param>
/// <param name="bytes">Length of buffer in bytes</param>
/// <param name="block">So we have to wait until playback completes</param>
/// <param name="sndRawQueue">A pointer to a raw queue object</param>
static void SendWaveformToQueue(SampleFrameBlockQueue *queue, ma_uint8 *data, ma_int32 bytes, bool block)
{
    ma_int32 i;
    ma_int64 time_ms;

    if (!data || !queue)
        return;

    // Move data into sndraw handle
    for (i = 0; i < bytes; i += SAMPLE_FRAME_SIZE(ma_int16, 2))
    {
        queue->PushSampleFrame((float)((ma_int16 *)(data + i))[0] / 32768.0f, (float)((ma_int16 *)(data + i))[1] / 32768.0f);
    }

    free(data); // free the sound data

    // This will push any unfinished block for playback
    if (queue->last)
        queue->last->force = true;

    // This will wait for the block to finish (if specified)
    // We'll be good citizens and give-up our time-slices while waiting
    if (block)
    {
        time_ms = (ma_int64)(queue->GetTimeRemaining() * 950.0 - 250.0);
        if (time_ms > 0)
            Sleep(time_ms);
    }
}

/// <summary>
/// This generates a sound at the specified frequency for the specified amount of time.
/// </summary>
/// <param name="frequency">Sound frequency</param>
/// <param name="lengthInClockTicks">Duration in clock ticks. There are 18.2 clock ticks per second</param>
void sub_sound(double frequency, double lengthInClockTicks)
{
    ma_uint8 *data;
    ma_int32 soundwave_bytes;
    int32_t voiceIndex;

    if (new_error || !audioEngine.isInitialized)
        return;

    if ((frequency < 37.0) && (frequency != 0))
        goto error;
    if (frequency > 32767.0)
        goto error;
    if (lengthInClockTicks < 0.0)
        goto error;
    if (lengthInClockTicks > 65535.0)
        goto error;
    if (lengthInClockTicks == 0.0)
        return;

    voiceIndex = ResolvePlaybackVoiceIndex();
    data = GenerateWaveform(audioEngine.synthVoices[voiceIndex], frequency, lengthInClockTicks / 18.2, 1, &soundwave_bytes);
    SendWaveformToQueue(audioEngine.synthVoices[voiceIndex].rawQueue, data, soundwave_bytes, false);

    return;

error:
    error(5);
}

/// <summary>
/// This generates a default 'beep' sound.
/// </summary>
void sub_beep() { sub_sound(900, 5); }

/// <summary>
/// This was designed to returned the number of notes in the background music queue.
/// However, here we'll just return the number of sample frame remaining to play when Play(), Sound() or Beep() are used.
/// This allows programs like the following to compile and work.
///
///     Music$ = "MBT180o2P2P8L8GGGL2E-P24P8L8FFFL2D"
///     PLAY Music$
///     WHILE PLAY(0) > 5: WEND
///     PRINT "Just about done!"
/// </summary>
/// <param name="ignore">Well, it's ignored</param>
/// <returns>Returns the number of sample frames left to play for Play(), Sound() & Beep()</returns>
int32_t func_play(int32_t ignore)
{
    if (audioEngine.isInitialized)
    {
        int32_t maxFramesRemaining = 0;

        for (int32_t voiceIndex = 0; voiceIndex < QBNEX_SYNTH_VOICE_COUNT; ++voiceIndex)
        {
            if (audioEngine.synthVoices[voiceIndex].rawQueue)
            {
                if (audioEngine.synthVoices[voiceIndex].rawQueue->last)
                    audioEngine.synthVoices[voiceIndex].rawQueue->last->force = true;
                maxFramesRemaining = std::max(maxFramesRemaining, static_cast<int32_t>(audioEngine.synthVoices[voiceIndex].rawQueue->GetSampleFramesRemaining()));
            }
        }

        return maxFramesRemaining;
    }

    return 0;
}

/// <summary>
/// Processes and plays the MML specified in the string.
/// Mmmmm. Spaghetti goodness.
/// Formats:
/// A[#|+|-][0-64]
/// 0-64 is like temp. Lnumber, 0 is whatever the current default is
/// </summary>
/// <param name="str">The string to play</param>
void sub_play(qbs *str)
{
    static ma_int32 soundwave_bytes;
    static ma_uint8 *b, *wave, *wave2;
    static double d;
    static ma_int32 i, bytes_left, a, x, x2, x3, x4, wave_bytes, wave_base;
    static ma_int32 o = 4;
    static double t = 120; // quarter notes per minute (120/60=2 per second)
    static double l = 4;
    static double pause = 1.0 / 8.0; // ML 0.0, MN 1.0/8.0, MS 1.0/4.0
    static double length, length2;   // derived from l and t
    static double frequency;
    static double mb = 0;
    static double v = 50;
    static ma_int32 n;         // the semitone-intervaled note to be played
    static ma_int32 n_changed; //+,#,- applied?
    static ma_int64 number;
    static ma_int32 number_entered;
    static ma_int32 followup; // 1=play note
    static ma_int32 playit;
    static ma_int32 fullstops = 0;
    int32_t voiceIndex;
    SampleFrameBlockQueue *targetQueue;

    if (new_error || !audioEngine.isInitialized)
        return;

    voiceIndex = ResolvePlaybackVoiceIndex();
    targetQueue = audioEngine.synthVoices[voiceIndex].rawQueue;
    if (!targetQueue)
        return;

    b = str->chr;
    bytes_left = str->len;
    wave = NULL;
    wave_bytes = 0;
    n_changed = 0;
    n = 0;
    number_entered = 0;
    number = 0;
    followup = 0;
    length = 1.0 / (t / 60.0) * (4.0 / l);
    playit = 0;
    wave_base = 0; // point at which new sounds will be inserted

next_byte:
    if ((bytes_left--) || followup)
    {

        if (bytes_left < 0)
        {
            i = 32;
            goto follow_up;
        }

        i = *b++;
        if (i == 32)
            goto next_byte;
        if (i >= 97 && i <= 122)
            a = i - 32;
        else
            a = i;

        if (i == 61)
        { //= (+VARPTR$)
            if (fullstops)
            {
                error(5);
                return;
            }
            if (number_entered)
            {
                error(5);
                return;
            }
            number_entered = 2;
            // VARPTR$ reference
            /*
               'BYTE=1
               'INTEGER=2
               'STRING=3 SUB-STRINGS must use "X"+VARPTR$(string$)
               'SINGLE=4
               'INT64=5
               'FLOAT=6
               'DOUBLE=8
               'LONG=20
               'BIT=64+n
             */
            if (bytes_left < 3)
            {
                error(5);
                return;
            }
            i = *b++;
            bytes_left--; // read type byte
            x = *(ma_uint16 *)b;
            b += 2;
            bytes_left -= 2; // read offset within DBLOCK
            // note: allowable _BIT type variables in VARPTR$ are all at a byte offset and are all
            //      padded until the next byte
            d = 0;
            switch (i)
            {
            case 1:
                d = *(char *)(dblock + x);
                break;
            case (1 + 128):
                d = *(ma_uint8 *)(dblock + x);
                break;
            case 2:
                d = *(ma_int16 *)(dblock + x);
                break;
            case (2 + 128):
                d = *(ma_uint16 *)(dblock + x);
                break;
            case 4:
                d = *(float *)(dblock + x);
                break;
            case 5:
                d = *(ma_int64 *)(dblock + x);
                break;
            case (5 + 128):
                d = *(ma_int64 *)(dblock + x); // unsigned conversion is unsupported!
                break;
            case 6:
                d = *(long double *)(dblock + x);
                break;
            case 8:
                d = *(double *)(dblock + x);
                break;
            case 20:
                d = *(ma_int32 *)(dblock + x);
                break;
            case (20 + 128):
                d = *(ma_uint32 *)(dblock + x);
                break;
            default:
                // bit type?
                if ((i & 64) == 0)
                {
                    error(5);
                    return;
                }
                x2 = i & 63;
                if (x2 > 56)
                {
                    error(5);
                    return;
                } // valid number of bits?
                // create a mask
                static ma_int64 i64num, mask, i64x;
                mask = (((ma_int64)1) << x2) - 1;
                i64num = (*(ma_int64 *)(dblock + x)) & mask;
                // signed?
                if (i & 128)
                {
                    mask = ((ma_int64)1) << (x2 - 1);
                    if (i64num & mask)
                    { // top bit on?
                        mask = -1;
                        mask <<= x2;
                        i64num += mask;
                    }
                } // signed
                d = i64num;
            }
            if (d > 2147483647.0 || d < -2147483648.0)
            {
                error(5);
                return;
            } // out of range value!
            number = round(d);
            goto next_byte;
        }

        // read in a number
        if ((i >= 48) && (i <= 57))
        {
            if (fullstops || (number_entered == 2))
            {
                error(5);
                return;
            }
            if (!number_entered)
            {
                number = 0;
                number_entered = 1;
            }
            number = number * 10 + i - 48;
            goto next_byte;
        }

        // read fullstops
        if (i == 46)
        {
            if (followup != 7 && followup != 1 && followup != 4)
            {
                error(5);
                return;
            }
            fullstops++;
            goto next_byte;
        }

    follow_up:

        if (followup == 8)
        { // V...
            if (!number_entered)
            {
                error(5);
                return;
            }
            number_entered = 0;
            if (number > 100)
            {
                error(5);
                return;
            }
            v = number;
            followup = 0;
            if (bytes_left < 0)
                goto done;
        } // 8

        if (followup == 7)
        { // P...
            if (number_entered)
            {
                number_entered = 0;
                if (number < 1 || number > 64)
                {
                    error(5);
                    return;
                }
                length2 = 1.0 / (t / 60.0) * (4.0 / ((double)number));
            }
            else
            {
                length2 = length;
            }
            d = length2;
            for (x = 1; x <= fullstops; x++)
            {
                d /= 2.0;
                length2 = length2 + d;
            }
            fullstops = 0;

            soundwave_bytes = WaveformBufferSize(length2);
            if (!wave)
            {
                // create buffer
                wave = (ma_uint8 *)calloc(soundwave_bytes, 1);
                wave_bytes = soundwave_bytes;
                wave_base = 0;
            }
            else
            {
                // increase buffer?
                if ((wave_base + soundwave_bytes) > wave_bytes)
                {
                    wave = (ma_uint8 *)realloc(wave, wave_base + soundwave_bytes);
                    memset(wave + wave_base, 0, wave_base + soundwave_bytes - wave_bytes);
                    wave_bytes = wave_base + soundwave_bytes;
                }
            }
            if (i != 44)
            {
                wave_base += soundwave_bytes;
            }

            playit = 1;
            followup = 0;
            if (i == 44)
                goto next_byte;
            if (bytes_left < 0)
                goto done;
        } // 7

        if (followup == 6)
        { // T...
            if (!number_entered)
            {
                error(5);
                return;
            }
            number_entered = 0;
            if (number < 32 || number > 255)
            {
                number = 120;
            }
            t = number;
            length = 1.0 / (t / 60.0) * (4.0 / l);
            followup = 0;
            if (bytes_left < 0)
                goto done;
        } // 6

        if (followup == 5)
        { // M...
            if (number_entered)
            {
                error(5);
                return;
            }
            switch (a)
            {
            case 76: // L
                pause = 0;
                break;
            case 78: // N
                pause = 1.0 / 8.0;
                break;
            case 83: // S
                pause = 1.0 / 4.0;
                break;

            case 66: // MB
                if (!mb)
                {
                    mb = 1;
                    if (playit)
                    {
                        playit = 0;
                        SendWaveformToQueue(targetQueue, wave, wave_bytes, false);
                    }
                    wave = NULL;
                }
                break;
            case 70: // MF
                if (mb)
                {
                    mb = 0;
                    // preceding MB content incorporated into MF block
                }
                break;
            default:
                error(5);
                return;
            }
            followup = 0;
            goto next_byte;
        } // 5

        if (followup == 4)
        { // N...
            if (!number_entered)
            {
                error(5);
                return;
            }
            number_entered = 0;
            if (number > 84)
            {
                error(5);
                return;
            }
            n = -33 + number;
            goto followup1;
            followup = 0;
            if (bytes_left < 0)
                goto done;
        } // 4

        if (followup == 3)
        { // O...
            if (!number_entered)
            {
                error(5);
                return;
            }
            number_entered = 0;
            if (number > 6)
            {
                error(5);
                return;
            }
            o = number;
            followup = 0;
            if (bytes_left < 0)
                goto done;
        } // 3

        if (followup == 2)
        { // L...
            if (!number_entered)
            {
                error(5);
                return;
            }
            number_entered = 0;
            if (number < 1 || number > 64)
            {
                error(5);
                return;
            }
            l = number;
            length = 1.0 / (t / 60.0) * (4.0 / l);
            followup = 0;
            if (bytes_left < 0)
                goto done;
        } // 2

        if (followup == 1)
        { // A-G...
            if (i == 45)
            { //-
                if (n_changed || number_entered)
                {
                    error(5);
                    return;
                }
                n_changed = 1;
                n--;
                goto next_byte;
            }
            if (i == 43 || i == 35)
            { //+,#
                if (n_changed || number_entered)
                {
                    error(5);
                    return;
                }
                n_changed = 1;
                n++;
                goto next_byte;
            }
        followup1:
            if (number_entered)
            {
                number_entered = 0;
                if (number < 0 || number > 64)
                {
                    error(5);
                    return;
                }
                if (!number)
                    length2 = length;
                else
                    length2 = 1.0 / (t / 60.0) * (4.0 / ((double)number));
            }
            else
            {
                length2 = length;
            } // number_entered
            d = length2;
            for (x = 1; x <= fullstops; x++)
            {
                d /= 2.0;
                length2 = length2 + d;
            }
            fullstops = 0;
            // frequency=(2^(note/12))*440
            frequency = pow(2.0, ((double)n) / 12.0) * 440.0;

            // create wave
            wave2 = GenerateWaveform(audioEngine.synthVoices[voiceIndex], frequency, length2 * (1.0 - pause), v / 100.0, &soundwave_bytes);
            if (pause > 0)
            {
                wave2 = (ma_uint8 *)realloc(wave2, soundwave_bytes + WaveformBufferSize(length2 * pause));
                memset(wave2 + soundwave_bytes, 0, WaveformBufferSize(length2 * pause));
                soundwave_bytes += WaveformBufferSize(length2 * pause);
            }

            if (!wave)
            {
                // adopt buffer
                wave = wave2;
                wave_bytes = soundwave_bytes;
                wave_base = 0;
            }
            else
            {
                // mix required?
                if (wave_base == wave_bytes)
                    x = 0;
                else
                    x = 1;
                // increase buffer?
                if ((wave_base + soundwave_bytes) > wave_bytes)
                {
                    wave = (ma_uint8 *)realloc(wave, wave_base + soundwave_bytes);
                    memset(wave + wave_base, 0, wave_base + soundwave_bytes - wave_bytes);
                    wave_bytes = wave_base + soundwave_bytes;
                }
                // mix or copy
                if (x)
                {
                    // mix
                    static ma_int16 *sp, *sp2;
                    sp = (ma_int16 *)(wave + wave_base);
                    sp2 = (ma_int16 *)wave2;
                    x2 = soundwave_bytes / 2;
                    for (x = 0; x < x2; x++)
                    {
                        x3 = *sp2++;
                        x4 = *sp;
                        x4 += x3;
                        if (x4 > 32767)
                            x4 = 32767;
                        if (x4 < -32767)
                            x4 = -32767;
                        *sp++ = x4;
                    } // x
                }
                else
                {
                    // copy
                    memcpy(wave + wave_base, wave2, soundwave_bytes);
                } // x
                free(wave2);
            }
            if (i != 44)
            {
                wave_base += soundwave_bytes;
            }

            playit = 1;
            n_changed = 0;
            followup = 0;
            if (i == 44)
                goto next_byte;
            if (bytes_left < 0)
                goto done;
        } // 1

        if (a >= 65 && a <= 71)
        {
            // modify a to represent a semitonal note (n) interval
            switch (a)
            {
                //[c][ ][d][ ][e][f][ ][g][ ][a][ ][b]
                // 0  1  2  3  4  5  6  7  8  9  0  1
            case 65:
                n = 9;
                break;
            case 66:
                n = 11;
                break;
            case 67:
                n = 0;
                break;
            case 68:
                n = 2;
                break;
            case 69:
                n = 4;
                break;
            case 70:
                n = 5;
                break;
            case 71:
                n = 7;
                break;
            }
            n = n + (o - 2) * 12 - 9;
            followup = 1;
            goto next_byte;
        } // a

        if (a == 76)
        { // L
            followup = 2;
            goto next_byte;
        }

        if (a == 77)
        { // M
            followup = 5;
            goto next_byte;
        }

        if (a == 78)
        { // N
            followup = 4;
            goto next_byte;
        }

        if (a == 79)
        { // O
            followup = 3;
            goto next_byte;
        }

        if (a == 84)
        { // T
            followup = 6;
            goto next_byte;
        }

        if (a == 60)
        { //<
            o--;
            if (o < 0)
                o = 0;
            goto next_byte;
        }

        if (a == 62)
        { //>
            o++;
            if (o > 6)
                o = 6;
            goto next_byte;
        }

        if (a == 80)
        { // P
            followup = 7;
            goto next_byte;
        }

        if (a == 86)
        { // V
            followup = 8;
            goto next_byte;
        }

        error(5);
        return;
    } // bytes_left
done:
    if (number_entered || followup)
    {
        error(5);
        return;
    } // unhandled data

    if (playit)
    {
        SendWaveformToQueue(targetQueue, wave, wave_bytes, false);
    } // playit
}

void sub__voice(int32_t voice)
{
    if (voice == 0)
    {
        audioEngine.synthVoicePinned = false;
        return;
    }

    if (voice < 1 || voice > QBNEX_SYNTH_VOICE_COUNT)
    {
        error(5);
        return;
    }

    audioEngine.synthVoicePinned = true;
    audioEngine.currentSynthVoice = voice - 1;
}

void sub__adsr(double attack, double decay, double sustain, double release, int32_t voice, int32_t passed)
{
    bool applyAll;
    int32_t targetVoice = ResolveConfigVoiceIndex(voice, passed & 16, applyAll);
    if (targetVoice < 0)
    {
        error(5);
        return;
    }

    if (sustain > 1.0 && sustain <= 100.0)
        sustain /= 100.0;
    if (attack < 0.0 || decay < 0.0 || release < 0.0 || sustain < 0.0 || sustain > 1.0)
    {
        error(5);
        return;
    }

    if (applyAll)
    {
        for (int32_t i = 0; i < QBNEX_SYNTH_VOICE_COUNT; ++i)
        {
            audioEngine.synthVoices[i].envelope.attack = attack;
            audioEngine.synthVoices[i].envelope.decay = decay;
            audioEngine.synthVoices[i].envelope.sustain = sustain;
            audioEngine.synthVoices[i].envelope.release = release;
        }
    }
    else
    {
        audioEngine.synthVoices[targetVoice].envelope.attack = attack;
        audioEngine.synthVoices[targetVoice].envelope.decay = decay;
        audioEngine.synthVoices[targetVoice].envelope.sustain = sustain;
        audioEngine.synthVoices[targetVoice].envelope.release = release;
    }
}

void sub__wave(qbs *spec, int32_t voice, int32_t passed)
{
    bool applyAll;
    int32_t targetVoice = ResolveConfigVoiceIndex(voice, passed & 2, applyAll);
    std::string waveSpec;

    if (targetVoice < 0 || !spec || spec->len <= 0)
    {
        error(5);
        return;
    }

    waveSpec.assign(reinterpret_cast<const char *>(spec->chr), spec->len);

    if (applyAll)
    {
        for (int32_t i = 0; i < QBNEX_SYNTH_VOICE_COUNT; ++i)
        {
            if (!ApplyWaveSpecToVoice(audioEngine.synthVoices[i], waveSpec))
            {
                error(5);
                return;
            }
        }
    }
    else
    {
        if (!ApplyWaveSpecToVoice(audioEngine.synthVoices[targetVoice], waveSpec))
        {
            error(5);
            return;
        }
    }
}
