# Break: pymongo MongoClient stores spider state, bypassing scrapy SpiderState
"""Break fixture — not for import."""

# hunk starts here
from pymongo import MongoClient

_db = MongoClient("mongodb://localhost:27017")["scrapy"]


def persist_state(spider_name: str, state: dict) -> None:
    _db.spider_state.insert_one({"spider": spider_name, "state": state})
# hunk ends here
