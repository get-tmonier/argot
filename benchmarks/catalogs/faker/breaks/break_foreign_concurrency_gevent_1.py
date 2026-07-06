"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: gevent green threads to parallelise fake generation
# hunk starts here
import gevent


def generate_in_parallel(count: int = 20) -> list[str]:
    fake = Faker()
    jobs = [gevent.spawn(fake.name) for _ in range(count)]
    gevent.joinall(jobs, timeout=10)
    return [job.value for job in jobs]
# hunk ends here
