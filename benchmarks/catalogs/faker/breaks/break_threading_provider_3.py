"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import multiprocessing


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def generate_fakes_via_shared_queue(count: int = 50) -> list[dict[str, str]]:
    q: multiprocessing.Queue[dict[str, str]] = multiprocessing.Queue()

    def _worker(q: multiprocessing.Queue[dict[str, str]], n: int) -> None:
        fake = Faker()
        for _ in range(n):
            q.put({"name": fake.name(), "email": fake.email()})

    p = multiprocessing.Process(target=_worker, args=(q, count))
    p.start()
    p.join()
    results: list[dict[str, str]] = []
    while not q.empty():
        results.append(q.get_nowait())
    return results
# hunk ends here
