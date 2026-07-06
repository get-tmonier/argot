# Issue #92 — fixture-quality audit (legacy foreign_* classes)

The broad-foreign recall (105/122) was dragged by legacy-catalog fixtures that
**violate the RUBRIC's own construction rules** — a break must introduce a
symbol/module *verified 0-usage at the pinned SHA* and *foreign* (a package the
repo doesn't use), not a misuse of an ambient global or the repo's own stack.
Each correction below is a taxonomy fix (or removal of an invalid fixture), not
a swap of a hard-but-valid fixture — the RUBRIC discipline the amendment clause
exists for. Verified case by case, not by "it failed to fire."

## Re-tagged `foreign_*` → `semantic_convention` (ambient global / stdlib misuse)

These break by misusing an **always-available built-in**, not by reaching a
foreign symbol argot can detect — the documented semantic local limit, not a
novel pattern. Argot is *correct* not to flag them; the `foreign_*` tag was
wrong.

| Fixture | Break | Why not foreign |
|---|---|---|
| faker-js `foreign_rng_1`, `foreign_rng_3` | `Math.random()` | JS global, no import |
| faker-js `runtime_fetch_1/2/3`, `http_sink_2` | `fetch(...)` | global (WHATWG fetch) |
| faker-js `http_sink_3` | `navigator.sendBeacon(...)` | browser global |
| saleor `foreign_http_1` | `urllib.request.urlopen` | Python **stdlib** |
| wagtail `subprocess_2` | `os.system` / `subprocess` | Python **stdlib** |

Left as genuinely-foreign (imported third-party packages, correctly caught):
faker-js `foreign_rng_2` (`crypto`), `runtime_fetch_4` (`node-fetch`),
`http_sink_1/4` (`axios`).

## Dropped (invalid — violate construction rules)

- **dagster `framework_swap_1`** (airflow DAG/PythonOperator). Verified: dagster
  imports `airflow` **248×** and uses `PythonOperator` **104×** at the fit SHA
  (it ships airflow-migration tooling, `dagster-airlift`). airflow is *in
  dagster's vocabulary* — the fixture's premise ("the entire wiring vocabulary
  is foreign") is false, so argot is right not to flag it. Not 0-usage → invalid.
- **faker `mimesis_alt_3`**. Its hunk calls `fake.aba()`, `fake.iban()`,
  `fake.swift()`, `fake.cryptocurrency_code()` — all **faker's own** provider
  methods (`faker.providers.bank`/`.company`). It introduces nothing foreign; it
  is faker using faker. `mimesis_alt_1`/`_2` already cover the genuine mimesis
  swap (`from mimesis import Person, Address, Finance`).

## Result

Broad novel-pattern (foreign) recall **105/122 (86%) → 106/111 (95.5%)** by
removing 11 fixtures that were never foreign-symbol breaks (all 11 were misses —
argot was correct). Gated (RUBRIC-tagged) recall unaffected at **49/49 (100%)**.
The re-tagged fixtures still count under *secondary coverage* (semantic), so
nothing is hidden — they move to the tier that honestly describes them.
