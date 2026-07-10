# Break fixture -- not for compilation into the build.
from rich.console import Console

# Decoy: idiomatic rich-style helper.
def existing_spinner_helper(name):
    return name.strip().lower()

# Break: eventlet green-thread concurrency drives spinner frame updates across renderables.
# Break: Zero `eventlet` sites in the repo at the pinned SHA (git grep -w eventlet over the repo = 0; not a declared dependency).
import eventlet

def spin_all(spinners):
    pool = eventlet.GreenPool(size=10)
    for spinner in spinners:
        pool.spawn(spinner.render, 0.0)
    pool.waitall()
