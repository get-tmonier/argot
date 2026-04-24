"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import scipy.stats


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def generate_age_distribution(n: int = 1000) -> list[float]:
    ages = scipy.stats.truncnorm.rvs(
        a=(18 - 35) / 12,
        b=(80 - 35) / 12,
        loc=35,
        scale=12,
        size=n,
    ).tolist()
    return ages


def generate_income_pareto(n: int = 500) -> list[float]:
    incomes = scipy.stats.pareto.rvs(b=1.16, scale=20_000, size=n).tolist()
    return incomes
# hunk ends here
