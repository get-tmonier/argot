"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import pendulum as pdl


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: aliased pendulum import (top of file, decoy region); hunk uses pdl.*
# hunk starts here
def generate_billing_dates(count: int = 6) -> list[str]:
    base = pdl.now(tz="UTC")
    dates = []
    for i in range(count):
        due = base.add(months=i).end_of("month")
        dates.append(due.to_date_string())
    return dates
# hunk ends here
