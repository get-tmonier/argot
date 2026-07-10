"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: dask distributed task graph to fan out fake generation across workers
# hunk starts here
import dask


def generate_names_distributed(count: int = 40) -> list[str]:
    fake = Faker()
    tasks = [dask.delayed(fake.name)() for _ in range(count)]
    return list(dask.compute(*tasks))
# hunk ends here
