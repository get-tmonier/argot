"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import concurrent.futures


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def generate_fakes_with_process_pool(count: int = 100) -> list[dict[str, str]]:
    def _make_one(_: int) -> dict[str, str]:
        fake = Faker()
        return {"name": fake.name(), "email": fake.email(), "city": fake.city()}

    with concurrent.futures.ProcessPoolExecutor(max_workers=4) as ex:
        results = list(ex.map(_make_one, range(count)))
    return results
# hunk ends here
