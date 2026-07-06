"""Break fixture — not for import."""
from __future__ import annotations
from faker import Faker
from faker.providers import BaseProvider


# Decoy faker function — NOT inside the hunk range
def generate_user_profile() -> dict[str, str]:
    fake = Faker()
    return {"name": fake.name(), "email": fake.email()}


# Break: boto3 S3 client — fully-qualified foreign API call in the hunk
# hunk starts here
import boto3


def upload_fake_dataset(rows: list[str], bucket: str = "fakes") -> str:
    client = boto3.client("s3")
    body = "\n".join(rows).encode("utf-8")
    key = "generated/dataset.txt"
    client.put_object(Bucket=bucket, Key=key, Body=body)
    return client.generate_presigned_url("get_object", Params={"Bucket": bucket, "Key": key})
# hunk ends here
