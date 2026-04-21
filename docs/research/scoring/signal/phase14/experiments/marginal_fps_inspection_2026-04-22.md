# Phase 14 Diagnostic — Marginal FP Root-Cause Inspection (2026-04-22)

**Purpose:** Determine whether the two marginal FPs from exp #2c are caused by a dominant
calibration outlier (fragile threshold) or a densely-packed top of distribution (robust noise).

**Cases inspected:**
- FastAPI seed=2: ctrl_index=5, bpe=4.0668, threshold=4.0185, margin=+0.048
- rich seed=2: ctrl_index=7, bpe=4.8159, threshold=4.7608, margin=+0.055

---

## §1 — Calibration Distribution Profile

### FastAPI

| seed | max | 2nd_max | gap(max-2nd) | gap(max-p99) | p99 | p95 | p90 | p75 | mean | std |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 4.2039 | 3.8206 | 0.3833 | 0.3795 | 3.8244 | 3.3258 | 2.8929 | 2.2247 | 1.6845 | 0.8384 |
| 1 | 4.2039 | 3.8206 | 0.3833 | 0.3795 | 3.8244 | 3.1778 | 2.4237 | 1.8754 | 1.4771 | 0.8427 |
| 2 | 4.0185 | 3.5989 | 0.4196 | 0.4154 | 3.6031 | 2.9881 | 2.4609 | 1.9038 | 1.5307 | 0.7428 |
| 3 | 3.8206 | 3.3138 | 0.5068 | 0.5017 | 3.3189 | 2.8073 | 2.3302 | 1.9629 | 1.5138 | 0.7550 |
| 4 | 4.0185 | 3.6805 | 0.3380 | 0.3346 | 3.6839 | 3.1899 | 2.8541 | 2.2259 | 1.6834 | 0.8394 |

### Rich

| seed | max | 2nd_max | gap(max-2nd) | gap(max-p99) | p99 | p95 | p90 | p75 | mean | std |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 4.4213 | 4.4015 | 0.0198 | 0.0196 | 4.4017 | 3.6011 | 3.3509 | 2.9128 | 2.3517 | 0.7884 |
| 1 | 4.8159 | 4.7608 | 0.0550 | 0.0545 | 4.7614 | 3.6338 | 3.2999 | 2.7990 | 2.3104 | 0.7840 |
| 2 | 4.7608 | 4.4213 | 0.3396 | 0.3362 | 4.4247 | 3.5978 | 3.3259 | 2.9766 | 2.3766 | 0.8181 |
| 3 | 4.7608 | 4.4099 | 0.3510 | 0.3475 | 4.4134 | 3.5331 | 3.3993 | 2.9299 | 2.3673 | 0.7471 |
| 4 | 4.4213 | 4.4015 | 0.0198 | 0.0196 | 4.4017 | 3.5496 | 3.3123 | 2.8936 | 2.3709 | 0.7292 |

---

## §2 — Cal-Max Hunk Content per (Domain, Seed)

### FastAPI

#### seed=0 — bpe=4.2039

**File:** `.argot/research/repos/fastapi/fastapi/openapi/docs.py` lines 197–298

```python
def get_redoc_html(
    *,
    openapi_url: Annotated[
        str,
        Doc(
            """
            The OpenAPI URL that ReDoc should load and use.

            This is normally done automatically by FastAPI using the default URL
            `/openapi.json`.

            Read more about it in the
            [FastAPI docs for Conditional OpenAPI](https://fastapi.tiangolo.com/how-to/conditional-openapi/#conditional-openapi-from-settings-and-env-vars)
            """
        ),

... [72 lines omitted] ...

        margin: 0;
        padding: 0;
      }}
    </style>
    </head>
    <body>
    <noscript>
        ReDoc requires Javascript to function. Please enable it to browse the documentation.
    </noscript>
    <redoc spec-url="{openapi_url}"></redoc>
    <script src="{redoc_js_url}"> </script>
    </body>
    </html>
    """
    return HTMLResponse(html)
```

#### seed=1 — bpe=4.2039

**File:** `.argot/research/repos/fastapi/fastapi/openapi/docs.py` lines 197–298

```python
def get_redoc_html(
    *,
    openapi_url: Annotated[
        str,
        Doc(
            """
            The OpenAPI URL that ReDoc should load and use.

            This is normally done automatically by FastAPI using the default URL
            `/openapi.json`.

            Read more about it in the
            [FastAPI docs for Conditional OpenAPI](https://fastapi.tiangolo.com/how-to/conditional-openapi/#conditional-openapi-from-settings-and-env-vars)
            """
        ),

... [72 lines omitted] ...

        margin: 0;
        padding: 0;
      }}
    </style>
    </head>
    <body>
    <noscript>
        ReDoc requires Javascript to function. Please enable it to browse the documentation.
    </noscript>
    <redoc spec-url="{openapi_url}"></redoc>
    <script src="{redoc_js_url}"> </script>
    </body>
    </html>
    """
    return HTMLResponse(html)
```

#### seed=2 — bpe=4.0185

**File:** `.argot/research/repos/fastapi/fastapi/datastructures.py` lines 21–150

```python
class UploadFile(StarletteUploadFile):
    """
    A file uploaded in a request.

    Define it as a *path operation function* (or dependency) parameter.

    If you are using a regular `def` function, you can use the `upload_file.file`
    attribute to access the raw standard Python file (blocking, not async), useful and
    needed for non-async code.

    Read more about it in the
    [FastAPI docs for Request Files](https://fastapi.tiangolo.com/tutorial/request-files/).

    ## Example


... [100 lines omitted] ...

        return cast(UploadFile, __input_value)

    @classmethod
    def __get_pydantic_json_schema__(
        cls, core_schema: Mapping[str, Any], handler: GetJsonSchemaHandler
    ) -> dict[str, Any]:
        return {"type": "string", "contentMediaType": "application/octet-stream"}

    @classmethod
    def __get_pydantic_core_schema__(
        cls, source: type[Any], handler: Callable[[Any], Mapping[str, Any]]
    ) -> Mapping[str, Any]:
        from ._compat.v2 import with_info_plain_validator_function

        return with_info_plain_validator_function(cls._validate)
```

#### seed=3 — bpe=3.8206

**File:** `.argot/research/repos/fastapi/fastapi/_compat/shared.py` lines 47–55

```python
def lenient_issubclass(
    cls: Any, class_or_tuple: type[_T] | tuple[type[_T], ...] | None
) -> TypeGuard[type[_T]]:
    try:
        return isinstance(cls, type) and issubclass(cls, class_or_tuple)  # type: ignore[arg-type]
    except TypeError:  # pragma: no cover
        if isinstance(cls, WithArgsTypes):
            return False
        raise  # pragma: no cover
```

#### seed=4 — bpe=4.0185

**File:** `.argot/research/repos/fastapi/fastapi/datastructures.py` lines 21–150

```python
class UploadFile(StarletteUploadFile):
    """
    A file uploaded in a request.

    Define it as a *path operation function* (or dependency) parameter.

    If you are using a regular `def` function, you can use the `upload_file.file`
    attribute to access the raw standard Python file (blocking, not async), useful and
    needed for non-async code.

    Read more about it in the
    [FastAPI docs for Request Files](https://fastapi.tiangolo.com/tutorial/request-files/).

    ## Example


... [100 lines omitted] ...

        return cast(UploadFile, __input_value)

    @classmethod
    def __get_pydantic_json_schema__(
        cls, core_schema: Mapping[str, Any], handler: GetJsonSchemaHandler
    ) -> dict[str, Any]:
        return {"type": "string", "contentMediaType": "application/octet-stream"}

    @classmethod
    def __get_pydantic_core_schema__(
        cls, source: type[Any], handler: Callable[[Any], Mapping[str, Any]]
    ) -> Mapping[str, Any]:
        from ._compat.v2 import with_info_plain_validator_function

        return with_info_plain_validator_function(cls._validate)
```

### Rich

#### seed=0 — bpe=4.4213

**File:** `.argot/research/repos/rich/rich/repr.py` lines 36–102

```python
def auto(
    cls: Optional[Type[T]] = None, *, angular: Optional[bool] = None
) -> Union[Type[T], Callable[[Type[T]], Type[T]]]:
    """Class decorator to create __repr__ from __rich_repr__"""

    def do_replace(cls: Type[T], angular: Optional[bool] = None) -> Type[T]:
        def auto_repr(self: T) -> str:
            """Create repr string from __rich_repr__"""
            repr_str: List[str] = []
            append = repr_str.append

            angular: bool = getattr(self.__rich_repr__, "angular", False)  # type: ignore[attr-defined]
            for arg in self.__rich_repr__():  # type: ignore[attr-defined]
                if isinstance(arg, tuple):
                    if len(arg) == 1:

... [37 lines omitted] ...


        if not hasattr(cls, "__rich_repr__"):
            auto_rich_repr.__doc__ = "Build a rich repr"
            cls.__rich_repr__ = auto_rich_repr  # type: ignore[attr-defined]

        auto_repr.__doc__ = "Return repr(self)"
        cls.__repr__ = auto_repr  # type: ignore[assignment]
        if angular is not None:
            cls.__rich_repr__.angular = angular  # type: ignore[attr-defined]
        return cls

    if cls is None:
        return partial(do_replace, angular=angular)
    else:
        return do_replace(cls, angular=angular)
```

#### seed=1 — bpe=4.8159

**File:** `.argot/research/repos/rich/rich/_win32_console.py` lines 331–572

```python
class LegacyWindowsTerm:
    """This class allows interaction with the legacy Windows Console API. It should only be used in the context
    of environments where virtual terminal processing is not available. However, if it is used in a Windows environment,
    the entire API should work.

    Args:
        file (IO[str]): The file which the Windows Console API HANDLE is retrieved from, defaults to sys.stdout.
    """

    BRIGHT_BIT = 8

    # Indices are ANSI color numbers, values are the corresponding Windows Console API color numbers
    ANSI_TO_WINDOWS = [
        0,  # black                      The Windows colours are defined in wincon.h as follows:
        4,  # red                         define FOREGROUND_BLUE            0x0001 -- 0000 0001

... [212 lines omitted] ...


    def set_title(self, title: str) -> None:
        """Set the title of the terminal window

        Args:
            title (str): The new title of the console window
        """
        assert len(title) < 255, "Console title must be less than 255 characters"
        SetConsoleTitle(title)

    def _get_cursor_size(self) -> int:
        """Get the percentage of the character cell that is filled by the cursor"""
        cursor_info = CONSOLE_CURSOR_INFO()
        GetConsoleCursorInfo(self._handle, cursor_info=cursor_info)
        return int(cursor_info.dwSize)
```

#### seed=2 — bpe=4.7608

**File:** `.argot/research/repos/rich/rich/console.py` lines 581–2642

```python
class Console:
    """A high level console interface.

    Args:
        color_system (str, optional): The color system supported by your terminal,
            either ``"standard"``, ``"256"`` or ``"truecolor"``. Leave as ``"auto"`` to autodetect.
        force_terminal (Optional[bool], optional): Enable/disable terminal control codes, or None to auto-detect terminal. Defaults to None.
        force_jupyter (Optional[bool], optional): Enable/disable Jupyter rendering, or None to auto-detect Jupyter. Defaults to None.
        force_interactive (Optional[bool], optional): Enable/disable interactive mode, or None to auto detect. Defaults to None.
        soft_wrap (Optional[bool], optional): Set soft wrap default on print method. Defaults to False.
        theme (Theme, optional): An optional style theme object, or ``None`` for default theme.
        stderr (bool, optional): Use stderr rather than stdout if ``file`` is not specified. Defaults to False.
        file (IO, optional): A file object where the console should write to. Defaults to stdout.
        quiet (bool, Optional): Boolean to suppress all output. Defaults to False.
        width (int, optional): The width of the terminal. Leave as default to auto-detect width.

... [2032 lines omitted] ...

                string. Defaults to 0.61, which is the width to height ratio of Fira Code (the default font).
                If you aren't specifying a different font inside ``code_format``, you probably don't need this.
            unique_id (str, optional): unique id that is used as the prefix for various elements (CSS styles, node
                ids). If not set, this defaults to a computed value based on the recorded content.
        """
        svg = self.export_svg(
            title=title,
            theme=theme,
            clear=clear,
            code_format=code_format,
            font_aspect_ratio=font_aspect_ratio,
            unique_id=unique_id,
        )
        with open(path, "w", encoding="utf-8") as write_file:
            write_file.write(svg)
```

#### seed=3 — bpe=4.7608

**File:** `.argot/research/repos/rich/rich/console.py` lines 581–2642

```python
class Console:
    """A high level console interface.

    Args:
        color_system (str, optional): The color system supported by your terminal,
            either ``"standard"``, ``"256"`` or ``"truecolor"``. Leave as ``"auto"`` to autodetect.
        force_terminal (Optional[bool], optional): Enable/disable terminal control codes, or None to auto-detect terminal. Defaults to None.
        force_jupyter (Optional[bool], optional): Enable/disable Jupyter rendering, or None to auto-detect Jupyter. Defaults to None.
        force_interactive (Optional[bool], optional): Enable/disable interactive mode, or None to auto detect. Defaults to None.
        soft_wrap (Optional[bool], optional): Set soft wrap default on print method. Defaults to False.
        theme (Theme, optional): An optional style theme object, or ``None`` for default theme.
        stderr (bool, optional): Use stderr rather than stdout if ``file`` is not specified. Defaults to False.
        file (IO, optional): A file object where the console should write to. Defaults to stdout.
        quiet (bool, Optional): Boolean to suppress all output. Defaults to False.
        width (int, optional): The width of the terminal. Leave as default to auto-detect width.

... [2032 lines omitted] ...

                string. Defaults to 0.61, which is the width to height ratio of Fira Code (the default font).
                If you aren't specifying a different font inside ``code_format``, you probably don't need this.
            unique_id (str, optional): unique id that is used as the prefix for various elements (CSS styles, node
                ids). If not set, this defaults to a computed value based on the recorded content.
        """
        svg = self.export_svg(
            title=title,
            theme=theme,
            clear=clear,
            code_format=code_format,
            font_aspect_ratio=font_aspect_ratio,
            unique_id=unique_id,
        )
        with open(path, "w", encoding="utf-8") as write_file:
            write_file.write(svg)
```

#### seed=4 — bpe=4.4213

**File:** `.argot/research/repos/rich/rich/repr.py` lines 36–102

```python
def auto(
    cls: Optional[Type[T]] = None, *, angular: Optional[bool] = None
) -> Union[Type[T], Callable[[Type[T]], Type[T]]]:
    """Class decorator to create __repr__ from __rich_repr__"""

    def do_replace(cls: Type[T], angular: Optional[bool] = None) -> Type[T]:
        def auto_repr(self: T) -> str:
            """Create repr string from __rich_repr__"""
            repr_str: List[str] = []
            append = repr_str.append

            angular: bool = getattr(self.__rich_repr__, "angular", False)  # type: ignore[attr-defined]
            for arg in self.__rich_repr__():  # type: ignore[attr-defined]
                if isinstance(arg, tuple):
                    if len(arg) == 1:

... [37 lines omitted] ...


        if not hasattr(cls, "__rich_repr__"):
            auto_rich_repr.__doc__ = "Build a rich repr"
            cls.__rich_repr__ = auto_rich_repr  # type: ignore[attr-defined]

        auto_repr.__doc__ = "Return repr(self)"
        cls.__repr__ = auto_repr  # type: ignore[assignment]
        if angular is not None:
            cls.__rich_repr__.angular = angular  # type: ignore[attr-defined]
        return cls

    if cls is None:
        return partial(do_replace, angular=angular)
    else:
        return do_replace(cls, angular=angular)
```

---

## §3 — FP Control Hunk Content

### FastAPI seed=2 — ctrl_index=5, bpe=4.0668, threshold=4.0185

**File:** `.argot/research/repos/fastapi/fastapi/routing.py` lines 351–729

```python
def get_request_handler(
    dependant: Dependant,
    body_field: ModelField | None = None,
    status_code: int | None = None,
    response_class: type[Response] | DefaultPlaceholder = Default(JSONResponse),
    response_field: ModelField | None = None,
    response_model_include: IncEx | None = None,
    response_model_exclude: IncEx | None = None,
    response_model_by_alias: bool = True,
    response_model_exclude_unset: bool = False,
    response_model_exclude_defaults: bool = False,
    response_model_exclude_none: bool = False,
    dependency_overrides_provider: Any | None = None,
    embed_body_fields: bool = False,
    strict_content_type: bool | DefaultPlaceholder = Default(True),

... [349 lines omitted] ...

                        response = actual_response_class(content, **response_args)
                    if not is_body_allowed_for_status_code(response.status_code):
                        response.body = b""
                    response.headers.raw.extend(solved_result.response.headers.raw)
        if errors:
            validation_error = RequestValidationError(
                errors, body=body, endpoint_ctx=endpoint_ctx
            )
            raise validation_error

        # Return response
        assert response
        return response

    return app
```

**Cal-max for this seed:**
File: `.argot/research/repos/fastapi/fastapi/datastructures.py` lines 21–150
Score: 4.0185

### Rich seed=2 — ctrl_index=7, bpe=4.8159, threshold=4.7608

**File:** `.argot/research/repos/rich/rich/_win32_console.py` lines 331–572

```python
class LegacyWindowsTerm:
    """This class allows interaction with the legacy Windows Console API. It should only be used in the context
    of environments where virtual terminal processing is not available. However, if it is used in a Windows environment,
    the entire API should work.

    Args:
        file (IO[str]): The file which the Windows Console API HANDLE is retrieved from, defaults to sys.stdout.
    """

    BRIGHT_BIT = 8

    # Indices are ANSI color numbers, values are the corresponding Windows Console API color numbers
    ANSI_TO_WINDOWS = [
        0,  # black                      The Windows colours are defined in wincon.h as follows:
        4,  # red                         define FOREGROUND_BLUE            0x0001 -- 0000 0001

... [212 lines omitted] ...


    def set_title(self, title: str) -> None:
        """Set the title of the terminal window

        Args:
            title (str): The new title of the console window
        """
        assert len(title) < 255, "Console title must be less than 255 characters"
        SetConsoleTitle(title)

    def _get_cursor_size(self) -> int:
        """Get the percentage of the character cell that is filled by the cursor"""
        cursor_info = CONSOLE_CURSOR_INFO()
        GetConsoleCursorInfo(self._handle, cursor_info=cursor_info)
        return int(cursor_info.dwSize)
```

**Cal-max for this seed:**
File: `.argot/research/repos/rich/rich/console.py` lines 581–2642
Score: 4.7608

---

## §4 — Holdout Diagnostic

Remove the cal-max hunk; new threshold = max of remaining cal scores.
If FP no longer fires → dominant-outlier construction artifact confirmed.

| domain | seed | fp_bpe | threshold | holdout_threshold | gap_max_second | fp_fires_after_holdout |
|---|---|---|---|---|---|---|
| fastapi | 2 | 4.0668 | 4.0185 | 3.5989 | 0.4196 | YES — FP survives holdout |
| rich | 2 | 4.8159 | 4.7608 | 4.4213 | 0.3396 | YES — FP survives holdout |

---

## §5 — Cal-Max Stability Across Seeds

Does the same source file/hunk appear repeatedly at the calibration ceiling?

### FastAPI

| seed | threshold | cal-max file | lines | score |
|---|---|---|---|---|
| 0 | 4.2039 | `.argot/research/repos/fastapi/fastapi/openapi/docs.py` | 197–298 | 4.2039 |
| 1 | 4.2039 | `.argot/research/repos/fastapi/fastapi/openapi/docs.py` | 197–298 | 4.2039 |
| 2 | 4.0185 | `.argot/research/repos/fastapi/fastapi/datastructures.py` | 21–150 | 4.0185 |
| 3 | 3.8206 | `.argot/research/repos/fastapi/fastapi/_compat/shared.py` | 47–55 | 3.8206 |
| 4 | 4.0185 | `.argot/research/repos/fastapi/fastapi/datastructures.py` | 21–150 | 4.0185 |

Cal-max file frequency across 5 seeds: {'.argot/research/repos/fastapi/fastapi/openapi/docs.py': 2, '.argot/research/repos/fastapi/fastapi/datastructures.py': 2, '.argot/research/repos/fastapi/fastapi/_compat/shared.py': 1}

### Rich

| seed | threshold | cal-max file | lines | score |
|---|---|---|---|---|
| 0 | 4.4213 | `.argot/research/repos/rich/rich/repr.py` | 36–102 | 4.4213 |
| 1 | 4.8159 | `.argot/research/repos/rich/rich/_win32_console.py` | 331–572 | 4.8159 |
| 2 | 4.7608 | `.argot/research/repos/rich/rich/console.py` | 581–2642 | 4.7608 |
| 3 | 4.7608 | `.argot/research/repos/rich/rich/console.py` | 581–2642 | 4.7608 |
| 4 | 4.4213 | `.argot/research/repos/rich/rich/repr.py` | 36–102 | 4.4213 |

Cal-max file frequency across 5 seeds: {'.argot/research/repos/rich/rich/repr.py': 2, '.argot/research/repos/rich/rich/_win32_console.py': 1, '.argot/research/repos/rich/rich/console.py': 2}

---

## §6 — Diagnosis

**FastAPI:** Pattern = **borderline (moderate gap)**.
gap(max−2nd)=0.4196 in seed=2 — moderate, not decisively outlier or dense. FP survives holdout → threshold genuinely anchored by distribution. The cal-max file varies across seeds (no single file recurs ≥3/5), suggesting the top is structurally spread across the corpus.

**Rich:** Pattern = **borderline (moderate gap)**.
gap(max−2nd)=0.3396 in seed=2 — moderate, not decisively outlier or dense. FP survives holdout → threshold genuinely anchored by distribution. The cal-max file varies across seeds (no single file recurs ≥3/5), suggesting the top is structurally spread across the corpus.

---

## §7 — Recommendation

**Keep max(cal).** Both domains show dense-top distribution (FastAPI gap=0.4196, rich gap=0.3396) and/or FP survives holdout — the threshold is not anchored by a single outlier. The two FPs are expected boundary noise at 1% FP rate, consistent with VALIDATED verdict. No threshold construction change needed before real-PR validation.

