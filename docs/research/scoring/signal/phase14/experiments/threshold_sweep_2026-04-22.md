# Phase 14 Prompt Q — Threshold Sweep

**Date:** 2026-04-22  
**Branch:** research/phase-14-import-graph  
**Why:** max(cal_scores) is outlier-sensitive and has no statistical guarantee.
Test whether p95 or p99 improves FP rate while preserving recall.

---

## §0 Comparison Table

| Threshold | FastAPI PR flag% | FastAPI hunk flag% | FastAPI FP est | Rich PR flag% | Rich hunk flag% | Phase 1 recall | Phase 2 recall |
|---|---|---|---|---|---|---|---|
| max | 31.4% | 1.5% | ≈hunk rate | 0.0% | 0.0% | 93.5% | 100.0% |
| p99 | 37.1% | 2.3% | ≈hunk rate | 0.0% | 0.0% | 96.8% | 100.0% |
| p95 | 48.6% | 4.4% | ≈hunk rate | 0.0% | 0.0% | 100.0% | 100.0% |
| p90 | 57.1% | 7.1% | ≈hunk rate | 0.0% | 0.0% | 100.0% | 100.0% |

FastAPI PRs are assumed clean (merged production code): hunk flag rate ≈ FP rate.
Rich flags at p90 include known auto-generated migration hunks.

---

## §1 FastAPI — Per-threshold Flag Set Diff vs max

### p99 (new flags vs max)

10 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #14898 | fastapi/datastructures.py | 157 | 4.1327 | bpe | `@@ -58,11 +56,11 @@ async def create_upload_file(file: UploadFile):          Bin` |
| #14609 | fastapi/encoders.py | 37 | 3.7140 | bpe | `@@ -18,14 +18,18 @@  from uuid import UUID    from annotated_doc import Doc -fro` |
| #14609 | fastapi/encoders.py | 43 | 3.7140 | bpe | `@@ -331,7 +318,11 @@ def jsonable_encoder(      for encoder, classes_tuple in en` |
| #14609 | fastapi/exceptions.py | 44 | 3.7140 | bpe | `@@ -233,6 +233,12 @@ def __init__(          self.body = body     +class Pydantic` |
| #14609 | fastapi/routing.py | 72 | 3.7140 | bpe | `@@ -47,8 +42,8 @@  from fastapi.encoders import jsonable_encoder  from fastapi.e` |
| #14609 | fastapi/routing.py | 80 | 3.7140 | bpe | `@@ -638,11 +566,9 @@ def __init__(              )              response_name = "` |
| #14609 | fastapi/routing.py | 81 | 3.7140 | bpe | `@@ -678,11 +604,9 @@ def __init__(                  )                  response_` |
| #14609 | fastapi/utils.py | 83 | 3.7140 | bpe | `@@ -19,11 +18,9 @@      UndefinedType,      Validator,      annotation_is_pydant` |
| #14609 | fastapi/utils.py | 84 | 3.7140 | bpe | `@@ -83,52 +80,18 @@ def create_model_field(      mode: Literal["validation", "se` |
| #14371 | fastapi/_compat/v2.py | 0 | 3.5992 | bpe | `@@ -110,6 +110,18 @@ def alias(self) -> str:          a = self.field_info.alias ` |

**Judgement per new flag:**

- **PR#14898 `fastapi/datastructures.py` hunk#157** (bpe=4.1327, thr=3.7783, margin=+0.3545)
  - reason: bpe
  - diff: `@@ -58,11 +56,11 @@ async def create_upload_file(file: UploadFile): ↵          BinaryIO, ↵          Doc("The standard Python file object (non-async)."), ↵      ] ↵ -    filename: Annotated[Optional[str], Doc(`
  - Judgement: AMBIGUOUS — margin +0.3545 is within noise band.

- **PR#14609 `fastapi/encoders.py` hunk#37** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -18,14 +18,18 @@ ↵  from uuid import UUID ↵   ↵  from annotated_doc import Doc ↵ -from fastapi._compat import may_v1 ↵ +from fastapi.exceptions import PydanticV1NotSupportedError ↵  from fastapi.types import I`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/encoders.py` hunk#43** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -331,7 +318,11 @@ def jsonable_encoder( ↵      for encoder, classes_tuple in encoders_by_class_tuples.items(): ↵          if isinstance(obj, classes_tuple): ↵              return encoder(obj) ↵ - ↵ +    if i`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/exceptions.py` hunk#44** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -233,6 +233,12 @@ def __init__( ↵          self.body = body ↵   ↵   ↵ +class PydanticV1NotSupportedError(FastAPIError): ↵ +    """ ↵ +    A pydantic.v1 model is used, which is no longer supported. ↵ +    """ ↵ + ↵ +`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/routing.py` hunk#72** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -47,8 +42,8 @@ ↵  from fastapi.encoders import jsonable_encoder ↵  from fastapi.exceptions import ( ↵      EndpointContext, ↵ -    FastAPIDeprecationWarning, ↵      FastAPIError, ↵ +    PydanticV1NotSupportedE`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/routing.py` hunk#80** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -638,11 +566,9 @@ def __init__( ↵              ) ↵              response_name = "Response_" + self.unique_id ↵              if annotation_is_pydantic_v1(self.response_model): ↵ -                warnings.wa`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/routing.py` hunk#81** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -678,11 +604,9 @@ def __init__( ↵                  ) ↵                  response_name = f"Response_{additional_status_code}_{self.unique_id}" ↵                  if annotation_is_pydantic_v1(model): ↵ -    `
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/utils.py` hunk#83** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -19,11 +18,9 @@ ↵      UndefinedType, ↵      Validator, ↵      annotation_is_pydantic_v1, ↵ -    lenient_issubclass, ↵ -    may_v1, ↵  ) ↵  from fastapi.datastructures import DefaultPlaceholder, DefaultType ↵ -fro`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14609 `fastapi/utils.py` hunk#84** (bpe=3.7140, thr=3.6171, margin=+0.0969)
  - reason: bpe
  - diff: `@@ -83,52 +80,18 @@ def create_model_field( ↵      mode: Literal["validation", "serialization"] = "validation", ↵      version: Literal["1", "auto"] = "auto", ↵  ) -> ModelField: ↵ -    class_validators = cla`
  - Judgement: AMBIGUOUS — margin +0.0969 is within noise band.

- **PR#14371 `fastapi/_compat/v2.py` hunk#0** (bpe=3.5992, thr=3.5706, margin=+0.0286)
  - reason: bpe
  - diff: `@@ -110,6 +110,18 @@ def alias(self) -> str: ↵          a = self.field_info.alias ↵          return a if a is not None else self.name ↵   ↵ +    @property ↵ +    def validation_alias(self) -> Union[str, None]: ↵ `
  - Judgement: AMBIGUOUS — margin +0.0286 is within noise band.

### p95 (new flags vs max)

38 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #14898 | fastapi/datastructures.py | 157 | 4.1327 | bpe | `@@ -58,11 +56,11 @@ async def create_upload_file(file: UploadFile):          Bin` |
| #14898 | fastapi/dependencies/models.py | 158 | 3.4770 | bpe | `@@ -1,13 +1,13 @@  import inspect  import sys +from collections.abc import Calla` |
| #14898 | fastapi/encoders.py | 164 | 3.0709 | bpe | `@@ -33,13 +34,13 @@      # Taken from Pydantic v1 as is -def isoformat(o: Union[` |
| #14898 | fastapi/openapi/models.py | 182 | 3.1906 | bpe | `@@ -98,19 +98,19 @@ class Reference(BaseModel):    class Discriminator(BaseModel` |
| #14898 | fastapi/routing.py | 392 | 3.0359 | bpe | `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T:        async def __aexit` |
| #14898 | fastapi/security/http.py | 541 | 3.2411 | bpe | `@@ -1,6 +1,6 @@  import binascii  from base64 import b64decode -from typing impo` |
| #14897 | fastapi/dependencies/utils.py | 7 | 2.9206 | bpe | `@@ -1,18 +1,19 @@  import dataclasses  import inspect  import sys -from collecti` |
| #14897 | fastapi/dependencies/utils.py | 10 | 2.9206 | bpe | `@@ -199,20 +199,17 @@ def get_flat_params(dependant: Dependant) -> list[ModelFie` |
| #14884 | fastapi/dependencies/utils.py | 0 | 2.9202 | bpe | `@@ -1,7 +1,7 @@  import dataclasses  import inspect  import sys -from collection` |
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

**Judgement per new flag:**

- **PR#14898 `fastapi/datastructures.py` hunk#157** (bpe=4.1327, thr=2.8518, margin=+1.2809)
  - reason: bpe
  - diff: `@@ -58,11 +56,11 @@ async def create_upload_file(file: UploadFile): ↵          BinaryIO, ↵          Doc("The standard Python file object (non-async)."), ↵      ] ↵ -    filename: Annotated[Optional[str], Doc(`
  - Judgement: AMBIGUOUS — margin +1.2809 is within noise band.

- **PR#14898 `fastapi/dependencies/models.py` hunk#158** (bpe=3.4770, thr=2.8518, margin=+0.6251)
  - reason: bpe
  - diff: `@@ -1,13 +1,13 @@ ↵  import inspect ↵  import sys ↵ +from collections.abc import Callable ↵  from dataclasses import dataclass, field ↵  from functools import cached_property, partial ↵ -from typing import Any, C`
  - Judgement: AMBIGUOUS — margin +0.6251 is within noise band.

- **PR#14898 `fastapi/encoders.py` hunk#164** (bpe=3.0709, thr=2.8518, margin=+0.2191)
  - reason: bpe
  - diff: `@@ -33,13 +34,13 @@ ↵   ↵   ↵  # Taken from Pydantic v1 as is ↵ -def isoformat(o: Union[datetime.date, datetime.time]) -> str: ↵ +def isoformat(o: datetime.date | datetime.time) -> str: ↵      return o.isoformat(`
  - Judgement: AMBIGUOUS — margin +0.2191 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#182** (bpe=3.1906, thr=2.8518, margin=+0.3388)
  - reason: bpe
  - diff: `@@ -98,19 +98,19 @@ class Reference(BaseModel): ↵   ↵  class Discriminator(BaseModel): ↵      propertyName: str ↵ -    mapping: Optional[dict[str, str]] = None ↵ +    mapping: dict[str, str] | None = None ↵   ↵   ↵  `
  - Judgement: AMBIGUOUS — margin +0.3388 is within noise band.

- **PR#14898 `fastapi/routing.py` hunk#392** (bpe=3.0359, thr=2.8518, margin=+0.1840)
  - reason: bpe
  - diff: `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T: ↵   ↵      async def __aexit__( ↵          self, ↵ -        exc_type: Optional[type[BaseException]], ↵ -        exc_value: Optional[BaseException], ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.1840 is within noise band.

- **PR#14898 `fastapi/security/http.py` hunk#541** (bpe=3.2411, thr=2.8518, margin=+0.3892)
  - reason: bpe
  - diff: `@@ -1,6 +1,6 @@ ↵  import binascii ↵  from base64 import b64decode ↵ -from typing import Annotated, Optional ↵ +from typing import Annotated ↵   ↵  from annotated_doc import Doc ↵  from fastapi.exceptions import HT`
  - Judgement: AMBIGUOUS — margin +0.3892 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#7** (bpe=2.9206, thr=2.8523, margin=+0.0683)
  - reason: bpe
  - diff: `@@ -1,18 +1,19 @@ ↵  import dataclasses ↵  import inspect ↵  import sys ↵ -from collections.abc import Mapping, Sequence ↵ +from collections.abc import Callable, Mapping, Sequence ↵  from contextlib import AsyncE`
  - Judgement: AMBIGUOUS — margin +0.0683 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#10** (bpe=2.9206, thr=2.8523, margin=+0.0683)
  - reason: bpe
  - diff: `@@ -199,20 +199,17 @@ def get_flat_params(dependant: Dependant) -> list[ModelField]: ↵   ↵   ↵  def _get_signature(call: Callable[..., Any]) -> inspect.Signature: ↵ -    if sys.version_info >= (3, 10): ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.0683 is within noise band.

- **PR#14884 `fastapi/dependencies/utils.py` hunk#0** (bpe=2.9202, thr=2.7992, margin=+0.1210)
  - reason: bpe
  - diff: `@@ -1,7 +1,7 @@ ↵  import dataclasses ↵  import inspect ↵  import sys ↵ -from collections.abc import Coroutine, Mapping, Sequence ↵ +from collections.abc import Mapping, Sequence ↵  from contextlib import AsyncEx`
  - Judgement: AMBIGUOUS — margin +0.1210 is within noise band.

- **PR#14851 `fastapi/routing.py` hunk#4** (bpe=2.7627, thr=2.7541, margin=+0.0087)
  - reason: bpe
  - diff: `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    # TODO: remove this once the lifespan (or alternative) inte`
  - Judgement: AMBIGUOUS — margin +0.0087 is within noise band.

- **PR#14786 `fastapi/security/utils.py` hunk#0** (bpe=3.2306, thr=2.8000, margin=+0.4306)
  - reason: bpe
  - diff: `@@ -7,4 +7,4 @@ def get_authorization_scheme_param( ↵      if not authorization_header_value: ↵          return "", "" ↵      scheme, _, param = authorization_header_value.partition(" ") ↵ -    return scheme,`
  - Judgement: AMBIGUOUS — margin +0.4306 is within noise band.

- **PR#14814 `fastapi/_compat/shared.py` hunk#0** (bpe=3.0646, thr=2.7903, margin=+0.2743)
  - reason: bpe
  - diff: `@@ -17,7 +17,7 @@ ↵  from starlette.datastructures import UploadFile ↵  from typing_extensions import get_args, get_origin ↵   ↵ -# Copy from Pydantic v2, compatible with v1 ↵ +# Copy from Pydantic: pydantic/_i`
  - Judgement: AMBIGUOUS — margin +0.2743 is within noise band.

- **PR#14609 `fastapi/encoders.py` hunk#37** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -18,14 +18,18 @@ ↵  from uuid import UUID ↵   ↵  from annotated_doc import Doc ↵ -from fastapi._compat import may_v1 ↵ +from fastapi.exceptions import PydanticV1NotSupportedError ↵  from fastapi.types import I`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/encoders.py` hunk#43** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -331,7 +318,11 @@ def jsonable_encoder( ↵      for encoder, classes_tuple in encoders_by_class_tuples.items(): ↵          if isinstance(obj, classes_tuple): ↵              return encoder(obj) ↵ - ↵ +    if i`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/exceptions.py` hunk#44** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -233,6 +233,12 @@ def __init__( ↵          self.body = body ↵   ↵   ↵ +class PydanticV1NotSupportedError(FastAPIError): ↵ +    """ ↵ +    A pydantic.v1 model is used, which is no longer supported. ↵ +    """ ↵ + ↵ +`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/routing.py` hunk#72** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -47,8 +42,8 @@ ↵  from fastapi.encoders import jsonable_encoder ↵  from fastapi.exceptions import ( ↵      EndpointContext, ↵ -    FastAPIDeprecationWarning, ↵      FastAPIError, ↵ +    PydanticV1NotSupportedE`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/routing.py` hunk#80** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -638,11 +566,9 @@ def __init__( ↵              ) ↵              response_name = "Response_" + self.unique_id ↵              if annotation_is_pydantic_v1(self.response_model): ↵ -                warnings.wa`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/routing.py` hunk#81** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -678,11 +604,9 @@ def __init__( ↵                  ) ↵                  response_name = f"Response_{additional_status_code}_{self.unique_id}" ↵                  if annotation_is_pydantic_v1(model): ↵ -    `
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/utils.py` hunk#83** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -19,11 +18,9 @@ ↵      UndefinedType, ↵      Validator, ↵      annotation_is_pydantic_v1, ↵ -    lenient_issubclass, ↵ -    may_v1, ↵  ) ↵  from fastapi.datastructures import DefaultPlaceholder, DefaultType ↵ -fro`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

- **PR#14609 `fastapi/utils.py` hunk#84** (bpe=3.7140, thr=2.8018, margin=+0.9122)
  - reason: bpe
  - diff: `@@ -83,52 +80,18 @@ def create_model_field( ↵      mode: Literal["validation", "serialization"] = "validation", ↵      version: Literal["1", "auto"] = "auto", ↵  ) -> ModelField: ↵ -    class_validators = cla`
  - Judgement: AMBIGUOUS — margin +0.9122 is within noise band.

### p90 (new flags vs max)

73 hunk(s) newly flagged:

| PR# | file | hunk_idx | bpe_score | reason | diff preview |
|---|---|---|---|---|---|
| #14898 | fastapi/_compat/v2.py | 7 | 2.5292 | bpe | `@@ -318,7 +320,7 @@ def serialize_sequence_value(*, field: ModelField, value: An` |
| #14898 | fastapi/datastructures.py | 157 | 4.1327 | bpe | `@@ -58,11 +56,11 @@ async def create_upload_file(file: UploadFile):          Bin` |
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
| #14898 | fastapi/utils.py | 581 | 2.7338 | bpe | `@@ -3,8 +3,7 @@  from typing import (      TYPE_CHECKING,      Any, -    Optiona` |
| #14897 | fastapi/dependencies/utils.py | 7 | 2.9206 | bpe | `@@ -1,18 +1,19 @@  import dataclasses  import inspect  import sys -from collecti` |
| #14897 | fastapi/dependencies/utils.py | 10 | 2.9206 | bpe | `@@ -199,20 +199,17 @@ def get_flat_params(dependant: Dependant) -> list[ModelFie` |
| #14897 | fastapi/dependencies/utils.py | 20 | 2.4760 | bpe | `@@ -950,7 +947,7 @@ async def request_body_to_args(          return {first_field` |
| #14884 | fastapi/dependencies/utils.py | 0 | 2.9202 | bpe | `@@ -1,7 +1,7 @@  import dataclasses  import inspect  import sys -from collection` |
| #14860 | fastapi/dependencies/utils.py | 13 | 2.4438 | bpe | `@@ -1021,7 +1016,6 @@ def get_body_field(      final_field = create_model_field(` |
| #14856 | fastapi/utils.py | 6 | 2.7406 | bpe | `@@ -1,13 +1,11 @@  import re  import warnings -from collections.abc import Mutab` |
| #14851 | fastapi/routing.py | 4 | 2.7627 | bpe | `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str):              generate_uniqu` |

**Judgement per new flag:**

- **PR#14898 `fastapi/_compat/v2.py` hunk#7** (bpe=2.5292, thr=2.3689, margin=+0.1603)
  - reason: bpe
  - diff: `@@ -318,7 +320,7 @@ def serialize_sequence_value(*, field: ModelField, value: Any) -> Sequence[Any]: ↵      return shared.sequence_annotation_to_type[origin_type](value)  # type: ignore[no-any-return,in`
  - Judgement: AMBIGUOUS — margin +0.1603 is within noise band.

- **PR#14898 `fastapi/datastructures.py` hunk#157** (bpe=4.1327, thr=2.3689, margin=+1.7638)
  - reason: bpe
  - diff: `@@ -58,11 +56,11 @@ async def create_upload_file(file: UploadFile): ↵          BinaryIO, ↵          Doc("The standard Python file object (non-async)."), ↵      ] ↵ -    filename: Annotated[Optional[str], Doc(`
  - Judgement: AMBIGUOUS — margin +1.7638 is within noise band.

- **PR#14898 `fastapi/dependencies/models.py` hunk#158** (bpe=3.4770, thr=2.3689, margin=+1.1081)
  - reason: bpe
  - diff: `@@ -1,13 +1,13 @@ ↵  import inspect ↵  import sys ↵ +from collections.abc import Callable ↵  from dataclasses import dataclass, field ↵  from functools import cached_property, partial ↵ -from typing import Any, C`
  - Judgement: AMBIGUOUS — margin +1.1081 is within noise band.

- **PR#14898 `fastapi/encoders.py` hunk#162** (bpe=2.3860, thr=2.3689, margin=+0.0170)
  - reason: bpe
  - diff: `@@ -1,6 +1,7 @@ ↵  import dataclasses ↵  import datetime ↵  from collections import defaultdict, deque ↵ +from collections.abc import Callable ↵  from decimal import Decimal ↵  from enum import Enum ↵  from ipaddre`
  - Judgement: AMBIGUOUS — margin +0.0170 is within noise band.

- **PR#14898 `fastapi/encoders.py` hunk#164** (bpe=3.0709, thr=2.3689, margin=+0.7020)
  - reason: bpe
  - diff: `@@ -33,13 +34,13 @@ ↵   ↵   ↵  # Taken from Pydantic v1 as is ↵ -def isoformat(o: Union[datetime.date, datetime.time]) -> str: ↵ +def isoformat(o: datetime.date | datetime.time) -> str: ↵      return o.isoformat(`
  - Judgement: AMBIGUOUS — margin +0.7020 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#181** (bpe=2.7075, thr=2.3689, margin=+0.3386)
  - reason: bpe
  - diff: `@@ -59,37 +59,37 @@ class BaseModelWithConfig(BaseModel): ↵   ↵   ↵  class Contact(BaseModelWithConfig): ↵ -    name: Optional[str] = None ↵ -    url: Optional[AnyUrl] = None ↵ -    email: Optional[EmailStr] = No`
  - Judgement: AMBIGUOUS — margin +0.3386 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#182** (bpe=3.1906, thr=2.3689, margin=+0.8217)
  - reason: bpe
  - diff: `@@ -98,19 +98,19 @@ class Reference(BaseModel): ↵   ↵  class Discriminator(BaseModel): ↵      propertyName: str ↵ -    mapping: Optional[dict[str, str]] = None ↵ +    mapping: dict[str, str] | None = None ↵   ↵   ↵  `
  - Judgement: AMBIGUOUS — margin +0.8217 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#183** (bpe=2.8501, thr=2.3689, margin=+0.4812)
  - reason: bpe
  - diff: `@@ -123,80 +123,80 @@ class ExternalDocumentation(BaseModelWithConfig): ↵  class Schema(BaseModelWithConfig): ↵      # Ref: JSON Schema 2020-12: https://json-schema.org/draft/2020-12/json-schema-core.html`
  - Judgement: AMBIGUOUS — margin +0.4812 is within noise band.

- **PR#14898 `fastapi/openapi/models.py` hunk#186** (bpe=2.5441, thr=2.3689, margin=+0.1751)
  - reason: bpe
  - diff: `@@ -265,57 +265,57 @@ class Header(ParameterBase): ↵   ↵   ↵  class RequestBody(BaseModelWithConfig): ↵ -    description: Optional[str] = None ↵ +    description: str | None = None ↵      content: dict[str, Media`
  - Judgement: AMBIGUOUS — margin +0.1751 is within noise band.

- **PR#14898 `fastapi/routing.py` hunk#392** (bpe=3.0359, thr=2.3689, margin=+0.6669)
  - reason: bpe
  - diff: `@@ -168,10 +166,10 @@ async def __aenter__(self) -> _T: ↵   ↵      async def __aexit__( ↵          self, ↵ -        exc_type: Optional[type[BaseException]], ↵ -        exc_value: Optional[BaseException], ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.6669 is within noise band.

- **PR#14898 `fastapi/security/http.py` hunk#541** (bpe=3.2411, thr=2.3689, margin=+0.8722)
  - reason: bpe
  - diff: `@@ -1,6 +1,6 @@ ↵  import binascii ↵  from base64 import b64decode ↵ -from typing import Annotated, Optional ↵ +from typing import Annotated ↵   ↵  from annotated_doc import Doc ↵  from fastapi.exceptions import HT`
  - Judgement: AMBIGUOUS — margin +0.8722 is within noise band.

- **PR#14898 `fastapi/types.py` hunk#580** (bpe=2.5292, thr=2.3689, margin=+0.1603)
  - reason: bpe
  - diff: `@@ -1,11 +1,12 @@ ↵  import types ↵ +from collections.abc import Callable ↵  from enum import Enum ↵ -from typing import Any, Callable, Optional, TypeVar, Union ↵ +from typing import Any, TypeVar, Union ↵   ↵  from`
  - Judgement: AMBIGUOUS — margin +0.1603 is within noise band.

- **PR#14898 `fastapi/utils.py` hunk#581** (bpe=2.7338, thr=2.3689, margin=+0.3649)
  - reason: bpe
  - diff: `@@ -3,8 +3,7 @@ ↵  from typing import ( ↵      TYPE_CHECKING, ↵      Any, ↵ -    Optional, ↵ -    Union, ↵ +    Literal, ↵  ) ↵   ↵  import fastapi`
  - Judgement: AMBIGUOUS — margin +0.3649 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#7** (bpe=2.9206, thr=2.3682, margin=+0.5524)
  - reason: bpe
  - diff: `@@ -1,18 +1,19 @@ ↵  import dataclasses ↵  import inspect ↵  import sys ↵ -from collections.abc import Mapping, Sequence ↵ +from collections.abc import Callable, Mapping, Sequence ↵  from contextlib import AsyncE`
  - Judgement: AMBIGUOUS — margin +0.5524 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#10** (bpe=2.9206, thr=2.3682, margin=+0.5524)
  - reason: bpe
  - diff: `@@ -199,20 +199,17 @@ def get_flat_params(dependant: Dependant) -> list[ModelField]: ↵   ↵   ↵  def _get_signature(call: Callable[..., Any]) -> inspect.Signature: ↵ -    if sys.version_info >= (3, 10): ↵ -     `
  - Judgement: AMBIGUOUS — margin +0.5524 is within noise band.

- **PR#14897 `fastapi/dependencies/utils.py` hunk#20** (bpe=2.4760, thr=2.3682, margin=+0.1078)
  - reason: bpe
  - diff: `@@ -950,7 +947,7 @@ async def request_body_to_args( ↵          return {first_field.name: v_}, errors_ ↵      for field in body_fields: ↵          loc = ("body", get_validation_alias(field)) ↵ -        value: `
  - Judgement: AMBIGUOUS — margin +0.1078 is within noise band.

- **PR#14884 `fastapi/dependencies/utils.py` hunk#0** (bpe=2.9202, thr=2.3677, margin=+0.5524)
  - reason: bpe
  - diff: `@@ -1,7 +1,7 @@ ↵  import dataclasses ↵  import inspect ↵  import sys ↵ -from collections.abc import Coroutine, Mapping, Sequence ↵ +from collections.abc import Mapping, Sequence ↵  from contextlib import AsyncEx`
  - Judgement: AMBIGUOUS — margin +0.5524 is within noise band.

- **PR#14860 `fastapi/dependencies/utils.py` hunk#13** (bpe=2.4438, thr=2.3692, margin=+0.0745)
  - reason: bpe
  - diff: `@@ -1021,7 +1016,6 @@ def get_body_field( ↵      final_field = create_model_field( ↵          name="body", ↵          type_=BodyModel, ↵ -        required=required, ↵          alias="body", ↵          field_info=`
  - Judgement: AMBIGUOUS — margin +0.0745 is within noise band.

- **PR#14856 `fastapi/utils.py` hunk#6** (bpe=2.7406, thr=2.3746, margin=+0.3660)
  - reason: bpe
  - diff: `@@ -1,13 +1,11 @@ ↵  import re ↵  import warnings ↵ -from collections.abc import MutableMapping ↵  from typing import ( ↵      TYPE_CHECKING, ↵      Any, ↵      Optional, ↵      Union, ↵  ) ↵ -from weakref import WeakKey`
  - Judgement: AMBIGUOUS — margin +0.3660 is within noise band.

- **PR#14851 `fastapi/routing.py` hunk#4** (bpe=2.7627, thr=2.3817, margin=+0.3810)
  - reason: bpe
  - diff: `@@ -4473,6 +4570,58 @@ def trace_item(item_id: str): ↵              generate_unique_id_function=generate_unique_id_function, ↵          ) ↵   ↵ +    # TODO: remove this once the lifespan (or alternative) inte`
  - Judgement: AMBIGUOUS — margin +0.3810 is within noise band.

---

## §2 Rich — Per-threshold Flag Set Diff vs max

### p99 (new flags vs max)

No new flags introduced relative to max.

### p95 (new flags vs max)

No new flags introduced relative to max.

### p90 (new flags vs max)

No new flags introduced relative to max.

---

## §3 Recall — Per-threshold Catch Rates

| Threshold | Phase 1 catch rate (catalog breaks) | Phase 2 catch rate (stage2-only) |
|---|---|---|
| max | 93.5% | 100.0% |
| p99 | 96.8% | 100.0% |
| p95 | 100.0% | 100.0% |
| p90 | 100.0% | 100.0% |

Phase 1 gate: ≥50%. Phase 2 gate: ≥70%.

---

## §4 Verdict

**Winner: max**

Neither p99 nor p95 strictly dominates max on both dimensions. p99 new flags: 10, p95: 38, p90: 73. p90 recall: Phase1=100.0%, Phase2=100.0%. Keep max(cal_scores) — it is conservative and avoids introducing new false positives.

---

## §5 Honest Call-out

Threshold choice matters: p90 introduces 73 new FastAPI flags and shifts Phase1 recall by 6.5%. Adopt the winner and lock it in before the PR campaign.

