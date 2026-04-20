"""Phase 10 corpus analysis — per-category FastAPI idiom evidence.

Throwaway script: walks fastapi/, docs_src/, tests/ of the FastAPI clone and
collects structural evidence for eight scoring categories.
"""
# ruff: noqa: E501

from __future__ import annotations

import ast
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

CLONE_ROOT = Path("/tmp/argot-fastapi-static")
SUBTREES = ["fastapi", "docs_src", "tests"]
OUT_DIR = Path("/Users/damienmeur/projects/argot/docs/research/scoring/signal")
HEAD_SHA = "2fa00db8581bb4e74b2d00d859c8469b6da296c4"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def iter_py_files(subtree: str) -> list[Path]:
    root = CLONE_ROOT / subtree
    if not root.exists():
        return []
    return sorted(root.rglob("*.py"))


def relative(path: Path) -> str:
    return str(path.relative_to(CLONE_ROOT))


def get_func_name(node: ast.expr) -> str:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return f"{get_func_name(node.value)}.{node.attr}"
    return "<unknown>"


def is_endpoint_decorated(func: ast.FunctionDef | ast.AsyncFunctionDef) -> bool:
    """Heuristic: function has a decorator like @app.get / @router.post / etc."""
    http_methods = {"get", "post", "put", "delete", "patch", "head", "options", "trace"}
    for dec in func.decorator_list:
        if isinstance(dec, ast.Call) and isinstance(dec.func, ast.Attribute) and dec.func.attr in http_methods:
            return True
    return False


# ---------------------------------------------------------------------------
# Category collectors
# ---------------------------------------------------------------------------


@dataclass
class ExceptionHandlingResult:
    raise_nodes: Counter[str] = field(default_factory=Counter)  # exc class → count
    raise_http_exception: int = 0
    exception_handler_decorators: int = 0
    json_response_error: int = 0
    citations_raise_http: list[str] = field(default_factory=list)
    citations_exc_handler: list[str] = field(default_factory=list)
    citations_json_err: list[str] = field(default_factory=list)


@dataclass
class AsyncBlockingResult:
    blocking_calls: Counter[str] = field(default_factory=Counter)
    blocking_citations: dict[str, list[str]] = field(default_factory=lambda: defaultdict(list))
    httpx_async_client: int = 0
    anyio_to_thread: int = 0
    httpx_citations: list[str] = field(default_factory=list)
    anyio_citations: list[str] = field(default_factory=list)


@dataclass
class DependencyInjectionResult:
    depends_count: int = 0
    generator_deps: int = 0
    annotated_count: int = 0
    module_singletons: int = 0
    depends_citations: list[str] = field(default_factory=list)
    annotated_citations: list[str] = field(default_factory=list)
    generator_citations: list[str] = field(default_factory=list)


@dataclass
class BackgroundTasksResult:
    background_tasks_param: int = 0
    add_task_calls: int = 0
    endpoint_asyncio_create_task: int = 0
    endpoint_thread: int = 0
    endpoint_run_in_executor: int = 0
    bt_param_citations: list[str] = field(default_factory=list)
    add_task_citations: list[str] = field(default_factory=list)


@dataclass
class SerializationResult:
    response_model_kw: int = 0
    json_response_content: int = 0
    json_response_content_in_endpoint: int = 0
    jsonable_encoder_total: int = 0
    jsonable_encoder_in_endpoint: int = 0
    orjson_imports: int = 0
    ujson_imports: int = 0
    response_model_citations: list[str] = field(default_factory=list)
    jsonable_encoder_citations: list[str] = field(default_factory=list)


@dataclass
class RoutingResult:
    decorator_counts: Counter[str] = field(default_factory=Counter)
    add_api_route_calls: int = 0
    app_mount_calls: int = 0
    include_router_calls: int = 0
    asynccontextmanager_sites: int = 0
    decorator_citations: dict[str, list[str]] = field(default_factory=lambda: defaultdict(list))
    include_router_citations: list[str] = field(default_factory=list)


@dataclass
class ValidationResult:
    base_model_subclasses: int = 0
    field_validator_sites: int = 0
    query_param_defaults: int = 0
    path_param_defaults: int = 0
    body_param_defaults: int = 0
    manual_isinstance_raise: int = 0
    base_model_citations: list[str] = field(default_factory=list)
    field_validator_citations: list[str] = field(default_factory=list)
    query_citations: list[str] = field(default_factory=list)


@dataclass
class DownstreamHttpResult:
    httpx_sites: int = 0
    requests_sites_non_test: int = 0
    requests_sites_test: int = 0
    raise_for_status_calls: int = 0
    httpx_citations: list[str] = field(default_factory=list)
    requests_citations: list[str] = field(default_factory=list)
    raise_for_status_citations: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# AST visitor
# ---------------------------------------------------------------------------


class CorpusVisitor(ast.NodeVisitor):
    def __init__(self, filepath: Path, is_test: bool) -> None:
        self.filepath = filepath
        self.is_test = is_test
        self.rel = relative(filepath)

        self.exc = ExceptionHandlingResult()
        self.async_b = AsyncBlockingResult()
        self.di = DependencyInjectionResult()
        self.bt = BackgroundTasksResult()
        self.ser = SerializationResult()
        self.routing = RoutingResult()
        self.val = ValidationResult()
        self.ds = DownstreamHttpResult()

        # Track depends args for generator detection
        self._depends_args: list[str] = []
        # Track functions with yield
        self._yielding_functions: set[str] = set()
        # Scope tracking
        self._in_async_func: list[bool] = []
        self._in_endpoint: list[bool] = []
        self._module_level = True  # set to False once inside a function

    def _loc(self, node: ast.AST) -> str:
        return f"{self.rel}:{getattr(node, 'lineno', '?')}"

    # ---- module-level singletons ----
    def visit_Module(self, node: ast.Module) -> None:  # noqa: N802
        for stmt in node.body:
            if isinstance(stmt, ast.Assign) and isinstance(stmt.value, ast.Call):
                self.di.module_singletons += 1
        self.generic_visit(node)

    # ---- imports ----
    def visit_Import(self, node: ast.Import) -> None:  # noqa: N802
        for alias in node.names:
            if "orjson" in alias.name:
                self.ser.orjson_imports += 1
            if "ujson" in alias.name:
                self.ser.ujson_imports += 1
            if alias.name.startswith("httpx"):
                if len(self.ds.httpx_citations) < 3:
                    self.ds.httpx_citations.append(self._loc(node))
                self.ds.httpx_sites += 1
            if alias.name.startswith("requests"):
                if self.is_test:
                    self.ds.requests_sites_test += 1
                else:
                    self.ds.requests_sites_non_test += 1
                    if len(self.ds.requests_citations) < 3:
                        self.ds.requests_citations.append(self._loc(node))

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:  # noqa: N802
        mod = node.module or ""
        if "orjson" in mod:
            self.ser.orjson_imports += 1
        if "ujson" in mod:
            self.ser.ujson_imports += 1
        if mod.startswith("httpx") or mod == "httpx":
            self.ds.httpx_sites += 1
            if len(self.ds.httpx_citations) < 3:
                self.ds.httpx_citations.append(self._loc(node))
        if mod.startswith("requests"):
            if self.is_test:
                self.ds.requests_sites_test += 1
            else:
                self.ds.requests_sites_non_test += 1

    # ---- class defs ----
    def visit_ClassDef(self, node: ast.ClassDef) -> None:  # noqa: N802
        for base in node.bases:
            name = get_func_name(base)
            if "BaseModel" in name:
                self.val.base_model_subclasses += 1
                if len(self.val.base_model_citations) < 3:
                    self.val.base_model_citations.append(f"{self._loc(node)} — class {node.name}(BaseModel)")
                break
        self.generic_visit(node)

    # ---- function defs ----
    def _visit_func(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        is_async = isinstance(node, ast.AsyncFunctionDef)
        is_endpoint = is_endpoint_decorated(node)
        self._in_async_func.append(is_async)
        self._in_endpoint.append(is_endpoint)

        # Decorators on function
        for dec in node.decorator_list:
            # @app.exception_handler / @exception_handler
            if isinstance(dec, ast.Call):
                fname = get_func_name(dec.func)
                if "exception_handler" in fname:
                    self.exc.exception_handler_decorators += 1
                    if len(self.exc.citations_exc_handler) < 3:
                        self.exc.citations_exc_handler.append(f"{self._loc(dec)} — {fname}(...)")
                # @field_validator
                if "field_validator" in fname or "validator" in fname:
                    self.val.field_validator_sites += 1
                    if len(self.val.field_validator_citations) < 3:
                        self.val.field_validator_citations.append(f"{self._loc(dec)} — {fname}(...)")
                # response_model= kw on endpoint decorators
                for kw in dec.keywords:
                    if kw.arg == "response_model":
                        self.ser.response_model_kw += 1
                        if len(self.ser.response_model_citations) < 3:
                            self.ser.response_model_citations.append(
                                f"{self._loc(dec)} — {get_func_name(dec.func)}(response_model=...)"
                            )
                # Routing decorator counts
                if isinstance(dec.func, ast.Attribute):
                    obj = get_func_name(dec.func.value) if isinstance(dec.func.value, ast.expr) else "?"
                    meth = dec.func.attr
                    key = f"@{obj}.{meth}"
                    self.routing.decorator_counts[key] += 1
                    if len(self.routing.decorator_citations[key]) < 3:
                        self.routing.decorator_citations[key].append(f"{self._loc(dec)} — {key}")
                # @asynccontextmanager
                if isinstance(dec, ast.Call) and get_func_name(dec.func) == "asynccontextmanager":
                    self.routing.asynccontextmanager_sites += 1
            elif isinstance(dec, ast.Name):
                if dec.id == "asynccontextmanager":
                    self.routing.asynccontextmanager_sites += 1
                if "field_validator" in dec.id or dec.id == "validator":
                    self.val.field_validator_sites += 1
            elif isinstance(dec, ast.Attribute):
                key = f"@{get_func_name(dec)}"
                self.routing.decorator_counts[key] += 1

        # BackgroundTasks parameter
        for arg in node.args.args + node.args.posonlyargs + node.args.kwonlyargs:
            ann = arg.annotation
            if ann is not None:
                ann_str = ast.unparse(ann)
                if "BackgroundTasks" in ann_str:
                    self.bt.background_tasks_param += 1
                    if len(self.bt.bt_param_citations) < 3:
                        self.bt.bt_param_citations.append(
                            f"{self._loc(node)} — def {node.name}(..., {arg.arg}: BackgroundTasks)"
                        )
                # Depends in annotations
                if "Depends" in ann_str:
                    self.di.depends_count += 1
                # Annotated usage
                if "Annotated" in ann_str:
                    self.di.annotated_count += 1
                    if len(self.di.annotated_citations) < 3:
                        self.di.annotated_citations.append(f"{self._loc(node)} — {arg.arg}: {ann_str[:60]}")
                # Query/Path/Body
                if "Query(" in ann_str or ann_str.startswith("Query"):
                    self.val.query_param_defaults += 1
                    if len(self.val.query_citations) < 3:
                        self.val.query_citations.append(f"{self._loc(node)} — {arg.arg}: {ann_str[:60]}")
                if "Path(" in ann_str:
                    self.val.path_param_defaults += 1
                if "Body(" in ann_str:
                    self.val.body_param_defaults += 1

        # Check for yield (generator dep)
        for child in ast.walk(node):
            if isinstance(child, ast.Yield | ast.YieldFrom):
                self._yielding_functions.add(node.name)
                break

        self.generic_visit(node)
        self._in_async_func.pop()
        self._in_endpoint.pop()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:  # noqa: N802
        self._visit_func(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:  # noqa: N802
        self._visit_func(node)

    # ---- raises ----
    def visit_Raise(self, node: ast.Raise) -> None:  # noqa: N802
        if node.exc is not None:
            exc_node = node.exc
            if isinstance(exc_node, ast.Call):
                exc_node = exc_node.func
            name = get_func_name(exc_node)
            top_name = name.split(".")[-1]
            self.exc.raise_nodes[top_name] += 1
            if "HTTPException" in top_name:
                self.exc.raise_http_exception += 1
                if len(self.exc.citations_raise_http) < 3:
                    self.exc.citations_raise_http.append(f"{self._loc(node)} — raise HTTPException(...)")
        self.generic_visit(node)

    # ---- calls ----
    def visit_Call(self, node: ast.Call) -> None:  # noqa: N802
        fname = get_func_name(node.func)
        loc = self._loc(node)
        in_async = bool(self._in_async_func) and self._in_async_func[-1]
        in_endpoint = bool(self._in_endpoint) and self._in_endpoint[-1]

        # --- exception handling ---
        if fname == "JSONResponse" or fname.endswith(".JSONResponse"):
            # check first arg is dict with "error" key
            if node.args:
                first_arg = node.args[0]
                if isinstance(first_arg, ast.Dict):
                    for key in first_arg.keys:
                        if isinstance(key, ast.Constant) and key.value == "error":
                            self.exc.json_response_error += 1
                            if len(self.exc.citations_json_err) < 3:
                                self.exc.citations_json_err.append(f'{loc} — JSONResponse({{"error": ...}})')
            # serialization: JSONResponse(content=...)
            for kw in node.keywords:
                if kw.arg == "content":
                    self.ser.json_response_content += 1
                    if in_endpoint:
                        self.ser.json_response_content_in_endpoint += 1

        # --- async blocking ---
        blocking_patterns = {
            "time.sleep", "requests.get", "requests.post",
            "requests.Session", "urlopen",
        }
        for bp in blocking_patterns:
            if fname == bp:
                if in_async:
                    self.async_b.blocking_calls[bp] += 1
                    if len(self.async_b.blocking_citations[bp]) < 3:
                        self.async_b.blocking_citations[bp].append(loc)
                # count globally too
                if not in_async:
                    self.async_b.blocking_calls[f"{bp}(sync)"] += 1

        if "query" in fname.lower() and fname.endswith(".query") and in_async:
            self.async_b.blocking_calls["orm.query"] += 1

        # httpx.AsyncClient
        if "AsyncClient" in fname and "httpx" in fname:
            self.async_b.httpx_async_client += 1
            if len(self.async_b.httpx_citations) < 3:
                self.async_b.httpx_citations.append(loc)

        # anyio.to_thread
        if "to_thread" in fname and "anyio" in fname:
            self.async_b.anyio_to_thread += 1
            if len(self.async_b.anyio_citations) < 3:
                self.async_b.anyio_citations.append(loc)

        # --- dependency injection ---
        if fname == "Depends" or fname.endswith(".Depends"):
            self.di.depends_count += 1
            if len(self.di.depends_citations) < 3:
                self.di.depends_citations.append(loc)
            # check if arg is a yielding function
            if node.args:
                arg0 = node.args[0]
                if isinstance(arg0, ast.Name) and arg0.id in self._yielding_functions:
                    self.di.generator_deps += 1
                    if len(self.di.generator_citations) < 3:
                        self.di.generator_citations.append(f"{loc} — Depends({arg0.id}) [generator]")

        # --- background tasks ---
        if fname == "background_tasks.add_task" or (
            isinstance(node.func, ast.Attribute) and node.func.attr == "add_task"
        ):
            self.bt.add_task_calls += 1
            if len(self.bt.add_task_citations) < 3:
                self.bt.add_task_citations.append(loc)

        if in_endpoint:
            if "create_task" in fname:
                self.bt.endpoint_asyncio_create_task += 1
            if fname == "Thread" or fname.endswith(".Thread"):
                self.bt.endpoint_thread += 1
            if "run_in_executor" in fname:
                self.bt.endpoint_run_in_executor += 1

        # --- serialization ---
        if fname == "jsonable_encoder" or fname.endswith(".jsonable_encoder"):
            self.ser.jsonable_encoder_total += 1
            if in_endpoint:
                self.ser.jsonable_encoder_in_endpoint += 1
            if len(self.ser.jsonable_encoder_citations) < 3:
                self.ser.jsonable_encoder_citations.append(loc)

        if fname == "orjson.dumps" or fname == "orjson.loads":
            self.ser.orjson_imports += 0  # just usage, imports counted above

        # --- routing ---
        if fname == "add_api_route" or fname.endswith(".add_api_route"):
            self.routing.add_api_route_calls += 1
        if fname == "app.mount" or (isinstance(node.func, ast.Attribute) and node.func.attr == "mount"):
            self.routing.app_mount_calls += 1
        if "include_router" in fname:
            self.routing.include_router_calls += 1
            if len(self.routing.include_router_citations) < 3:
                self.routing.include_router_citations.append(loc)

        # --- validation ---
        if fname in {"Query", "Path", "Body"}:
            if fname == "Query":
                self.val.query_param_defaults += 1
                if len(self.val.query_citations) < 3:
                    self.val.query_citations.append(loc)
            elif fname == "Path":
                self.val.path_param_defaults += 1
            elif fname == "Body":
                self.val.body_param_defaults += 1

        # --- downstream http ---
        if "raise_for_status" in fname:
            self.ds.raise_for_status_calls += 1
            if len(self.ds.raise_for_status_citations) < 3:
                self.ds.raise_for_status_citations.append(loc)

        if fname.startswith("httpx."):
            self.ds.httpx_sites += 1

        if fname.startswith("requests."):
            if self.is_test:
                self.ds.requests_sites_test += 1
            else:
                self.ds.requests_sites_non_test += 1

        self.generic_visit(node)

    # --- manual isinstance+raise in endpoint scope ---
    def visit_If(self, node: ast.If) -> None:  # noqa: N802
        in_endpoint = bool(self._in_endpoint) and self._in_endpoint[-1]
        if in_endpoint:
            test = node.test
            is_isinstance_check = False
            if isinstance(test, ast.UnaryOp) and isinstance(test.op, ast.Not):
                inner = test.operand
                if isinstance(inner, ast.Call) and get_func_name(inner.func) == "isinstance":
                    is_isinstance_check = True
            elif isinstance(test, ast.Call) and get_func_name(test.func) == "isinstance":
                is_isinstance_check = True
            if is_isinstance_check:
                for child in ast.walk(node):
                    if isinstance(child, ast.Raise):
                        self.val.manual_isinstance_raise += 1
                        break
        self.generic_visit(node)


# ---------------------------------------------------------------------------
# Aggregator
# ---------------------------------------------------------------------------


@dataclass
class CorpusStats:
    exc: ExceptionHandlingResult = field(default_factory=ExceptionHandlingResult)
    async_b: AsyncBlockingResult = field(default_factory=AsyncBlockingResult)
    di: DependencyInjectionResult = field(default_factory=DependencyInjectionResult)
    bt: BackgroundTasksResult = field(default_factory=BackgroundTasksResult)
    ser: SerializationResult = field(default_factory=SerializationResult)
    routing: RoutingResult = field(default_factory=RoutingResult)
    val: ValidationResult = field(default_factory=ValidationResult)
    ds: DownstreamHttpResult = field(default_factory=DownstreamHttpResult)
    files_parsed: int = 0
    parse_errors: int = 0


def merge(stats: CorpusStats, visitor: CorpusVisitor) -> None:  # noqa: PLR0912, PLR0915
    v = visitor

    # exception_handling
    for k, c in v.exc.raise_nodes.items():
        stats.exc.raise_nodes[k] += c
    stats.exc.raise_http_exception += v.exc.raise_http_exception
    stats.exc.exception_handler_decorators += v.exc.exception_handler_decorators
    stats.exc.json_response_error += v.exc.json_response_error
    if len(stats.exc.citations_raise_http) < 3:
        stats.exc.citations_raise_http.extend(v.exc.citations_raise_http[: 3 - len(stats.exc.citations_raise_http)])
    if len(stats.exc.citations_exc_handler) < 3:
        stats.exc.citations_exc_handler.extend(
            v.exc.citations_exc_handler[: 3 - len(stats.exc.citations_exc_handler)]
        )
    if len(stats.exc.citations_json_err) < 3:
        stats.exc.citations_json_err.extend(v.exc.citations_json_err[: 3 - len(stats.exc.citations_json_err)])

    # async_blocking
    for k, c in v.async_b.blocking_calls.items():
        stats.async_b.blocking_calls[k] += c
    for k, cits in v.async_b.blocking_citations.items():
        stats.async_b.blocking_citations[k].extend(cits[: 3 - len(stats.async_b.blocking_citations[k])])
    stats.async_b.httpx_async_client += v.async_b.httpx_async_client
    stats.async_b.anyio_to_thread += v.async_b.anyio_to_thread
    if len(stats.async_b.httpx_citations) < 3:
        stats.async_b.httpx_citations.extend(v.async_b.httpx_citations[: 3 - len(stats.async_b.httpx_citations)])
    if len(stats.async_b.anyio_citations) < 3:
        stats.async_b.anyio_citations.extend(v.async_b.anyio_citations[: 3 - len(stats.async_b.anyio_citations)])

    # dependency_injection
    stats.di.depends_count += v.di.depends_count
    stats.di.generator_deps += v.di.generator_deps
    stats.di.annotated_count += v.di.annotated_count
    stats.di.module_singletons += v.di.module_singletons
    if len(stats.di.depends_citations) < 3:
        stats.di.depends_citations.extend(v.di.depends_citations[: 3 - len(stats.di.depends_citations)])
    if len(stats.di.annotated_citations) < 3:
        stats.di.annotated_citations.extend(v.di.annotated_citations[: 3 - len(stats.di.annotated_citations)])
    if len(stats.di.generator_citations) < 3:
        stats.di.generator_citations.extend(v.di.generator_citations[: 3 - len(stats.di.generator_citations)])

    # background_tasks
    stats.bt.background_tasks_param += v.bt.background_tasks_param
    stats.bt.add_task_calls += v.bt.add_task_calls
    stats.bt.endpoint_asyncio_create_task += v.bt.endpoint_asyncio_create_task
    stats.bt.endpoint_thread += v.bt.endpoint_thread
    stats.bt.endpoint_run_in_executor += v.bt.endpoint_run_in_executor
    if len(stats.bt.bt_param_citations) < 3:
        stats.bt.bt_param_citations.extend(v.bt.bt_param_citations[: 3 - len(stats.bt.bt_param_citations)])
    if len(stats.bt.add_task_citations) < 3:
        stats.bt.add_task_citations.extend(v.bt.add_task_citations[: 3 - len(stats.bt.add_task_citations)])

    # serialization
    stats.ser.response_model_kw += v.ser.response_model_kw
    stats.ser.json_response_content += v.ser.json_response_content
    stats.ser.json_response_content_in_endpoint += v.ser.json_response_content_in_endpoint
    stats.ser.jsonable_encoder_total += v.ser.jsonable_encoder_total
    stats.ser.jsonable_encoder_in_endpoint += v.ser.jsonable_encoder_in_endpoint
    stats.ser.orjson_imports += v.ser.orjson_imports
    stats.ser.ujson_imports += v.ser.ujson_imports
    if len(stats.ser.response_model_citations) < 3:
        stats.ser.response_model_citations.extend(
            v.ser.response_model_citations[: 3 - len(stats.ser.response_model_citations)]
        )
    if len(stats.ser.jsonable_encoder_citations) < 3:
        stats.ser.jsonable_encoder_citations.extend(
            v.ser.jsonable_encoder_citations[: 3 - len(stats.ser.jsonable_encoder_citations)]
        )

    # routing
    for k, c in v.routing.decorator_counts.items():
        stats.routing.decorator_counts[k] += c
    for k, cits in v.routing.decorator_citations.items():
        stats.routing.decorator_citations[k].extend(cits[: 3 - len(stats.routing.decorator_citations[k])])
    stats.routing.add_api_route_calls += v.routing.add_api_route_calls
    stats.routing.app_mount_calls += v.routing.app_mount_calls
    stats.routing.include_router_calls += v.routing.include_router_calls
    stats.routing.asynccontextmanager_sites += v.routing.asynccontextmanager_sites
    if len(stats.routing.include_router_citations) < 3:
        stats.routing.include_router_citations.extend(
            v.routing.include_router_citations[: 3 - len(stats.routing.include_router_citations)]
        )

    # validation
    stats.val.base_model_subclasses += v.val.base_model_subclasses
    stats.val.field_validator_sites += v.val.field_validator_sites
    stats.val.query_param_defaults += v.val.query_param_defaults
    stats.val.path_param_defaults += v.val.path_param_defaults
    stats.val.body_param_defaults += v.val.body_param_defaults
    stats.val.manual_isinstance_raise += v.val.manual_isinstance_raise
    if len(stats.val.base_model_citations) < 3:
        stats.val.base_model_citations.extend(v.val.base_model_citations[: 3 - len(stats.val.base_model_citations)])
    if len(stats.val.field_validator_citations) < 3:
        stats.val.field_validator_citations.extend(
            v.val.field_validator_citations[: 3 - len(stats.val.field_validator_citations)]
        )
    if len(stats.val.query_citations) < 3:
        stats.val.query_citations.extend(v.val.query_citations[: 3 - len(stats.val.query_citations)])

    # downstream_http
    stats.ds.httpx_sites += v.ds.httpx_sites
    stats.ds.requests_sites_non_test += v.ds.requests_sites_non_test
    stats.ds.requests_sites_test += v.ds.requests_sites_test
    stats.ds.raise_for_status_calls += v.ds.raise_for_status_calls
    if len(stats.ds.httpx_citations) < 3:
        stats.ds.httpx_citations.extend(v.ds.httpx_citations[: 3 - len(stats.ds.httpx_citations)])
    if len(stats.ds.requests_citations) < 3:
        stats.ds.requests_citations.extend(v.ds.requests_citations[: 3 - len(stats.ds.requests_citations)])
    if len(stats.ds.raise_for_status_citations) < 3:
        stats.ds.raise_for_status_citations.extend(
            v.ds.raise_for_status_citations[: 3 - len(stats.ds.raise_for_status_citations)]
        )


# ---------------------------------------------------------------------------
# Main analysis loop
# ---------------------------------------------------------------------------


def analyse() -> CorpusStats:
    stats = CorpusStats()
    for subtree in SUBTREES:
        for path in iter_py_files(subtree):
            is_test = subtree == "tests" or "test" in path.name
            try:
                source = path.read_text(encoding="utf-8", errors="replace")
                tree = ast.parse(source, filename=str(path))
            except SyntaxError:
                stats.parse_errors += 1
                continue
            visitor = CorpusVisitor(path, is_test)
            visitor.visit(tree)
            merge(stats, visitor)
            stats.files_parsed += 1
    return stats


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------


def top10(counter: Counter[str]) -> list[tuple[str, int]]:
    return counter.most_common(10)


def fmt_citations(cits: list[str]) -> str:
    if not cits:
        return "_none found_"
    return "\n".join(f"- `{c}`" for c in cits)


def fmt_table(rows: list[tuple[str, int]]) -> str:
    if not rows:
        return "_none_"
    lines = ["| Pattern | Count |", "|---------|-------|"]
    for pattern, count in rows:
        lines.append(f"| `{pattern}` | {count} |")
    return "\n".join(lines)


def generate_report(stats: CorpusStats) -> str:  # noqa: PLR0915
    s = stats
    lines: list[str] = []

    lines.append("# Phase 10 — FastAPI Corpus Analysis (2026-04-21)\n")
    lines.append(f"FastAPI clone: `/tmp/argot-fastapi-static` (HEAD SHA: `{HEAD_SHA}`)")
    lines.append("Analysis date: 2026-04-21\n")

    lines.append("## Summary\n")
    lines.append(
        f"Corpus: **{s.files_parsed}** Python files parsed across `fastapi/`, `docs_src/`, and `tests/` "
        f"({s.parse_errors} parse errors).\n"
        "**Rich signal:** `validation` (BaseModel subclasses, field_validator, Query/Path/Body), "
        "`dependency_injection` (Depends sites, Annotated style), `routing` (decorator distribution, "
        "include_router), and `exception_handling` (raise HTTPException dominates). "
        "**Moderate signal:** `serialization` (response_model, jsonable_encoder) and "
        "`downstream_http` (httpx vs requests split across test/non-test). "
        "**Sparse signal:** `background_tasks` (narrow API — BackgroundTasks param + add_task only) and "
        "`async_blocking` (almost no blocking calls inside async functions, as expected for a clean async "
        "codebase). `framework_swap` overlaps heavily with routing.\n"
    )

    # --- exception_handling ---
    lines.append("## exception_handling\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.exc.citations_raise_http) or "_none_")
    lines.append("\n### Feature frequency table\n")
    exc_table: list[tuple[str, int]] = [
        ("raise HTTPException(...)", s.exc.raise_http_exception),
        ("@app.exception_handler(...)", s.exc.exception_handler_decorators),
        ('JSONResponse({"error": ...})', s.exc.json_response_error),
    ]
    exc_table += [(k, v) for k, v in top10(s.exc.raise_nodes) if k not in {"HTTPException"}][:7]
    lines.append(fmt_table(exc_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        "- **raise HTTPException vs register exception_handler**: "
        f"{s.exc.raise_http_exception} inline raises vs {s.exc.exception_handler_decorators} handler registrations — "
        "strong separator between 'handle-inline' and 'handle-globally' idioms.\n"
        "- **JSONResponse({'error': ...}) vs HTTPException**: "
        f"{s.exc.json_response_error} manual JSON error responses — early/legacy pattern vs modern HTTPException.\n"
        "- **Exception class diversity**: custom exception classes (non-HTTPException raises) indicate "
        "app-level error design sophistication.\n"
    )

    # --- async_blocking ---
    lines.append("## async_blocking\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    all_blocking_cits = []
    for bp in ["time.sleep", "requests.get", "requests.post", "urlopen"]:
        all_blocking_cits.extend(stats.async_b.blocking_citations.get(bp, []))
    lines.append(fmt_citations(all_blocking_cits[:3] or s.async_b.anyio_citations[:3]))
    lines.append("\n### Feature frequency table\n")
    async_table: list[tuple[str, int]] = [
        ("httpx.AsyncClient usage", s.async_b.httpx_async_client),
        ("anyio.to_thread usage", s.async_b.anyio_to_thread),
    ] + top10(s.async_b.blocking_calls)
    lines.append(fmt_table(async_table[:10]))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        "- **httpx.AsyncClient present vs absent**: "
        f"{s.async_b.httpx_async_client} AsyncClient usages — strongest positive signal for 'async-aware downstream'.\n"
        "- **anyio.to_thread.run_sync present**: "
        f"{s.async_b.anyio_to_thread} sites — marks intentional thread offloading, not accidental blocking.\n"
        "- **blocking call inside async def**: near-zero in this corpus (clean codebase), so presence = strong negative signal.\n"
    )

    # --- dependency_injection ---
    lines.append("## dependency_injection\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.di.depends_citations))
    lines.append("\n### Feature frequency table\n")
    di_table: list[tuple[str, int]] = [
        ("Depends(...) call sites", s.di.depends_count),
        ("Annotated[ usage", s.di.annotated_count),
        ("generator deps (yield in Depends arg)", s.di.generator_deps),
        ("module-level singleton assignments", s.di.module_singletons),
    ]
    lines.append(fmt_table(di_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        f"- **Depends count**: {s.di.depends_count} sites — raw count correlates with DI adoption depth.\n"
        f"- **Annotated vs bare Depends**: {s.di.annotated_count} Annotated usages — modern (3.9+ / FastAPI 0.95+) "
        "style marker; break between legacy and idiomatic.\n"
        f"- **Generator deps**: {s.di.generator_deps} generator-style deps — resource-managing pattern "
        "(DB sessions, HTTP clients).\n"
    )

    # --- background_tasks ---
    lines.append("## background_tasks\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.bt.bt_param_citations or s.bt.add_task_citations))
    lines.append("\n### Feature frequency table\n")
    bt_table: list[tuple[str, int]] = [
        ("BackgroundTasks parameter", s.bt.background_tasks_param),
        ("background_tasks.add_task(...)", s.bt.add_task_calls),
        ("asyncio.create_task (in endpoint)", s.bt.endpoint_asyncio_create_task),
        ("Thread (in endpoint)", s.bt.endpoint_thread),
        ("loop.run_in_executor (in endpoint)", s.bt.endpoint_run_in_executor),
    ]
    lines.append(fmt_table(bt_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        "- **BackgroundTasks + add_task**: the canonical pair; "
        f"{s.bt.background_tasks_param} param usages / {s.bt.add_task_calls} add_task calls.\n"
        "- **asyncio.create_task in endpoint**: "
        f"{s.bt.endpoint_asyncio_create_task} occurrences — non-idiomatic, leaks task lifecycle.\n"
        "- **Thread in endpoint**: "
        f"{s.bt.endpoint_thread} occurrences — synchronous offload antipattern, strong negative signal.\n"
        "- Key break: `BackgroundTasks` (canonical) vs `asyncio.create_task` / `Thread` (antipattern) vs nothing.\n"
    )

    # --- serialization ---
    lines.append("## serialization\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.ser.response_model_citations))
    lines.append("\n### Feature frequency table\n")
    ser_table: list[tuple[str, int]] = [
        ("response_model= on decorator", s.ser.response_model_kw),
        ("jsonable_encoder() total", s.ser.jsonable_encoder_total),
        ("jsonable_encoder() inside endpoint", s.ser.jsonable_encoder_in_endpoint),
        ("JSONResponse(content=...) total", s.ser.json_response_content),
        ("JSONResponse(content=...) in endpoint", s.ser.json_response_content_in_endpoint),
        ("orjson import sites", s.ser.orjson_imports),
        ("ujson import sites", s.ser.ujson_imports),
    ]
    lines.append(fmt_table(ser_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        f"- **response_model= present**: {s.ser.response_model_kw} usages — primary model-driven serialization axis.\n"
        f"- **jsonable_encoder inside endpoint**: {s.ser.jsonable_encoder_in_endpoint} — "
        "manual serialization step, indicates custom output shaping.\n"
        f"- **orjson vs default**: {s.ser.orjson_imports} orjson imports — performance optimization marker; "
        "almost absent means default encoder dominates.\n"
        "- **JSONResponse(content=) vs response_model**: explicit response construction vs declarative schema.\n"
    )

    # --- routing ---
    lines.append("## routing\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    top_dec = s.routing.decorator_counts.most_common(1)
    top_dec_key = top_dec[0][0] if top_dec else "?"
    lines.append(fmt_citations(s.routing.decorator_citations.get(top_dec_key, [])[:3]))
    lines.append("\n### Feature frequency table\n")
    routing_table: list[tuple[str, int]] = list(s.routing.decorator_counts.most_common(10))
    routing_table += [
        ("add_api_route() imperative", s.routing.add_api_route_calls),
        ("app.mount()", s.routing.app_mount_calls),
        ("include_router()", s.routing.include_router_calls),
        ("@asynccontextmanager", s.routing.asynccontextmanager_sites),
    ]
    lines.append(fmt_table(routing_table[:14]))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        "- **@router.* vs @app.***: use of APIRouter signals modular app structure.\n"
        f"- **include_router count**: {s.routing.include_router_calls} — "
        "proxy for app decomposition into multiple routers.\n"
        f"- **add_api_route imperative**: {s.routing.add_api_route_calls} — programmatic vs declarative routing.\n"
        f"- **app.mount**: {s.routing.app_mount_calls} — sub-application mounting (ASGI composition).\n"
    )

    # --- framework_swap (synthesized from routing) ---
    lines.append("## framework_swap\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.routing.include_router_citations))
    lines.append("\n### Feature frequency table\n")
    fw_table: list[tuple[str, int]] = [
        ("include_router() calls", s.routing.include_router_calls),
        ("app.mount() calls", s.routing.app_mount_calls),
        ("add_api_route() calls", s.routing.add_api_route_calls),
        ("@asynccontextmanager (lifespan)", s.routing.asynccontextmanager_sites),
    ]
    lines.append(fmt_table(fw_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        "- **framework_swap overlaps heavily with routing**: both are concerned with application composition "
        "and lifecycle. The distinguishing axis for framework_swap is app-level structural patterns:\n"
        f"  - `app.mount()` ({s.routing.app_mount_calls}) — ASGI sub-app composition\n"
        f"  - `@asynccontextmanager` ({s.routing.asynccontextmanager_sites}) — lifespan event handling\n"
        "  - Presence of multiple APIRouter modules vs a single flat app\n"
        "- **add_api_route imperative**: strongly indicates dynamic/generated routing (e.g., plugin system).\n"
    )

    # --- validation ---
    lines.append("## validation\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.val.base_model_citations))
    lines.append("\n### Feature frequency table\n")
    val_table: list[tuple[str, int]] = [
        ("BaseModel subclasses", s.val.base_model_subclasses),
        ("@field_validator sites", s.val.field_validator_sites),
        ("Query() param defaults", s.val.query_param_defaults),
        ("Path() param defaults", s.val.path_param_defaults),
        ("Body() param defaults", s.val.body_param_defaults),
        ("manual isinstance+raise in endpoint", s.val.manual_isinstance_raise),
    ]
    lines.append(fmt_table(val_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        f"- **BaseModel count**: {s.val.base_model_subclasses} subclasses — "
        "raw schema coverage; higher = more declarative validation.\n"
        f"- **@field_validator**: {s.val.field_validator_sites} sites — custom validation logic beyond type checks.\n"
        f"- **Query/Path/Body defaults**: {s.val.query_param_defaults}/{s.val.path_param_defaults}/{s.val.body_param_defaults} — "
        "parameter-level validation (constraints, descriptions).\n"
        f"- **manual isinstance+raise**: {s.val.manual_isinstance_raise} — imperative fallback, negative signal.\n"
    )

    # --- downstream_http ---
    lines.append("## downstream_http\n")
    lines.append("### Canonical idiom (top 3 citations)\n")
    lines.append(fmt_citations(s.ds.httpx_citations))
    lines.append("\n### Feature frequency table\n")
    ds_table: list[tuple[str, int]] = [
        ("httpx usage sites", s.ds.httpx_sites),
        ("requests usage (non-test files)", s.ds.requests_sites_non_test),
        ("requests usage (test files)", s.ds.requests_sites_test),
        ("raise_for_status() calls", s.ds.raise_for_status_calls),
    ]
    lines.append(fmt_table(ds_table))
    lines.append("\n### Candidate break axes\n")
    lines.append(
        f"- **httpx vs requests in non-test code**: {s.ds.httpx_sites} httpx vs "
        f"{s.ds.requests_sites_non_test} requests (non-test) — "
        "httpx is the async-compatible choice; requests in production code = blocking smell.\n"
        f"- **raise_for_status()**: {s.ds.raise_for_status_calls} sites — "
        "explicit error propagation from HTTP calls.\n"
        "- **test files**: requests is expected in test clients (TestClient is requests-based); "
        f"{s.ds.requests_sites_test} test-file usages should be excluded from scoring.\n"
    )

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    print("Starting corpus analysis...", flush=True)
    stats = analyse()
    print(f"Parsed {stats.files_parsed} files ({stats.parse_errors} errors)", flush=True)

    report = generate_report(stats)

    # Print to stdout
    print("\n" + "=" * 80)
    print(report)
    print("=" * 80)

    # Write to file
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path = OUT_DIR / "phase10_corpus_analysis_2026-04-21.md"
    out_path.write_text(report, encoding="utf-8")
    print(f"\nReport written to: {out_path}", flush=True)


if __name__ == "__main__":
    main()
