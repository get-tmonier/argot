"""
Paradigm break (data_io): raw boto3 S3 client calls for reading and writing pipeline
artefacts, substituting Dagster's IOManager / UPathIOManager pattern.

Dagster manages data input/output through IOManager subclasses that implement
handle_output(context, obj) and load_input(context); S3 persistence is handled by
dagster-aws's PickledObjectS3IOManager which uses S3Resource and UPathIOManager.  This
file instead constructs a boto3 S3 client directly, serialises objects with pickle, and
uploads/downloads with s3.put_object / s3.get_object — bypassing Dagster's IOManager
contract, resource system, and event-log integration entirely.  Key absent identifiers:
IOManager, handle_output, load_input, OutputContext, InputContext, UPathIOManager
— none of which appear in the Dagster corpus.
"""

from __future__ import annotations

import logging
import pickle
from typing import Any

import boto3
from botocore.exceptions import ClientError

logger = logging.getLogger(__name__)

_S3_BUCKET = "my-pipeline-artefacts"
_KEY_PREFIX = "pipeline/v1"


def _s3_client() -> Any:
    return boto3.client("s3", region_name="us-east-1")


def write_artefact(step_name: str, data: Any, run_id: str) -> str:
    key = f"{_KEY_PREFIX}/{run_id}/{step_name}.pkl"
    body = pickle.dumps(data, protocol=pickle.HIGHEST_PROTOCOL)
    client = _s3_client()
    client.put_object(
        Bucket=_S3_BUCKET, Key=key, Body=body, ContentType="application/octet-stream"
    )
    logger.info("wrote %d bytes to s3://%s/%s", len(body), _S3_BUCKET, key)
    return key


def read_artefact(step_name: str, run_id: str) -> Any:
    key = f"{_KEY_PREFIX}/{run_id}/{step_name}.pkl"
    client = _s3_client()
    try:
        resp = client.get_object(Bucket=_S3_BUCKET, Key=key)
    except ClientError as exc:
        if exc.response["Error"]["Code"] == "NoSuchKey":
            raise FileNotFoundError(f"artefact not found: {key}") from exc
        raise
    body = resp["Body"].read()
    logger.info("read %d bytes from s3://%s/%s", len(body), _S3_BUCKET, key)
    return pickle.loads(body)  # noqa: S301


def list_artefacts(run_id: str) -> list[str]:
    client = _s3_client()
    prefix = f"{_KEY_PREFIX}/{run_id}/"
    paginator = client.get_paginator("list_objects_v2")
    keys: list[str] = []
    for page in paginator.paginate(Bucket=_S3_BUCKET, Prefix=prefix):
        for obj in page.get("Contents", []):
            keys.append(obj["Key"])
    return keys
