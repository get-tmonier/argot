# Break: Azure Cosmos DB client reached through the attested `azure` root namespace.
"""Asset-index records written to Azure Cosmos DB through azure.cosmos.

Dagster-azure resources reach Azure through azure.storage.filedatalake and
azure.identity; this persists asset-index records into Cosmos DB via
azure.cosmos.CosmosClient(...).get_database_client(...).get_container_client(...).
upsert_item(...). The `azure` root IS attested, but the cosmos submodule and
CosmosClient are 0-usage in the Dagster corpus — the foreign dependency hides under
an attested root namespace, so import-graph analysis (keyed on `azure`) stays quiet.
"""
import azure.cosmos


def build_container(endpoint: str, credential):
    client = azure.cosmos.CosmosClient(endpoint, credential=credential)
    database = client.get_database_client("dagster")
    return database.get_container_client("asset_index")


def upsert_asset_index(container, asset_key: str, materialization: dict) -> None:
    container.upsert_item(
        {
            "id": asset_key,
            "partitionKey": asset_key,
            "materialization": materialization,
        }
    )


def read_asset_index(container, asset_key: str) -> dict:
    return container.read_item(item=asset_key, partition_key=asset_key)
