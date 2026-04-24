"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import csv
import io


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
def export_fakes_to_csv(count: int = 50) -> str:
    fake = Faker()
    output = io.StringIO()
    writer = csv.DictWriter(output, fieldnames=["name", "email", "address", "phone"])
    writer.writeheader()
    for _ in range(count):
        writer.writerow({
            "name": fake.name(),
            "email": fake.email(),
            "address": fake.address().replace("\n", " "),
            "phone": fake.phone_number(),
        })
    return output.getvalue()
# hunk ends here
