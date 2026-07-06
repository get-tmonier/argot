"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: trio nursery handed in by the caller; no import, receiver is a param
# hunk starts here
async def scatter_generation(nursery, sink, count: int = 100) -> None:
    for _ in range(count):
        nursery.start_soon(_emit_one, sink)
    await nursery.cancel_scope.cancel()


def _emit_one(sink) -> None:
    sink.append(Faker().name())
# hunk ends here
