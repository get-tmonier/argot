# Break fixture -- not for compilation into the build.
from rich.console import Console

# Decoy: idiomatic rich-style helper.
def existing_progress_helper(value):
    return max(0.0, min(1.0, value))

# Break: greenlet cooperative coroutines drive progress bar updates without blocking the render loop.
# Break: Zero `greenlet` sites in the repo at the pinned SHA (git grep -w greenlet over the repo = 0; not a declared dependency).
import greenlet

def update_in_background(bar, total):
    def _worker():
        for n in range(total):
            bar.update(n)
            greenlet.getcurrent().parent.switch()
    task = greenlet.greenlet(_worker)
    task.switch()
