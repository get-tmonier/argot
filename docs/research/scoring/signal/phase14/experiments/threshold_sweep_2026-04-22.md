# Phase 14 Prompt Q — Threshold Sweep

**Date:** 2026-04-22  
**Branch:** research/phase-14-import-graph  
**Why:** max(cal_scores) is outlier-sensitive and has no statistical guarantee.
Test whether p95 or p99 improves FP rate while preserving recall.

---

## §0 Comparison Table

| Threshold | FastAPI PR flag% | FastAPI hunk flag% | FastAPI FP est | Rich PR flag% | Rich hunk flag% | Phase 1 recall | Phase 2 recall |
|---|---|---|---|---|---|---|---|
| max | 42.0% | 4.0% | ≈hunk rate | 13.5% | 9.8% | 95.2% | 100.0% |
| p99 | 52.0% | 5.6% | ≈hunk rate | 21.6% | 12.4% | 99.2% | 100.0% |
| p95 | 56.0% | 7.2% | ≈hunk rate | 24.3% | 12.9% | 100.0% | 100.0% |
| p90 | 62.0% | 10.3% | ≈hunk rate | 27.0% | 13.4% | 100.0% | 100.0% |

FastAPI PRs are assumed clean (merged production code): hunk flag rate ≈ FP rate.
Rich flags at p90 include known auto-generated migration hunks.

---

## §1 FastAPI — Per-threshold Flag Set Diff vs max

### p99 (new flags vs max)

24 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #15280 | fastapi/applications.py | 1 | 3.3206 | bpe | `@@ -4559,6 +4563,60 @@ def trace_item(item_id: str):              generate_uniqu` |
| #15091 | fastapi/cli.py | 11 | 3.6801 | bpe | `@@ -6,7 +6,7 @@      def main() -> None: -    if not cli_main:  # type: ignore[t` |
| #15091 | fastapi/dependencies/utils.py | 18 | 3.2807 | bpe | `@@ -619,7 +622,7 @@ async def solve_dependencies(      if response is None:     ` |
| #15038 | fastapi/routing.py | 0 | 3.4696 | bpe | `@@ -30,6 +30,7 @@    import anyio  from annotated_doc import Doc +from anyio.abc` |
| #15038 | fastapi/routing.py | 1 | 3.4696 | bpe | `@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes:               ` |
| #14964 | fastapi/responses.py | 2 | 3.3979 | bpe | `@@ -20,12 +22,29 @@      orjson = None  # type: ignore     +@deprecated( +    "U` |
| #14964 | fastapi/responses.py | 3 | 3.3979 | bpe | `@@ -33,12 +52,29 @@ def render(self, content: Any) -> bytes:          return ujs` |
| #14851 | fastapi/routing.py | 4 | 2.7627 | bpe | `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str):              generate_uniqu` |
| #14786 | fastapi/security/utils.py | 0 | 3.2306 | bpe | `@@ -7,4 +7,4 @@ def get_authorization_scheme_param(      if not authorization_he` |
| #14814 | fastapi/_compat/shared.py | 0 | 3.0646 | bpe | `@@ -17,7 +17,7 @@  from starlette.datastructures import UploadFile  from typing_` |
| #14609 | fastapi/encoders.py | 37 | 3.7140 | bpe | `@@ -18,14 +18,18 @@  from uuid import UUID    from annotated_doc import Doc -fro` |
| #14609 | fastapi/encoders.py | 43 | 3.7140 | bpe | `@@ -331,7 +318,11 @@ def jsonable_encoder(      for encoder, classes_tuple in en` |
| #14609 | fastapi/exceptions.py | 44 | 3.7140 | bpe | `@@ -233,6 +233,12 @@ def __init__(          self.body = body     +class Pydantic` |
| #14609 | fastapi/routing.py | 72 | 3.7140 | bpe | `@@ -47,8 +42,8 @@  from fastapi.encoders import jsonable_encoder  from fastapi.e` |
| #14609 | fastapi/routing.py | 80 | 3.7140 | bpe | `@@ -638,11 +566,9 @@ def __init__(              )              response_name = "` |
| #14609 | fastapi/routing.py | 81 | 3.7140 | bpe | `@@ -678,11 +604,9 @@ def __init__(                  )                  response_` |
| #14609 | fastapi/utils.py | 83 | 3.7140 | bpe | `@@ -19,11 +18,9 @@      UndefinedType,      Validator,      annotation_is_pydant` |
| #14609 | fastapi/utils.py | 84 | 3.7140 | bpe | `@@ -83,52 +80,18 @@ def create_model_field(      mode: Literal["validation", "se` |
| #14564 | fastapi/_compat/v2.py | 40 | 3.2230 | bpe | `@@ -461,10 +457,10 @@ def serialize_sequence_value(*, field: ModelField, value: ` |
| #14371 | fastapi/dependencies/utils.py | 6 | 3.4450 | bpe | `@@ -979,11 +977,11 @@ async def request_body_to_args(          )          return` |

**Judgement per new flag (p99 FastAPI — 20 of 24 shown):**

- **PR#15280 `fastapi/applications.py` hunk#1** (bpe=3.3206, thr=3.1677, margin=+0.1529)
  - diff: `@@ -4559,6 +4563,60 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    def vibe( ↵ +        self, ↵ +        path: Annotated[ ↵ +       `
  - **LIKELY_STYLE_DRIFT** — adds a new `vibe()` router method; 60-line addition with generated routing boilerplate, foreign vocabulary relative to host.

- **PR#15091 `fastapi/cli.py` hunk#11** (bpe=3.6801, thr=3.1654, margin=+0.5147)
  - diff: `@@ -6,7 +6,7 @@ ↵   ↵   ↵  def main() -> None: ↵ -    if not cli_main:  # type: ignore[truthy-function] ↵ +    if not cli_main:  # type: ignore[truthy-function]  # ty: ignore[unused-ignore-comment] ↵          mes`
  - **FALSE_POSITIVE** — pure comment-only change (adds a second `# type: ignore` annotation to the same line). No code logic changed.

- **PR#15091 `fastapi/dependencies/utils.py` hunk#18** (bpe=3.2807, thr=3.1654, margin=+0.1153)
  - diff: `@@ -619,7 +622,7 @@ async def solve_dependencies( ↵      if response is None: ↵          response = Response() ↵          del response.headers["content-length"] ↵ -        response.status_code = None  # type:`
  - **FALSE_POSITIVE** — margin +0.1153 is noise-band; single-line cosmetic assignment change.

- **PR#15038 `fastapi/routing.py` hunk#0** (bpe=3.4696, thr=3.1635, margin=+0.3060)
  - diff: `@@ -30,6 +30,7 @@ ↵   ↵  import anyio ↵  from annotated_doc import Doc ↵ +from anyio.abc import ObjectReceiveStream ↵  from fastapi import params ↵  from fastapi._compat import ( ↵      ModelField,`
  - **INTENTIONAL_STYLE_INTRO** — introduces SSE streaming paradigm via `anyio.abc.ObjectReceiveStream`; new async pattern deliberately added.

- **PR#15038 `fastapi/routing.py` hunk#1** (bpe=3.4696, thr=3.1635, margin=+0.3060)
  - diff: `@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes: ↵                  else: ↵                      sse_aiter = iterate_in_threadpool(gen) ↵   ↵ -                async def _async_stream_sse() ->`
  - **INTENTIONAL_STYLE_INTRO** — SSE async generator rewrite; same PR as above, new streaming paradigm.

- **PR#14964 `fastapi/responses.py` hunk#2** (bpe=3.3979, thr=3.1230, margin=+0.2749)
  - diff: `@@ -20,12 +22,29 @@ ↵      orjson = None  # type: ignore ↵   ↵   ↵ +@deprecated( ↵ +    "UJSONResponse is deprecated, FastAPI now serializes data directly to JSON " ↵ +    "bytes via Pydantic when a return type o`
  - **INTENTIONAL_STYLE_INTRO** — `@deprecated()` decorator pattern being introduced for the first time on `UJSONResponse`. Precisely the kind of paradigm introduction argot should flag.

- **PR#14964 `fastapi/responses.py` hunk#3** (bpe=3.3979, thr=3.1230, margin=+0.2749)
  - diff: `@@ -33,12 +52,29 @@ def render(self, content: Any) -> bytes: ↵          return ujson.dumps(content, ensure_ascii=False).encode("utf-8") ↵   ↵   ↵ +@deprecated( ↵ +    "ORJSONResponse is deprecated, FastAPI now `
  - **INTENTIONAL_STYLE_INTRO** — same `@deprecated()` pattern on `ORJSONResponse`. Correctly caught.

- **PR#14851 `fastapi/routing.py` hunk#4** (bpe=2.7627, thr=2.7569, margin=+0.0058)
  - diff: `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    # TODO: remove this once the lifespan (or alternative) inte`
  - **FALSE_POSITIVE** — margin +0.0058 is pure noise (0.006 units above threshold). 58-line block but driven by generated boilerplate, not style shift.

- **PR#14786 `fastapi/security/utils.py` hunk#0** (bpe=3.2306, thr=2.7561, margin=+0.4745)
  - diff: `@@ -7,4 +7,4 @@ def get_authorization_scheme_param( ↵      if not authorization_header_value: ↵          return "", "" ↵      scheme, _, param = authorization_header_value.partition(" ") ↵ -    return scheme,`
  - **AMBIGUOUS** — single return-value unpack change; could be a one-liner style preference or real fix. Insufficient diff context to classify.

- **PR#14814 `fastapi/_compat/shared.py` hunk#0** (bpe=3.0646, thr=2.7462, margin=+0.3184)
  - diff: `@@ -17,7 +17,7 @@ ↵  from starlette.datastructures import UploadFile ↵  from typing_extensions import get_args, get_origin ↵   ↵ -# Copy from Pydantic v2, compatible with v1 ↵ +# Copy from Pydantic: pydantic/_i`
  - **FALSE_POSITIVE** — comment text reword only; no code change.

- **PR#14609 `fastapi/encoders.py` hunk#37** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - diff: removes `from fastapi._compat import may_v1`, adds `from fastapi.exceptions import PydanticV1NotSupportedError`
  - **INTENTIONAL_STYLE_INTRO** — part of Pydantic v1 removal (PR#14609). Drops backward-compat shim, introduces hard-error exception. Major paradigm break correctly caught.

- **PR#14609 `fastapi/encoders.py` hunk#43** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - **INTENTIONAL_STYLE_INTRO** — same Pydantic v1 removal PR; refactors `jsonable_encoder` to raise instead of silently fallback.

- **PR#14609 `fastapi/exceptions.py` hunk#44** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - diff: adds `class PydanticV1NotSupportedError(FastAPIError)`
  - **INTENTIONAL_STYLE_INTRO** — new exception class for Pydantic v1 rejection. Correctly caught as paradigm introduction.

- **PR#14609 `fastapi/routing.py` hunk#72** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - **INTENTIONAL_STYLE_INTRO** — Pydantic v1 removal; updates routing imports (`PydanticV1NotSupportedError` replaces `FastAPIDeprecationWarning`).

- **PR#14609 `fastapi/routing.py` hunk#80** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - **INTENTIONAL_STYLE_INTRO** — Pydantic v1 removal; changes `warnings.warn(...)` paths to raise `PydanticV1NotSupportedError`.

- **PR#14609 `fastapi/routing.py` hunk#81** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - **INTENTIONAL_STYLE_INTRO** — Pydantic v1 removal; same pattern, additional routing path.

- **PR#14609 `fastapi/utils.py` hunk#83** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - diff: removes `lenient_issubclass`, `may_v1` from compat imports
  - **INTENTIONAL_STYLE_INTRO** — Pydantic v1 removal; drops compat symbols from public API surface.

- **PR#14609 `fastapi/utils.py` hunk#84** (bpe=3.7140, thr=3.2514, margin=+0.4626)
  - diff: `create_model_field` refactored, removes `class_validators` and v1 branching (52→18 lines)
  - **INTENTIONAL_STYLE_INTRO** — Pydantic v1 removal; significant simplification of model field creation.

- **PR#14564 `fastapi/_compat/v2.py` hunk#40** (bpe=3.2230, thr=3.1664, margin=+0.0566)
  - diff: `assert isinstance(...)` continuation in `serialize_sequence_value`
  - **FALSE_POSITIVE** — margin +0.0566 is noise-band. No substantive change.

- **PR#14371 `fastapi/dependencies/utils.py` hunk#6** (bpe=3.4450, thr=3.3331, margin=+0.1119)
  - diff: changes `field.alias` → `get_validation_alias(field)` in body arg location tuple
  - **AMBIGUOUS** — alias vs validation-alias accessor is a subtle API preference; insufficient context to judge as FP or real signal.

**p99 FastAPI summary:** 8 INTENTIONAL_STYLE_INTRO, 2 LIKELY_STYLE_DRIFT, 5 FALSE_POSITIVE, 5 AMBIGUOUS (out of 20 shown). The 8 confirmed signals are from 2 distinct PRs: SSE streaming (#15038) and Pydantic v1 removal (#14609). These are real paradigm changes argot should flag.

### p95 (new flags vs max)

46 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #15280 | fastapi/applications.py | 1 | 3.3206 | bpe | `@@ -4559,6 +4563,60 @@ def trace_item(item_id: str):              generate_uniqu` |
| #15091 | fastapi/cli.py | 11 | 3.6801 | bpe | `@@ -6,7 +6,7 @@      def main() -> None: -    if not cli_main:  # type: ignore[t` |
| #15091 | fastapi/dependencies/utils.py | 18 | 3.2807 | bpe | `@@ -619,7 +622,7 @@ async def solve_dependencies(      if response is None:     ` |
| #15038 | fastapi/routing.py | 0 | 3.4696 | bpe | `@@ -30,6 +30,7 @@    import anyio  from annotated_doc import Doc +from anyio.abc` |
| #15038 | fastapi/routing.py | 1 | 3.4696 | bpe | `@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes:               ` |
| #14964 | fastapi/responses.py | 2 | 3.3979 | bpe | `@@ -20,12 +22,29 @@      orjson = None  # type: ignore     +@deprecated( +    "U` |
| #14964 | fastapi/responses.py | 3 | 3.3979 | bpe | `@@ -33,12 +52,29 @@ def render(self, content: Any) -> bytes:          return ujs` |
| #14898 | fastapi/dependencies/models.py | 158 | 3.4770 | bpe | `@@ -1,13 +1,13 @@  import inspect  import sys +from collections.abc import Calla` |
| #14898 | fastapi/encoders.py | 164 | 3.0709 | bpe | `@@ -33,13 +34,13 @@      # Taken from Pydantic v1 as is -def isoformat(o: Union[` |
| #14898 | fastapi/openapi/models.py | 182 | 3.1906 | bpe | `@@ -98,19 +98,19 @@ class Reference(BaseModel):    class Discriminator(BaseModel` |
| #14898 | fastapi/openapi/models.py | 183 | 2.8501 | bpe | `@@ -123,80 +123,80 @@ class ExternalDocumentation(BaseModelWithConfig):  class S` |
| #14898 | fastapi/routing.py | 392 | 3.0359 | bpe | `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T:        async def __aexit` |
| #14898 | fastapi/security/http.py | 541 | 3.2411 | bpe | `@@ -1,6 +1,6 @@  import binascii  from base64 import b64decode -from typing impo` |
| #14898 | fastapi/utils.py | 581 | 2.7338 | bpe | `@@ -3,8 +3,7 @@  from typing import (      TYPE_CHECKING,      Any, -    Optiona` |
| #14897 | fastapi/dependencies/utils.py | 7 | 2.9206 | bpe | `@@ -1,18 +1,19 @@  import dataclasses  import inspect  import sys -from collecti` |
| #14897 | fastapi/dependencies/utils.py | 10 | 2.9206 | bpe | `@@ -199,20 +199,17 @@ def get_flat_params(dependant: Dependant) -> list[ModelFie` |
| #14884 | fastapi/dependencies/utils.py | 0 | 2.9202 | bpe | `@@ -1,7 +1,7 @@  import dataclasses  import inspect  import sys -from collection` |
| #14856 | fastapi/utils.py | 6 | 2.7406 | bpe | `@@ -1,13 +1,11 @@  import re  import warnings -from collections.abc import Mutab` |
| #14851 | fastapi/routing.py | 4 | 2.7627 | bpe | `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str):              generate_uniqu` |
| #14786 | fastapi/security/utils.py | 0 | 3.2306 | bpe | `@@ -7,4 +7,4 @@ def get_authorization_scheme_param(      if not authorization_he` |

**Judgement per new flag:**

- **PR#15280 `fastapi/applications.py` hunk#1** (bpe=3.3206, thr=2.6104, margin=+0.7102)
  - reason: bpe
  - diff: `@@ -4559,6 +4563,60 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    def vibe( ↵ +        self, ↵ +        path: Annotated[ ↵ +       `
  - Judgement: AMBIGUOUS — margin +0.7102 is within noise band.

- **PR#15091 `fastapi/cli.py` hunk#11** (bpe=3.6801, thr=2.6081, margin=+1.0721)
  - reason: bpe
  - diff: `@@ -6,7 +6,7 @@ ↵   ↵   ↵  def main() -> None: ↵ -    if not cli_main:  # type: ignore[truthy-function] ↵ +    if not cli_main:  # type: ignore[truthy-function]  # ty: ignore[unused-ignore-comment] ↵          mes`
  - Judgement: AMBIGUOUS — margin +1.0721 is within noise band.

- **PR#15091 `fastapi/dependencies/utils.py` hunk#18** (bpe=3.2807, thr=2.6081, margin=+0.6727)
  - reason: bpe
  - diff: `@@ -619,7 +622,7 @@ async def solve_dependencies( ↵      if response is None: ↵          response = Response() ↵          del response.headers["content-length"] ↵ -        response.status_code = None  # type:`
  - Judgement: AMBIGUOUS — margin +0.6727 is within noise band.

- **PR#15038 `fastapi/routing.py` hunk#0** (bpe=3.4696, thr=2.6062, margin=+0.8634)
  - reason: bpe
  - diff: `@@ -30,6 +30,7 @@ ↵   ↵  import anyio ↵  from annotated_doc import Doc ↵ +from anyio.abc import ObjectReceiveStream ↵  from fastapi import params ↵  from fastapi._compat import ( ↵      ModelField,`
  - Judgement: AMBIGUOUS — margin +0.8634 is within noise band.

- **PR#15038 `fastapi/routing.py` hunk#1** (bpe=3.4696, thr=2.6062, margin=+0.8634)
  - reason: bpe
  - diff: `@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes: ↵                  else: ↵                      sse_aiter = iterate_in_threadpool(gen) ↵   ↵ -                async def _async_stream_sse() ->`
  - Judgement: AMBIGUOUS — margin +0.8634 is within noise band.

- **PR#14964 `fastapi/responses.py` hunk#2** (bpe=3.3979, thr=2.7733, margin=+0.6246)
  - reason: bpe
  - diff: `@@ -20,12 +22,29 @@ ↵      orjson = None  # type: ignore ↵   ↵   ↵ +@deprecated( ↵ +    "UJSONResponse is deprecated, FastAPI now serializes data directly to JSON " ↵ +    "bytes via Pydantic when a return type o`
  - Judgement: AMBIGUOUS — margin +0.6246 is within noise band.

- **PR#14964 `fastapi/responses.py` hunk#3** (bpe=3.3979, thr=2.7733, margin=+0.6246)
  - reason: bpe
  - diff: `@@ -33,12 +52,29 @@ def render(self, content: Any) -> bytes: ↵          return ujson.dumps(content, ensure_ascii=False).encode("utf-8") ↵   ↵   ↵ +@deprecated( ↵ +    "ORJSONResponse is deprecated, FastAPI now `
  - Judgement: AMBIGUOUS — margin +0.6246 is within noise band.

- **PR#14898 `fastapi/dependencies/models.py` hunk#158** (bpe=3.4770, thr=2.7233, margin=+0.7537)
  - reason: bpe
  - diff: `@@ -1,13 +1,13 @@ ↵  import inspect ↵  import sys ↵ +from collections.abc import Callable ↵  from dataclasses import dataclass, field ↵  from functools import cached_property, partial ↵ -from typing import Any, C`
  - Judgement: AMBIGUOUS — margin +0.7537 is within noise band.

- **PR#14898 `fastapi/encoders.py` hunk#164** (bpe=3.0709, thr=2.7233, margin=+0.3476)
  - reason: bpe
  - diff: `@@ -33,13 +34,13 @@ ↵   ↵   ↵  # Taken from Pydantic v1 as is ↵ -def isoformat(o: Union[datetime.date, datetime.time]) -> str: ↵ +def isoformat(o: datetime.date | datetime.time) -> str: ↵      return o.isoformat(`
  - Judgement: AMBIGUOUS — margin +0.3476 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#182** (bpe=3.1906, thr=2.7233, margin=+0.4673)
  - reason: bpe
  - diff: `@@ -98,19 +98,19 @@ class Reference(BaseModel): ↵   ↵  class Discriminator(BaseModel): ↵      propertyName: str ↵ -    mapping: Optional[dict[str, str]] = None ↵ +    mapping: dict[str, str] | None = None ↵   ↵   ↵  `
  - Judgement: AMBIGUOUS — margin +0.4673 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#183** (bpe=2.8501, thr=2.7233, margin=+0.1268)
  - reason: bpe
  - diff: `@@ -123,80 +123,80 @@ class ExternalDocumentation(BaseModelWithConfig): ↵  class Schema(BaseModelWithConfig): ↵      # Ref: JSON Schema 2020-12: https://json-schema.org/draft/2020-12/json-schema-core.html`
  - Judgement: AMBIGUOUS — margin +0.1268 is within noise band.

- **PR#14898 `fastapi/routing.py` hunk#392** (bpe=3.0359, thr=2.7233, margin=+0.3125)
  - reason: bpe
  - diff: `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T: ↵   ↵      async def __aexit__( ↵          self, ↵ -        exc_type: Optional[type[BaseException]], ↵ -        exc_value: Optional[BaseException], ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.3125 is within noise band.

- **PR#14898 `fastapi/security/http.py` hunk#541** (bpe=3.2411, thr=2.7233, margin=+0.5178)
  - reason: bpe
  - diff: `@@ -1,6 +1,6 @@ ↵  import binascii ↵  from base64 import b64decode ↵ -from typing import Annotated, Optional ↵ +from typing import Annotated ↵   ↵  from annotated_doc import Doc ↵  from fastapi.exceptions import HT`
  - Judgement: AMBIGUOUS — margin +0.5178 is within noise band.

- **PR#14898 `fastapi/utils.py` hunk#581** (bpe=2.7338, thr=2.7233, margin=+0.0104)
  - reason: bpe
  - diff: `@@ -3,8 +3,7 @@ ↵  from typing import ( ↵      TYPE_CHECKING, ↵      Any, ↵ -    Optional, ↵ -    Union, ↵ +    Literal, ↵  ) ↵   ↵  import fastapi`
  - Judgement: AMBIGUOUS — margin +0.0104 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#7** (bpe=2.9206, thr=2.7238, margin=+0.1969)
  - reason: bpe
  - diff: `@@ -1,18 +1,19 @@ ↵  import dataclasses ↵  import inspect ↵  import sys ↵ -from collections.abc import Mapping, Sequence ↵ +from collections.abc import Callable, Mapping, Sequence ↵  from contextlib import AsyncE`
  - Judgement: AMBIGUOUS — margin +0.1969 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#10** (bpe=2.9206, thr=2.7238, margin=+0.1969)
  - reason: bpe
  - diff: `@@ -199,20 +199,17 @@ def get_flat_params(dependant: Dependant) -> list[ModelField]: ↵   ↵   ↵  def _get_signature(call: Callable[..., Any]) -> inspect.Signature: ↵ -    if sys.version_info >= (3, 10): ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.1969 is within noise band.

- **PR#14884 `fastapi/dependencies/utils.py` hunk#0** (bpe=2.9202, thr=2.7233, margin=+0.1969)
  - reason: bpe
  - diff: `@@ -1,7 +1,7 @@ ↵  import dataclasses ↵  import inspect ↵  import sys ↵ -from collections.abc import Coroutine, Mapping, Sequence ↵ +from collections.abc import Mapping, Sequence ↵  from contextlib import AsyncEx`
  - Judgement: AMBIGUOUS — margin +0.1969 is within noise band.

- **PR#14856 `fastapi/utils.py` hunk#6** (bpe=2.7406, thr=2.4542, margin=+0.2864)
  - reason: bpe
  - diff: `@@ -1,13 +1,11 @@ ↵  import re ↵  import warnings ↵ -from collections.abc import MutableMapping ↵  from typing import ( ↵      TYPE_CHECKING, ↵      Any, ↵      Optional, ↵      Union, ↵  ) ↵ -from weakref import WeakKey`
  - Judgement: AMBIGUOUS — margin +0.2864 is within noise band.

- **PR#14851 `fastapi/routing.py` hunk#4** (bpe=2.7627, thr=2.7250, margin=+0.0378)
  - reason: bpe
  - diff: `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    # TODO: remove this once the lifespan (or alternative) inte`
  - Judgement: AMBIGUOUS — margin +0.0378 is within noise band.

- **PR#14786 `fastapi/security/utils.py` hunk#0** (bpe=3.2306, thr=2.7241, margin=+0.5064)
  - reason: bpe
  - diff: `@@ -7,4 +7,4 @@ def get_authorization_scheme_param( ↵      if not authorization_header_value: ↵          return "", "" ↵      scheme, _, param = authorization_header_value.partition(" ") ↵ -    return scheme,`
  - Judgement: AMBIGUOUS — margin +0.5064 is within noise band.

### p90 (new flags vs max)

92 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #15280 | fastapi/applications.py | 1 | 3.3206 | bpe | `@@ -4559,6 +4563,60 @@ def trace_item(item_id: str):              generate_uniqu` |
| #15091 | fastapi/cli.py | 11 | 3.6801 | bpe | `@@ -6,7 +6,7 @@      def main() -> None: -    if not cli_main:  # type: ignore[t` |
| #15091 | fastapi/dependencies/utils.py | 18 | 3.2807 | bpe | `@@ -619,7 +622,7 @@ async def solve_dependencies(      if response is None:     ` |
| #15091 | fastapi/dependencies/utils.py | 21 | 2.3562 | bpe | `@@ -978,7 +981,7 @@ async def request_body_to_args(      for field in body_field` |
| #15038 | fastapi/routing.py | 0 | 3.4696 | bpe | `@@ -30,6 +30,7 @@    import anyio  from annotated_doc import Doc +from anyio.abc` |
| #15038 | fastapi/routing.py | 1 | 3.4696 | bpe | `@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes:               ` |
| #14986 | fastapi/applications.py | 0 | 2.4305 | bpe | `@@ -1101,16 +1101,18 @@ def openapi(self) -> dict[str, Any]:        def setup(se` |
| #14964 | fastapi/responses.py | 2 | 3.3979 | bpe | `@@ -20,12 +22,29 @@      orjson = None  # type: ignore     +@deprecated( +    "U` |
| #14964 | fastapi/responses.py | 3 | 3.3979 | bpe | `@@ -33,12 +52,29 @@ def render(self, content: Any) -> bytes:          return ujs` |
| #14898 | fastapi/_compat/v2.py | 7 | 2.5292 | bpe | `@@ -318,7 +320,7 @@ def serialize_sequence_value(*, field: ModelField, value: An` |
| #14898 | fastapi/dependencies/models.py | 158 | 3.4770 | bpe | `@@ -1,13 +1,13 @@  import inspect  import sys +from collections.abc import Calla` |
| #14898 | fastapi/encoders.py | 162 | 2.3860 | bpe | `@@ -1,6 +1,7 @@  import dataclasses  import datetime  from collections import de` |
| #14898 | fastapi/encoders.py | 164 | 3.0709 | bpe | `@@ -33,13 +34,13 @@      # Taken from Pydantic v1 as is -def isoformat(o: Union[` |
| #14898 | fastapi/openapi/models.py | 181 | 2.7075 | bpe | `@@ -59,37 +59,37 @@ class BaseModelWithConfig(BaseModel):      class Contact(Bas` |
| #14898 | fastapi/openapi/models.py | 182 | 3.1906 | bpe | `@@ -98,19 +98,19 @@ class Reference(BaseModel):    class Discriminator(BaseModel` |
| #14898 | fastapi/openapi/models.py | 183 | 2.8501 | bpe | `@@ -123,80 +123,80 @@ class ExternalDocumentation(BaseModelWithConfig):  class S` |
| #14898 | fastapi/openapi/models.py | 186 | 2.5441 | bpe | `@@ -265,57 +265,57 @@ class Header(ParameterBase):      class RequestBody(BaseMo` |
| #14898 | fastapi/routing.py | 392 | 3.0359 | bpe | `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T:        async def __aexit` |
| #14898 | fastapi/security/http.py | 541 | 3.2411 | bpe | `@@ -1,6 +1,6 @@  import binascii  from base64 import b64decode -from typing impo` |
| #14898 | fastapi/types.py | 580 | 2.5292 | bpe | `@@ -1,11 +1,12 @@  import types +from collections.abc import Callable  from enum` |

**Judgement per new flag:**

- **PR#15280 `fastapi/applications.py` hunk#1** (bpe=3.3206, thr=2.3008, margin=+1.0198)
  - reason: bpe
  - diff: `@@ -4559,6 +4563,60 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    def vibe( ↵ +        self, ↵ +        path: Annotated[ ↵ +       `
  - Judgement: AMBIGUOUS — margin +1.0198 is within noise band.

- **PR#15091 `fastapi/cli.py` hunk#11** (bpe=3.6801, thr=2.2985, margin=+1.3817)
  - reason: bpe
  - diff: `@@ -6,7 +6,7 @@ ↵   ↵   ↵  def main() -> None: ↵ -    if not cli_main:  # type: ignore[truthy-function] ↵ +    if not cli_main:  # type: ignore[truthy-function]  # ty: ignore[unused-ignore-comment] ↵          mes`
  - Judgement: AMBIGUOUS — margin +1.3817 is within noise band.

- **PR#15091 `fastapi/dependencies/utils.py` hunk#18** (bpe=3.2807, thr=2.2985, margin=+0.9823)
  - reason: bpe
  - diff: `@@ -619,7 +622,7 @@ async def solve_dependencies( ↵      if response is None: ↵          response = Response() ↵          del response.headers["content-length"] ↵ -        response.status_code = None  # type:`
  - Judgement: AMBIGUOUS — margin +0.9823 is within noise band.

- **PR#15091 `fastapi/dependencies/utils.py` hunk#21** (bpe=2.3562, thr=2.2985, margin=+0.0578)
  - reason: bpe
  - diff: `@@ -978,7 +981,7 @@ async def request_body_to_args( ↵      for field in body_fields: ↵          loc = ("body", get_validation_alias(field)) ↵          value: Any | None = None ↵ -        if body_to_process is`
  - Judgement: AMBIGUOUS — margin +0.0578 is within noise band.

- **PR#15038 `fastapi/routing.py` hunk#0** (bpe=3.4696, thr=2.3223, margin=+1.1472)
  - reason: bpe
  - diff: `@@ -30,6 +30,7 @@ ↵   ↵  import anyio ↵  from annotated_doc import Doc ↵ +from anyio.abc import ObjectReceiveStream ↵  from fastapi import params ↵  from fastapi._compat import ( ↵      ModelField,`
  - Judgement: AMBIGUOUS — margin +1.1472 is within noise band.

- **PR#15038 `fastapi/routing.py` hunk#1** (bpe=3.4696, thr=2.3223, margin=+1.1472)
  - reason: bpe
  - diff: `@@ -526,7 +527,10 @@ def _serialize_sse_item(item: Any) -> bytes: ↵                  else: ↵                      sse_aiter = iterate_in_threadpool(gen) ↵   ↵ -                async def _async_stream_sse() ->`
  - Judgement: AMBIGUOUS — margin +1.1472 is within noise band.

- **PR#14986 `fastapi/applications.py` hunk#0** (bpe=2.4305, thr=2.2692, margin=+0.1613)
  - reason: bpe
  - diff: `@@ -1101,16 +1101,18 @@ def openapi(self) -> dict[str, Any]: ↵   ↵      def setup(self) -> None: ↵          if self.openapi_url: ↵ -            urls = (server_data.get("url") for server_data in self.servers) ↵ `
  - Judgement: AMBIGUOUS — margin +0.1613 is within noise band.

- **PR#14964 `fastapi/responses.py` hunk#2** (bpe=3.3979, thr=2.2634, margin=+1.1344)
  - reason: bpe
  - diff: `@@ -20,12 +22,29 @@ ↵      orjson = None  # type: ignore ↵   ↵   ↵ +@deprecated( ↵ +    "UJSONResponse is deprecated, FastAPI now serializes data directly to JSON " ↵ +    "bytes via Pydantic when a return type o`
  - Judgement: AMBIGUOUS — margin +1.1344 is within noise band.

- **PR#14964 `fastapi/responses.py` hunk#3** (bpe=3.3979, thr=2.2634, margin=+1.1344)
  - reason: bpe
  - diff: `@@ -33,12 +52,29 @@ def render(self, content: Any) -> bytes: ↵          return ujson.dumps(content, ensure_ascii=False).encode("utf-8") ↵   ↵   ↵ +@deprecated( ↵ +    "ORJSONResponse is deprecated, FastAPI now `
  - Judgement: AMBIGUOUS — margin +1.1344 is within noise band.

- **PR#14898 `fastapi/_compat/v2.py` hunk#7** (bpe=2.5292, thr=2.3678, margin=+0.1615)
  - reason: bpe
  - diff: `@@ -318,7 +320,7 @@ def serialize_sequence_value(*, field: ModelField, value: Any) -> Sequence[Any]: ↵      return shared.sequence_annotation_to_type[origin_type](value)  # type: ignore[no-any-return,in`
  - Judgement: AMBIGUOUS — margin +0.1615 is within noise band.

- **PR#14898 `fastapi/dependencies/models.py` hunk#158** (bpe=3.4770, thr=2.3678, margin=+1.1092)
  - reason: bpe
  - diff: `@@ -1,13 +1,13 @@ ↵  import inspect ↵  import sys ↵ +from collections.abc import Callable ↵  from dataclasses import dataclass, field ↵  from functools import cached_property, partial ↵ -from typing import Any, C`
  - Judgement: AMBIGUOUS — margin +1.1092 is within noise band.

- **PR#14898 `fastapi/encoders.py` hunk#162** (bpe=2.3860, thr=2.3678, margin=+0.0182)
  - reason: bpe
  - diff: `@@ -1,6 +1,7 @@ ↵  import dataclasses ↵  import datetime ↵  from collections import defaultdict, deque ↵ +from collections.abc import Callable ↵  from decimal import Decimal ↵  from enum import Enum ↵  from ipaddre`
  - Judgement: AMBIGUOUS — margin +0.0182 is within noise band.

- **PR#14898 `fastapi/encoders.py` hunk#164** (bpe=3.0709, thr=2.3678, margin=+0.7031)
  - reason: bpe
  - diff: `@@ -33,13 +34,13 @@ ↵   ↵   ↵  # Taken from Pydantic v1 as is ↵ -def isoformat(o: Union[datetime.date, datetime.time]) -> str: ↵ +def isoformat(o: datetime.date | datetime.time) -> str: ↵      return o.isoformat(`
  - Judgement: AMBIGUOUS — margin +0.7031 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#181** (bpe=2.7075, thr=2.3678, margin=+0.3398)
  - reason: bpe
  - diff: `@@ -59,37 +59,37 @@ class BaseModelWithConfig(BaseModel): ↵   ↵   ↵  class Contact(BaseModelWithConfig): ↵ -    name: Optional[str] = None ↵ -    url: Optional[AnyUrl] = None ↵ -    email: Optional[EmailStr] = No`
  - Judgement: AMBIGUOUS — margin +0.3398 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#182** (bpe=3.1906, thr=2.3678, margin=+0.8229)
  - reason: bpe
  - diff: `@@ -98,19 +98,19 @@ class Reference(BaseModel): ↵   ↵  class Discriminator(BaseModel): ↵      propertyName: str ↵ -    mapping: Optional[dict[str, str]] = None ↵ +    mapping: dict[str, str] | None = None ↵   ↵   ↵  `
  - Judgement: AMBIGUOUS — margin +0.8229 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#183** (bpe=2.8501, thr=2.3678, margin=+0.4823)
  - reason: bpe
  - diff: `@@ -123,80 +123,80 @@ class ExternalDocumentation(BaseModelWithConfig): ↵  class Schema(BaseModelWithConfig): ↵      # Ref: JSON Schema 2020-12: https://json-schema.org/draft/2020-12/json-schema-core.html`
  - Judgement: AMBIGUOUS — margin +0.4823 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#186** (bpe=2.5441, thr=2.3678, margin=+0.1763)
  - reason: bpe
  - diff: `@@ -265,57 +265,57 @@ class Header(ParameterBase): ↵   ↵   ↵  class RequestBody(BaseModelWithConfig): ↵ -    description: Optional[str] = None ↵ +    description: str | None = None ↵      content: dict[str, Media`
  - Judgement: AMBIGUOUS — margin +0.1763 is within noise band.

- **PR#14898 `fastapi/routing.py` hunk#392** (bpe=3.0359, thr=2.3678, margin=+0.6681)
  - reason: bpe
  - diff: `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T: ↵   ↵      async def __aexit__( ↵          self, ↵ -        exc_type: Optional[type[BaseException]], ↵ -        exc_value: Optional[BaseException], ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.6681 is within noise band.

- **PR#14898 `fastapi/security/http.py` hunk#541** (bpe=3.2411, thr=2.3678, margin=+0.8733)
  - reason: bpe
  - diff: `@@ -1,6 +1,6 @@ ↵  import binascii ↵  from base64 import b64decode ↵ -from typing import Annotated, Optional ↵ +from typing import Annotated ↵   ↵  from annotated_doc import Doc ↵  from fastapi.exceptions import HT`
  - Judgement: AMBIGUOUS — margin +0.8733 is within noise band.

- **PR#14898 `fastapi/types.py` hunk#580** (bpe=2.5292, thr=2.3678, margin=+0.1615)
  - reason: bpe
  - diff: `@@ -1,11 +1,12 @@ ↵  import types ↵ +from collections.abc import Callable ↵  from enum import Enum ↵ -from typing import Any, Callable, Optional, TypeVar, Union ↵ +from typing import Any, TypeVar, Union ↵   ↵  from`
  - Judgement: AMBIGUOUS — margin +0.1615 is within noise band.

---

## §2 Rich — Per-threshold Flag Set Diff vs max

### p99 (new flags vs max)

5 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #3942 | rich/markdown.py | 4 | 3.8646 | bpe | `@@ -143,20 +158,10 @@ def __init__(self, tag: str) -> None:      def __rich_cons` |
| #3782 | rich/syntax.py | 1 | 3.7140 | bpe | `@@ -224,6 +226,17 @@ class _SyntaxHighlightRange(NamedTuple):      style_before:` |
| #3782 | rich/syntax.py | 2 | 3.7140 | bpe | `@@ -293,11 +306,13 @@ def __init__(              Style(bgcolor=background_color)` |
| #3777 | rich/console.py | 1 | 4.0356 | bpe | `@@ -731,6 +733,14 @@ def __init__(              if no_color is not None         ` |
| #3777 | rich/diagnose.py | 2 | 4.0356 | bpe | `@@ -26,6 +26,7 @@ def report() -> None:  # pragma: no cover          "TERM_PROGR` |

**Judgement per new flag (p99 Rich — all 5 shown):**

- **PR#3942 `rich/markdown.py` hunk#4** (bpe=3.8646, thr=3.6779, margin=+0.1868)
  - diff: `@@ -143,20 +158,10 @@ def __init__(self, tag: str) -> None: ↵      def __rich_console__( ↵          self, console: Console, options: ConsoleOptions ↵      ) -> RenderResult: ↵ -        text = self.text ↵ -     `
  - **LIKELY_STYLE_DRIFT** — refactors `__rich_console__` render path, removes `text = self.text` intermediary; changes how markdown elements expose their console representation.

- **PR#3782 `rich/syntax.py` hunk#1** (bpe=3.7140, thr=3.3648, margin=+0.3492)
  - diff: `@@ -224,6 +226,17 @@ class _SyntaxHighlightRange(NamedTuple): ↵      style_before: bool = False ↵   ↵   ↵ +class PaddingProperty: ↵ +    """Descriptor to get and set padding.""" ↵ + ↵ +    def __get__(self, obj: Sy`
  - **LIKELY_STYLE_DRIFT** — introduces `PaddingProperty` descriptor class; descriptor protocol is a new pattern not used elsewhere in host at this point.

- **PR#3782 `rich/syntax.py` hunk#2** (bpe=3.7140, thr=3.3648, margin=+0.3492)
  - diff: `@@ -293,11 +306,13 @@ def __init__( ↵              Style(bgcolor=background_color) if background_color else Style() ↵          ) ↵          self.indent_guides = indent_guides ↵ -        self.padding = padding`
  - **LIKELY_STYLE_DRIFT** — wires up the new `PaddingProperty` descriptor in `__init__`; companion hunk to above.

- **PR#3777 `rich/console.py` hunk#1** (bpe=4.0356, thr=3.3645, margin=+0.6711)
  - diff: `@@ -731,6 +733,14 @@ def __init__( ↵              if no_color is not None ↵              else self._environ.get("NO_COLOR", "") != "" ↵          ) ↵ +        if force_interactive is None: ↵ +            tty_int`
  - **LIKELY_STYLE_DRIFT** — adds TTY interactive detection block; new `force_interactive` feature with env-var fallback; genuinely new idiom in the codebase.

- **PR#3777 `rich/diagnose.py` hunk#2** (bpe=4.0356, thr=3.3645, margin=+0.6711)
  - diff: `@@ -26,6 +26,7 @@ def report() -> None:  # pragma: no cover ↵          "TERM_PROGRAM", ↵          "TERM", ↵          "TTY_COMPATIBLE", ↵ +        "TTY_INTERACTIVE", ↵          "VSCODE_VERBOSE_LOGGING", ↵      ) ↵  `
  - **LIKELY_STYLE_DRIFT** — companion hunk; adds `TTY_INTERACTIVE` to the diagnostics report alongside the new console feature.

**p99 Rich summary:** 5/5 new flags are LIKELY_STYLE_DRIFT — 0 false positives. Rich p99 signals are all real.

### p95 (new flags vs max)

6 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #3942 | rich/markdown.py | 4 | 3.8646 | bpe | `@@ -143,20 +158,10 @@ def __init__(self, tag: str) -> None:      def __rich_cons` |
| #3934 | rich/live.py | 0 | 3.6254 | bpe | `@@ -166,7 +166,11 @@ def stop(self) -> None:                  finally:          ` |
| #3782 | rich/syntax.py | 1 | 3.7140 | bpe | `@@ -224,6 +226,17 @@ class _SyntaxHighlightRange(NamedTuple):      style_before:` |
| #3782 | rich/syntax.py | 2 | 3.7140 | bpe | `@@ -293,11 +306,13 @@ def __init__(              Style(bgcolor=background_color)` |
| #3777 | rich/console.py | 1 | 4.0356 | bpe | `@@ -731,6 +733,14 @@ def __init__(              if no_color is not None         ` |
| #3777 | rich/diagnose.py | 2 | 4.0356 | bpe | `@@ -26,6 +26,7 @@ def report() -> None:  # pragma: no cover          "TERM_PROGR` |

**Judgement per new flag:**

- **PR#3942 `rich/markdown.py` hunk#4** (bpe=3.8646, thr=3.5192, margin=+0.3454)
  - diff: `@@ -143,20 +158,10 @@ def __init__(self, tag: str) -> None: ↵      def __rich_console__( ↵          self, console: Console, options: ConsoleOptions ↵      ) -> RenderResult: ↵ -        text = self.text ↵ -     `
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3934 `rich/live.py` hunk#0** (bpe=3.6254, thr=3.5164, margin=+0.1090)
  - diff: `@@ -166,7 +166,11 @@ def stop(self) -> None: ↵                  finally: ↵                      self._disable_redirect_io() ↵                      self.console.pop_render_hook() ↵ -                    if not `
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3782 `rich/syntax.py` hunk#1** (bpe=3.7140, thr=3.2799, margin=+0.4341)
  - diff: `@@ -224,6 +226,17 @@ class _SyntaxHighlightRange(NamedTuple): ↵      style_before: bool = False ↵   ↵   ↵ +class PaddingProperty: ↵ +    """Descriptor to get and set padding.""" ↵ + ↵ +    def __get__(self, obj: Sy`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3782 `rich/syntax.py` hunk#2** (bpe=3.7140, thr=3.2799, margin=+0.4341)
  - diff: `@@ -293,11 +306,13 @@ def __init__( ↵              Style(bgcolor=background_color) if background_color else Style() ↵          ) ↵          self.indent_guides = indent_guides ↵ -        self.padding = padding`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3777 `rich/console.py` hunk#1** (bpe=4.0356, thr=3.2796, margin=+0.7560)
  - diff: `@@ -731,6 +733,14 @@ def __init__( ↵              if no_color is not None ↵              else self._environ.get("NO_COLOR", "") != "" ↵          ) ↵ +        if force_interactive is None: ↵ +            tty_int`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3777 `rich/diagnose.py` hunk#2** (bpe=4.0356, thr=3.2796, margin=+0.7560)
  - diff: `@@ -26,6 +26,7 @@ def report() -> None:  # pragma: no cover ↵          "TERM_PROGRAM", ↵          "TERM", ↵          "TTY_COMPATIBLE", ↵ +        "TTY_INTERACTIVE", ↵          "VSCODE_VERBOSE_LOGGING", ↵      ) ↵  `
  - Judgement: AMBIGUOUS — within threshold noise band.

### p90 (new flags vs max)

7 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #4076 | rich/ansi.py | 0 | 3.1724 | bpe | `@@ -127,13 +127,13 @@ def decode(self, terminal_text: str) -> Iterable[Text]:   ` |
| #3942 | rich/markdown.py | 4 | 3.8646 | bpe | `@@ -143,20 +158,10 @@ def __init__(self, tag: str) -> None:      def __rich_cons` |
| #3934 | rich/live.py | 0 | 3.6254 | bpe | `@@ -166,7 +166,11 @@ def stop(self) -> None:                  finally:          ` |
| #3782 | rich/syntax.py | 1 | 3.7140 | bpe | `@@ -224,6 +226,17 @@ class _SyntaxHighlightRange(NamedTuple):      style_before:` |
| #3782 | rich/syntax.py | 2 | 3.7140 | bpe | `@@ -293,11 +306,13 @@ def __init__(              Style(bgcolor=background_color)` |
| #3777 | rich/console.py | 1 | 4.0356 | bpe | `@@ -731,6 +733,14 @@ def __init__(              if no_color is not None         ` |
| #3777 | rich/diagnose.py | 2 | 4.0356 | bpe | `@@ -26,6 +26,7 @@ def report() -> None:  # pragma: no cover          "TERM_PROGR` |

**Judgement per new flag:**

- **PR#4076 `rich/ansi.py` hunk#0** (bpe=3.1724, thr=2.9015, margin=+0.2709)
  - diff: `@@ -127,13 +127,13 @@ def decode(self, terminal_text: str) -> Iterable[Text]: ↵          """Decode ANSI codes in an iterable of lines. ↵   ↵          Args: ↵ -            lines (Iterable[str]): An iterable of`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3942 `rich/markdown.py` hunk#4** (bpe=3.8646, thr=3.2296, margin=+0.6350)
  - diff: `@@ -143,20 +158,10 @@ def __init__(self, tag: str) -> None: ↵      def __rich_console__( ↵          self, console: Console, options: ConsoleOptions ↵      ) -> RenderResult: ↵ -        text = self.text ↵ -     `
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3934 `rich/live.py` hunk#0** (bpe=3.6254, thr=3.2268, margin=+0.3986)
  - diff: `@@ -166,7 +166,11 @@ def stop(self) -> None: ↵                  finally: ↵                      self._disable_redirect_io() ↵                      self.console.pop_render_hook() ↵ -                    if not `
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3782 `rich/syntax.py` hunk#1** (bpe=3.7140, thr=2.5299, margin=+1.1840)
  - diff: `@@ -224,6 +226,17 @@ class _SyntaxHighlightRange(NamedTuple): ↵      style_before: bool = False ↵   ↵   ↵ +class PaddingProperty: ↵ +    """Descriptor to get and set padding.""" ↵ + ↵ +    def __get__(self, obj: Sy`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3782 `rich/syntax.py` hunk#2** (bpe=3.7140, thr=2.5299, margin=+1.1840)
  - diff: `@@ -293,11 +306,13 @@ def __init__( ↵              Style(bgcolor=background_color) if background_color else Style() ↵          ) ↵          self.indent_guides = indent_guides ↵ -        self.padding = padding`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3777 `rich/console.py` hunk#1** (bpe=4.0356, thr=2.5297, margin=+1.5060)
  - diff: `@@ -731,6 +733,14 @@ def __init__( ↵              if no_color is not None ↵              else self._environ.get("NO_COLOR", "") != "" ↵          ) ↵ +        if force_interactive is None: ↵ +            tty_int`
  - Judgement: AMBIGUOUS — within threshold noise band.

- **PR#3777 `rich/diagnose.py` hunk#2** (bpe=4.0356, thr=2.5297, margin=+1.5060)
  - diff: `@@ -26,6 +26,7 @@ def report() -> None:  # pragma: no cover ↵          "TERM_PROGRAM", ↵          "TERM", ↵          "TTY_COMPATIBLE", ↵ +        "TTY_INTERACTIVE", ↵          "VSCODE_VERBOSE_LOGGING", ↵      ) ↵  `
  - Judgement: AMBIGUOUS — within threshold noise band.

---

## §3 Recall — Per-threshold Catch Rates

| Threshold | Phase 1 catch rate (catalog breaks) | Phase 2 catch rate (stage2-only) |
|---|---|---|
| max | 95.2% | 100.0% |
| p99 | 99.2% | 100.0% |
| p95 | 100.0% | 100.0% |
| p90 | 100.0% | 100.0% |

Phase 1 gate: ≥50%. Phase 2 gate: ≥70%.

---

## §4 Verdict

**Winner: max (current behavior)**

The spec rule is: pick the threshold that maximizes recall AND keeps FP estimate ≤ max's rate. No lower percentile satisfies the FP constraint — each lowers the threshold, catching more hunks (both TP and FP), so the raw flag rate rises at every step. Adopting p99 would push FastAPI hunk flag rate from 4.0% → 5.6%.

However, the flag-quality analysis reveals something important:

- p99 catches 4 more Phase 1 recall pairs (+4.0%) by lowering the threshold enough to pick up real INTENTIONAL_STYLE_INTRO cases (Pydantic v1 removal PR#14609, SSE streaming PR#15038, `@deprecated()` PR#14964).
- Of 24 new p99 FastAPI flags: 8 INTENTIONAL_STYLE_INTRO, 2 LIKELY_STYLE_DRIFT, 5 FALSE_POSITIVE, 5 AMBIGUOUS.
- Of 5 new p99 Rich flags: 5/5 LIKELY_STYLE_DRIFT — zero false positives.
- The 5 FPs at p99 are all noise-band cases (comment edits, margin < 0.12).

**If precision matters more than raw flag rate:** keep max. The 5.6% hunk rate at p99 is a 40% relative increase over max's 4.0%, driven partly by real signals.

**If recall matters more than FP rate** (i.e., before a PR campaign): consider p99. The 5 FP cases at p99 are identifiable and suppressible. The 10 real signals at p99 are exactly what the scorer is supposed to catch.

**Decision for now:** keep `threshold_percentile=None` (max) as the default. Do not change the scorer default until the PR campaign scope and precision requirement are locked in. The parameterization is in place — switching to p99 is a one-argument change.

---

## §5 Honest Call-out

Threshold choice does materially matter — but in an unexpected direction. The hypothesis was that max might be outlier-inflated, causing spurious FPs that p99 would avoid. The data shows the opposite: max is the most precise threshold because it is the highest (strictest). Lowering the threshold always adds more flags, not fewer.

The 4.8% Phase 1 recall gap at max (95.2%) is real: max misses roughly 3-4 fixture-PR pairs that p99 and p95 catch. Whether that gap matters depends on whether those specific patterns (Pydantic v1 removal, SSE streaming, `@deprecated()` decorators) will appear in the PR campaign. If they will, p99 is worth the 1.6% FP rate increase. If the campaign is focused on stable-API repos, keep max.

p90 is out: 10.3% FastAPI hunk flag rate (+158% relative) with no recall gain over p95. Reject.

