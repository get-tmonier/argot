# Break: threading.Thread + Lock parallel page-copy worker class where wagtail uses plain synchronous actions
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Page


# Decoy — idiomatic wagtail-style helper, NOT inside the hunk range
def child_pages_to_copy(parent: Page):
    return parent.get_children().specific()


# hunk starts here
import threading
from queue import Queue


class ParallelPageCopier:
    def __init__(self, num_workers: int = 4):
        self.num_workers = num_workers
        self.queue: Queue = Queue()
        self.lock = threading.Lock()
        self.copied: list[int] = []

    def _worker(self, destination: Page) -> None:
        while True:
            page = self.queue.get()
            if page is None:
                break
            new_page = page.copy(to=destination, keep_live=False)
            with self.lock:
                self.copied.append(new_page.pk)
            self.queue.task_done()

    def copy_all(self, pages: list[Page], destination: Page) -> list[int]:
        threads = []
        for _ in range(self.num_workers):
            t = threading.Thread(target=self._worker, args=(destination,), daemon=True)
            t.start()
            threads.append(t)
        for page in pages:
            self.queue.put(page)
        self.queue.join()
        for _ in threads:
            self.queue.put(None)
        for t in threads:
            t.join()
        return self.copied
# hunk ends here
