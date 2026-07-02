#include <SDL2/SDL.h>

int init_video(void) {
    return SDL_Init(SDL_INIT_VIDEO);
}
