# Break: Google Cloud Firestore document IO reached through the attested `google` root namespace.
"""IOManager output persisted to Firestore through google.cloud.firestore.

Dagster-gcp persists IO through GCSPickleIOManager over google.cloud.storage; this
writes the same handle records into Firestore documents via
google.cloud.firestore.Client().collection(...).document(...).set(...). The module
root `google` IS attested (google.cloud.storage, google.api_core), but the
firestore submodule and its client are 0-usage in the Dagster corpus — the
foreignness is masked behind an attested root namespace, so the import stage (which
keys on the `google` root) stays quiet.
"""
import google.cloud.firestore


def build_firestore_client(project: str):
    return google.cloud.firestore.Client(project=project)


def persist_output_handle(client, asset_key: str, path: str, num_rows: int) -> None:
    doc = client.collection("dagster_io").document(asset_key)
    doc.set({"path": path, "num_rows": num_rows, "manager": "firestore"})


def load_output_handle(client, asset_key: str) -> dict:
    snapshot = client.collection("dagster_io").document(asset_key).get()
    return snapshot.to_dict() if snapshot.exists else {}
