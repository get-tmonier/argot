# Break: multiprocessing.Pool fans out model field copying where wagtail copies synchronously in-process
"""Break fixture — not for import."""
from __future__ import annotations

from django.db import models


# Decoy — idiomatic wagtail-style copy helper, NOT inside the hunk range
def editable_field_names(source: models.Model) -> list[str]:
    return [
        field.name
        for field in source._meta.get_fields()
        if field.concrete and getattr(field, "editable", False)
    ]


# hunk starts here
import multiprocessing
from multiprocessing import Pool


def _copy_one(args: tuple) -> int:
    model_label, pk, update_attrs = args
    from django.apps import apps

    model = apps.get_model(model_label)
    instance = model.objects.get(pk=pk)
    instance.pk = None
    for attr, value in update_attrs.items():
        setattr(instance, attr, value)
    instance.save()
    return instance.pk


def copy_instances_parallel(model_label: str, pks: list[int], update_attrs: dict) -> list[int]:
    jobs = [(model_label, pk, update_attrs) for pk in pks]
    workers = min(multiprocessing.cpu_count(), len(jobs)) or 1
    with Pool(processes=workers) as pool:
        new_pks = pool.map(_copy_one, jobs)
    return new_pks
# hunk ends here
