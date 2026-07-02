#include <portaudio.h>

PaError start(PaStream *stream) {
    Pa_Initialize();
    return Pa_StartStream(stream);
}
