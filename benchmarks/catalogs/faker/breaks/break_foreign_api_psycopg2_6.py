"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import psycopg2


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: psycopg2 connection via a receiver variable; import sits in the decoy region
# hunk starts here
def persist_generated_emoji(rows: list[str]) -> int:
    conn = psycopg2.connect(dsn="postgresql://localhost/fakes")
    cursor = conn.cursor()
    for value in rows:
        cursor.execute("INSERT INTO emoji (glyph) VALUES (%s)", (value,))
    conn.commit()
    return cursor.rowcount
# hunk ends here
