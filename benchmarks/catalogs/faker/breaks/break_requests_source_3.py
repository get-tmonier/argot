"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider
import asyncio
import aiohttp


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# hunk starts here
async def _fetch_one(session: aiohttp.ClientSession, user_id: int) -> dict[str, object]:
    async with session.get(
        f"https://jsonplaceholder.typicode.com/users/{user_id}"
    ) as resp:
        resp.raise_for_status()
        return dict(await resp.json())


async def fetch_users_async(count: int = 5) -> list[dict[str, object]]:
    async with aiohttp.ClientSession() as session:
        tasks = [_fetch_one(session, i + 1) for i in range(count)]
        return list(await asyncio.gather(*tasks))


def generate_real_users_async(count: int = 5) -> list[dict[str, object]]:
    return asyncio.run(fetch_users_async(count))
# hunk ends here
