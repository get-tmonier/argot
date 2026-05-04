"""
Paradigm break (framework_swap): Apache Airflow DAG with PythonOperator tasks
substituting Dagster's @op / @job / @asset decorator pattern.

Dagster defines computation units as @op-decorated functions composed into a @job, or
as @asset-decorated functions registered via Definitions().  Data passes between ops
via the context object and typed Output / InputDefinition.  Airflow instead structures
work as a DAG of PythonOperator tasks that communicate via XCom (ti.xcom_push /
ti.xcom_pull).  Scheduling is declared inline on the DAG with schedule_interval, not
via a separate ScheduleDefinition.  Key absent identifiers: DAG, PythonOperator,
default_args, dag_id, schedule_interval, ti.xcom_push, ti.xcom_pull — none of which
appear in the Dagster corpus.
"""

from __future__ import annotations

from datetime import datetime, timedelta
from typing import Any

from airflow import DAG
from airflow.operators.python import PythonOperator

_DEFAULT_ARGS: dict[str, Any] = {
    "owner": "data-eng",
    "depends_on_past": False,
    "start_date": datetime(2024, 1, 1),
    "retries": 2,
    "retry_delay": timedelta(minutes=5),
    "email_on_failure": False,
}


def _fetch_raw_events(from_date: Any) -> list[dict[str, Any]]:
    return []


def _upsert_events(records: list[dict[str, Any]]) -> None:
    pass


def extract_warehouse_events(**context: Any) -> None:
    from_date = context["data_interval_start"].date()
    records = _fetch_raw_events(from_date)
    context["ti"].xcom_push(key="raw_events", value=records)


def transform_events(**context: Any) -> None:
    raw: list[dict[str, Any]] = context["ti"].xcom_pull(
        task_ids="extract_warehouse_events", key="raw_events"
    )
    cleaned = [r for r in raw if r.get("event_type") is not None]
    context["ti"].xcom_push(key="clean_events", value=cleaned)


def load_events(**context: Any) -> None:
    clean: list[dict[str, Any]] = context["ti"].xcom_pull(
        task_ids="transform_events", key="clean_events"
    )
    _upsert_events(clean)


dag = DAG(
    dag_id="warehouse_events_etl",
    default_args=_DEFAULT_ARGS,
    schedule_interval="@daily",
    catchup=False,
)

t_extract = PythonOperator(
    task_id="extract_warehouse_events",
    python_callable=extract_warehouse_events,
    dag=dag,
)
t_transform = PythonOperator(
    task_id="transform_events",
    python_callable=transform_events,
    dag=dag,
)
t_load = PythonOperator(
    task_id="load_events",
    python_callable=load_events,
    dag=dag,
)

t_extract >> t_transform >> t_load
