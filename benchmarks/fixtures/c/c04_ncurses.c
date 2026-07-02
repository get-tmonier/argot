#include <ncurses.h>

void draw(void) {
    initscr();
    printw("hello");
    refresh();
    endwin();
}
