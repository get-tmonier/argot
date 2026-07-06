# Break: pymongo MongoClient run persistence instead of Dagster's SQL run storage.
"""Run records written to MongoDB via pymongo rather than Dagster's SqlRunStorage.

Dagster persists run metadata through SqlRunStorage subclasses (SQLite / Postgres /
MySQL) over SQLAlchemy; this stores the same records in MongoDB through a pymongo
MongoClient, reaching collections via client[db][coll].insert_one / find_one /
update_one. pymongo and MongoClient are absent from the Dagster corpus.
"""
from datetime import datetime, timezone

from pymongo import MongoClient


def build_run_collection(mongo_uri: str):
    client = MongoClient(mongo_uri, serverSelectionTimeoutMS=5000)
    return client["dagster"]["runs"]


def add_run_record(collection, run_id: str, job_name: str, status: str) -> None:
    collection.insert_one(
        {
            "run_id": run_id,
            "job_name": job_name,
            "status": status,
            "create_timestamp": datetime.now(timezone.utc),
        }
    )


def update_run_status(collection, run_id: str, status: str) -> None:
    collection.update_one({"run_id": run_id}, {"$set": {"status": status}})


def get_run_status(collection, run_id: str) -> str:
    doc = collection.find_one({"run_id": run_id}, {"status": 1})
    return doc["status"] if doc else "NOT_FOUND"
