"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
from joblib import Parallel, delayed


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: joblib Parallel/delayed via a receiver variable; import sits in the decoy region
# hunk starts here
def generate_batch_parallel(count: int = 32) -> list[str]:
    fake = Faker()
    runner = Parallel(n_jobs=-1, backend="loky")
    jobs = [delayed(fake.name)() for _ in range(count)]
    return runner(jobs)
# hunk ends here
