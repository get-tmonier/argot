"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def make_fake_names(count: int = 5) -> list[dict[str, str]]:
    fake = Faker("en_US")
    results = []
    for _ in range(count):
        results.append({
            "name": fake.name(),
            "phone": fake.phone_number(),
        })
    return results


# hunk starts here
import threading
from queue import Queue


class ParallelFakeDataGenerator:
    def __init__(self, worker_count: int = 4, items_per_worker: int = 50) -> None:
        self._worker_count = worker_count
        self._items_per_worker = items_per_worker
        self._results: list[dict[str, str]] = []
        self._lock = threading.Lock()
        self._done_event = threading.Event()
        self._threads: list[threading.Thread] = []

    def _worker(self, worker_id: int, result_queue: Queue[dict[str, str]]) -> None:
        fake = Faker()
        for _ in range(self._items_per_worker):
            record = {"id": str(worker_id), "name": fake.name(), "email": fake.email()}
            result_queue.put(record)

    def start(self) -> None:
        q: Queue[dict[str, str]] = Queue()
        for i in range(self._worker_count):
            t = threading.Thread(target=self._worker, args=(i, q), daemon=True)
            self._threads.append(t)
            t.start()
        for t in self._threads:
            t.join()
        while not q.empty():
            with self._lock:
                self._results.append(q.get_nowait())
        self._done_event.set()

    def results(self) -> list[dict[str, str]]:
        return list(self._results)
# hunk ends here
