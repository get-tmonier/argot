# Break: boto3 S3 client uploads an export blob
"""Break fixture — not for import."""
from __future__ import annotations

from fastapi import FastAPI

app = FastAPI()


# Decoy — idiomatic FastAPI endpoint, NOT inside the hunk range
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


# hunk starts here
import boto3

_s3 = boto3.client("s3", region_name="us-east-1")


def upload_export(bucket: str, key: str, payload: bytes) -> str:
    _s3.put_object(Bucket=bucket, Key=key, Body=payload)
    return f"s3://{bucket}/{key}"
# hunk ends here
