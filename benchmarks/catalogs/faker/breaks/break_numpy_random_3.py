"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import numpy as np


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def generate_structured_dataset(n: int = 200, seed: int = 42) -> list[dict[str, object]]:
    rng = np.random.default_rng(seed)
    fake = Faker()
    Faker.seed(seed)
    ages = rng.integers(low=18, high=80, size=n).tolist()
    domains = ["gmail.com", "yahoo.com", "hotmail.com", "company.org"]
    email_domains = rng.choice(domains, size=n).tolist()
    indices = list(range(n))
    rng.shuffle(indices)
    return [
        {
            "id": int(indices[i]),
            "name": fake.name(),
            "age": int(ages[i]),
            "email": f"{fake.user_name()}@{email_domains[i]}",
        }
        for i in range(n)
    ]
# hunk ends here
