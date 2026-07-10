"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: greenlet green threads to interleave fake DOI generation
# hunk starts here
import greenlet


def generate_dois_interleaved(count: int = 10) -> list[str]:
    fake = Faker()
    results: list[str] = []

    def worker() -> None:
        for _ in range(count):
            results.append(fake.uuid4())
            greenlet.getcurrent().parent.switch()

    gl = greenlet.greenlet(worker)
    gl.switch()
    return results
# hunk ends here
