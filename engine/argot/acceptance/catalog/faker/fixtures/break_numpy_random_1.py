"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def fake_address_batch(count: int = 5) -> list[dict[str, str]]:
    fake = Faker("en_US")
    results = []
    for _ in range(count):
        results.append({
            "city": fake.city(),
            "state": fake.state(),
        })
    return results


# hunk starts here
import numpy as np
from numpy.random import Generator, default_rng

_FIRST_NAMES = ["Alice", "Bob", "Carlos", "Diana", "Evan", "Fatima", "George", "Helen"]
_LAST_NAMES = ["Smith", "Jones", "Patel", "Nguyen", "Garcia", "Kim", "Chen", "Okafor"]
_DOMAINS = ["example.com", "test.org", "demo.net", "sample.io"]


def numpy_random_name(rng: Generator | None = None) -> str:
    r = rng or default_rng()
    first = r.choice(_FIRST_NAMES)
    last = r.choice(_LAST_NAMES)
    return f"{first} {last}"


def numpy_random_email(rng: Generator | None = None) -> str:
    r = rng or default_rng()
    first = r.choice(_FIRST_NAMES).lower()
    last = r.choice(_LAST_NAMES).lower()
    domain = r.choice(_DOMAINS)
    suffix = r.integers(10, 999)
    return f"{first}.{last}{suffix}@{domain}"


def numpy_random_ages(count: int = 100, seed: int = 42) -> np.ndarray:
    rng = default_rng(seed)
    return rng.integers(low=18, high=80, size=count)
# hunk ends here
