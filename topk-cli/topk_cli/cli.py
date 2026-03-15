import json
import mimetypes
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import typer
from topk_sdk import Client

from topk_cli.convert import to_plain
from topk_cli.schema import parse_schema


def _is_tty() -> bool:
    return sys.stdout.isatty()


def _wait_for_handle(ctx: typer.Context, ds, handle: str, timeout: int) -> None:
    # TODO: replace with ds.wait_for_handle() once topk-py exposes it
    ctx.obj.echo("Waiting for processing...")
    start = time.monotonic()
    deadline = start + timeout
    while True:
        status = ds.check_handle(handle)
        if status.processed:
            elapsed = time.monotonic() - start
            ctx.obj.echo(f"Done in {elapsed:.1f}s.")
            return
        if time.monotonic() >= deadline:
            ctx.obj.echo(f"Timed out after {timeout}s.")
            raise typer.Exit(1)
        time.sleep(1)


app = typer.Typer(help="TopK CLI — manage collections, datasets, and search")
collections_app = typer.Typer(help="Manage collections", invoke_without_command=True)
collection_app = typer.Typer(help="Operate on a single collection")
datasets_app = typer.Typer(help="Manage datasets", invoke_without_command=True)
dataset_app = typer.Typer(help="Operate on a single dataset")

app.add_typer(collections_app, name="collections")
app.add_typer(collection_app, name="collection")
app.add_typer(datasets_app, name="datasets")
app.add_typer(dataset_app, name="dataset")


@dataclass
class State:
    api_key: str | None = None
    region: str | None = None
    host: str = "topk.io"
    https: bool = True
    json_output: bool = False
    collection: str = ""
    dataset: str = ""

    def client(self) -> Client:
        if not self.api_key:
            typer.echo("Error: API key required. Set TOPK_API_KEY or use --api-key.", err=True)
            raise typer.Exit(1)
        if not self.region:
            typer.echo("Error: Region required. Set TOPK_REGION or use --region.", err=True)
            raise typer.Exit(1)
        return Client(
            api_key=self.api_key,
            region=self.region,
            host=self.host,
            https=self.https,
        )

    def print(self, obj, *, key: str | None = None):
        """Output SDK response.

        Modes:
        - --json: JSON to stdout
        - piped (non-TTY): key-only lines (for list commands), JSON otherwise
        - TTY (default): space-padded table for list-of-dicts, JSON for single objects
        """
        plain = to_plain(obj)
        if self.json_output:
            print(json.dumps(plain, indent=2, sort_keys=True, default=str))
            return
        if not _is_tty():
            if key:
                for item in (plain if isinstance(plain, list) else [plain]):
                    print(item[key] if isinstance(item, dict) else item)
                return
        if isinstance(plain, list) and (not plain or all(isinstance(item, dict) for item in plain)):
            _print_table(plain, key=key)
        else:
            print(json.dumps(plain, indent=2, sort_keys=True, default=str))

    def echo(self, msg):
        """Print a human message to stderr. Suppressed by --json."""
        if not self.json_output:
            typer.echo(msg, err=True)


def _print_table(items: list[dict], key: str | None = None):
    """Print a list of dicts as a space-padded table (gh/docker/kubectl style)."""
    if not items:
        typer.echo("(empty)", err=True)
        return
    all_keys = list(items[0].keys())
    keys = [k for k in all_keys if all(
        not isinstance(item.get(k), (dict, list)) for item in items
    )]
    if not keys:
        keys = all_keys
    headers = [k.upper() for k in keys]
    rows = [[str(item.get(k, "")) for k in keys] for item in items]
    widths = [max(len(h), *(len(r[i]) for r in rows)) for i, h in enumerate(headers)]
    print("   ".join(h.ljust(w) for h, w in zip(headers, widths)))
    for row in rows:
        print("   ".join(v.ljust(w) for v, w in zip(row, widths)))


def _parse_fields(value: str) -> list[str]:
    """Parse comma-separated field names."""
    return [f.strip() for f in value.split(",") if f.strip()]


def _parse_consistency(value: str) -> str:
    """Validate and return consistency level string."""
    if value not in ("indexed", "strong"):
        raise typer.BadParameter(f"Invalid consistency level: {value}. Use 'indexed' or 'strong'.")
    return value


def _parse_docs(value: str) -> list[dict]:
    """Parse JSON docs from a string, file path, or stdin (-)."""
    if value == "-":
        text = sys.stdin.read()
    elif value.startswith("@"):
        text = Path(value[1:]).read_text()
    else:
        text = value
    raw = json.loads(text)
    return raw if isinstance(raw, list) else [raw]


def _version_callback(value: bool):
    if value:
        from importlib.metadata import version
        print(f"topk {version('topk-cli')}")
        raise typer.Exit()


@app.callback(invoke_without_command=True)
def _global_options(
    ctx: typer.Context,
    api_key: str | None = typer.Option(None, "--api-key", envvar="TOPK_API_KEY", help="TopK API key"),
    region: str | None = typer.Option(None, "--region", envvar="TOPK_REGION", help="Region (e.g. aws-us-east-1)"),
    host: str = typer.Option("topk.io", "--host", envvar="TOPK_HOST", help="TopK host"),
    https: bool = typer.Option(True, "--https/--no-https", envvar="TOPK_HTTPS", help="Use HTTPS"),
    json_output: bool = typer.Option(False, "--json", "-j", help="Output as JSON"),
    version: bool = typer.Option(False, "--version", "-V", help="Show version", callback=_version_callback, is_eager=True),
):
    ctx.ensure_object(State)
    ctx.obj = State(
        api_key=api_key, region=region, host=host, https=https,
        json_output=json_output,
    )


# ── Collections ────────────────────────────────────────────────────────────────

@collections_app.command("list")
def collections_list(ctx: typer.Context):
    """List all collections."""
    client = ctx.obj.client()
    result = client.collections().list()
    ctx.obj.print(result, key="name")


@collections_app.command("create")
def collections_create(
    ctx: typer.Context,
    name: str = typer.Argument(help="Collection name"),
    schema_json: str = typer.Option(
        ..., "--schema", "-s",
        help='Schema JSON e.g. \'{"title": "text:semantic", "year": "int"}\'',
    ),
):
    """Create a collection."""
    client = ctx.obj.client()
    result = client.collections().create(name, schema=parse_schema(schema_json))
    if ctx.obj.json_output:
        ctx.obj.print(result)
    else:
        ctx.obj.echo(f"Collection '{name}' created.")


@collections_app.command("get")
def collections_get(
    ctx: typer.Context,
    name: str = typer.Argument(help="Collection name"),
):
    """Get collection details."""
    client = ctx.obj.client()
    result = client.collections().get(name)
    ctx.obj.print(result)


@collections_app.command("delete")
def collections_delete(
    ctx: typer.Context,
    name: str = typer.Argument(help="Collection name"),
    yes: bool = typer.Option(False, "--yes", "-y", help="Skip confirmation"),
):
    """Delete a collection."""
    if not yes:
        if not sys.stdin.isatty():
            typer.echo("Error: Use --yes to confirm deletion in non-interactive mode.", err=True)
            raise typer.Exit(1)
        typer.confirm(f"Delete collection '{name}'?", abort=True)
    client = ctx.obj.client()
    client.collections().delete(name)
    ctx.obj.echo(f"Collection '{name}' deleted.")


# ── Collection (single) ────────────────────────────────────────────────────────

@collection_app.callback(invoke_without_command=True)
def _collection_options(
    ctx: typer.Context,
    name: str = typer.Option(..., "--name", "-n", help="Collection name"),
):
    ctx.ensure_object(State)
    ctx.obj.collection = name


@collection_app.command("upsert")
def collection_upsert(
    ctx: typer.Context,
    data: list[dict] = typer.Argument(parser=_parse_docs, metavar="JSON", help="JSON doc(s), @path, or - for stdin"),
):
    """Upsert documents."""
    client = ctx.obj.client()
    result = client.collection(ctx.obj.collection).upsert(data)
    if ctx.obj.json_output:
        ctx.obj.print(result)
    else:
        ctx.obj.echo(f"Upserted {len(data)} document(s) into '{ctx.obj.collection}'.")


@collection_app.command("update")
def collection_update(
    ctx: typer.Context,
    data: list[dict] = typer.Argument(parser=_parse_docs, metavar="JSON", help="JSON doc(s), @path, or - for stdin"),
    fail_on_missing: bool | None = typer.Option(None, "--fail-on-missing", help="Fail if documents don't exist"),
):
    """Update documents (merge fields into existing docs)."""
    client = ctx.obj.client()
    result = client.collection(ctx.obj.collection).update(data, fail_on_missing=fail_on_missing)
    if ctx.obj.json_output:
        ctx.obj.print(result)
    else:
        ctx.obj.echo(f"Updated {len(data)} document(s) in '{ctx.obj.collection}'.")


@collection_app.command("get")
def collection_get(
    ctx: typer.Context,
    ids: list[str] = typer.Argument(help="Document IDs"),
    fields: str | None = typer.Option(None, "--fields", help="Comma-separated fields to return"),
    lsn: str | None = typer.Option(None, "--lsn", help="Read-your-writes LSN"),
    consistency: str | None = typer.Option(None, "--consistency", help="Consistency level (indexed, strong)"),
):
    """Get documents by ID."""
    client = ctx.obj.client()
    kwargs: dict = {}
    if fields:
        kwargs["fields"] = _parse_fields(fields)
    if lsn:
        kwargs["lsn"] = lsn
    if consistency:
        kwargs["consistency"] = _parse_consistency(consistency)
    result = client.collection(ctx.obj.collection).get(ids, **kwargs)
    ctx.obj.print(result)


@collection_app.command("delete")
def collection_delete(
    ctx: typer.Context,
    ids: list[str] = typer.Argument(help="Document IDs"),
):
    """Delete documents by ID."""
    client = ctx.obj.client()
    result = client.collection(ctx.obj.collection).delete(ids)
    if ctx.obj.json_output:
        ctx.obj.print(result)
    else:
        ctx.obj.echo(f"Deleted {len(ids)} document(s) from '{ctx.obj.collection}'.")


@collection_app.command("count")
def collection_count(
    ctx: typer.Context,
    lsn: str | None = typer.Option(None, "--lsn", help="Read-your-writes LSN"),
    consistency: str | None = typer.Option(None, "--consistency", help="Consistency level (indexed, strong)"),
):
    """Count documents."""
    client = ctx.obj.client()
    kwargs: dict = {}
    if lsn:
        kwargs["lsn"] = lsn
    if consistency:
        kwargs["consistency"] = _parse_consistency(consistency)
    count = client.collection(ctx.obj.collection).count(**kwargs)
    ctx.obj.print(count)


# ── Datasets ───────────────────────────────────────────────────────────────────

@datasets_app.command("list")
def datasets_list(ctx: typer.Context):
    """List all datasets."""
    client = ctx.obj.client()
    result = client.datasets().list()
    ctx.obj.print(result, key="name")


@datasets_app.command("create")
def datasets_create(
    ctx: typer.Context,
    name: str = typer.Argument(help="Dataset name"),
):
    """Create a dataset."""
    client = ctx.obj.client()
    result = client.datasets().create(name)
    if ctx.obj.json_output:
        ctx.obj.print(result)
    else:
        ctx.obj.echo(f"Dataset '{name}' created.")


@datasets_app.command("get")
def datasets_get(
    ctx: typer.Context,
    name: str = typer.Argument(help="Dataset name"),
):
    """Get dataset details."""
    client = ctx.obj.client()
    result = client.datasets().get(name)
    ctx.obj.print(result)


@datasets_app.command("delete")
def datasets_delete(
    ctx: typer.Context,
    name: str = typer.Argument(help="Dataset name"),
    yes: bool = typer.Option(False, "--yes", "-y", help="Skip confirmation"),
):
    """Delete a dataset."""
    if not yes:
        if not sys.stdin.isatty():
            typer.echo("Error: Use --yes to confirm deletion in non-interactive mode.", err=True)
            raise typer.Exit(1)
        typer.confirm(f"Delete dataset '{name}'?", abort=True)
    client = ctx.obj.client()
    result = client.datasets().delete(name)
    if ctx.obj.json_output:
        ctx.obj.print(result)
    else:
        ctx.obj.echo(f"Dataset '{name}' deleted.")


# ── Dataset (single) ───────────────────────────────────────────────────────────

@dataset_app.callback(invoke_without_command=True)
def _dataset_options(
    ctx: typer.Context,
    name: str = typer.Option(..., "--name", "-n", help="Dataset name"),
):
    ctx.ensure_object(State)
    ctx.obj.dataset = name


@dataset_app.command("list")
def dataset_list(
    ctx: typer.Context,
    filter: str | None = typer.Option(None, "--filter", "-f", help="JSON metadata filter"),
    fields: str | None = typer.Option(None, "--fields", help="Comma-separated metadata fields to return"),
):
    """List files in a dataset."""
    client = ctx.obj.client()
    ds = client.dataset(ctx.obj.dataset)
    kwargs: dict = {}
    if filter:
        kwargs["filter"] = json.loads(filter)
    if fields:
        kwargs["fields"] = _parse_fields(fields)
    result = list(ds.list(**kwargs))
    ctx.obj.print(result, key="id")


@dataset_app.command("upsert-file")
def dataset_upload(
    ctx: typer.Context,
    file_id: str = typer.Argument(help="File ID to assign"),
    file: Path = typer.Option(..., "--file", "-f", help="Path to file to upload"),
    metadata: str | None = typer.Option(None, "--metadata", "-m", help="JSON metadata"),
    wait: bool = typer.Option(False, "--wait", "-w", help="Wait for file to be processed"),
    timeout: int = typer.Option(900, "--timeout", "-t", help="Timeout in seconds for --wait"),
):
    """Upload a file to a dataset."""
    client = ctx.obj.client()
    ds = client.dataset(ctx.obj.dataset)
    meta = json.loads(metadata) if metadata else {}
    mime = mimetypes.guess_type(file.name)[0] or "application/octet-stream"
    result = ds.upsert_file(file_id, (file.name, file.read_bytes(), mime), meta)
    if wait:
        _wait_for_handle(ctx, ds, result.handle, timeout)
    ctx.obj.print(result)


@dataset_app.command("get-metadata")
def dataset_get_metadata(
    ctx: typer.Context,
    file_id: str = typer.Argument(help="File ID"),
    fields: str | None = typer.Option(None, "--fields", help="Comma-separated metadata fields to return"),
):
    """Get file metadata."""
    client = ctx.obj.client()
    kwargs: dict = {}
    if fields:
        kwargs["fields"] = _parse_fields(fields)
    result = client.dataset(ctx.obj.dataset).get_metadata(file_id, **kwargs)
    ctx.obj.print(result)


@dataset_app.command("update-metadata")
def dataset_update_metadata(
    ctx: typer.Context,
    file_id: str = typer.Argument(help="File ID"),
    metadata: str = typer.Option(..., "--metadata", "-m", help="JSON metadata"),
    wait: bool = typer.Option(False, "--wait", "-w", help="Wait for file to be processed"),
    timeout: int = typer.Option(900, "--timeout", "-t", help="Timeout in seconds for --wait"),
):
    """Update file metadata."""
    client = ctx.obj.client()
    ds = client.dataset(ctx.obj.dataset)
    result = ds.update_metadata(file_id, json.loads(metadata))
    if wait:
        _wait_for_handle(ctx, ds, result.handle, timeout)
    ctx.obj.print(result)


@dataset_app.command("delete")
def dataset_delete(
    ctx: typer.Context,
    file_id: str = typer.Argument(help="File ID"),
    wait: bool = typer.Option(False, "--wait", "-w", help="Wait for file to be processed"),
    timeout: int = typer.Option(900, "--timeout", "-t", help="Timeout in seconds for --wait"),
):
    """Delete a file from the dataset."""
    client = ctx.obj.client()
    ds = client.dataset(ctx.obj.dataset)
    result = ds.delete(file_id)
    if wait:
        _wait_for_handle(ctx, ds, result.handle, timeout)
    ctx.obj.print(result)


# ── Ask ────────────────────────────────────────────────────────────────────────

@app.command()
def ask(
    ctx: typer.Context,
    query: str = typer.Argument(help="Question to ask"),
    sources: list[str] = typer.Option(..., "--source", "-s", help="Dataset name to search (repeatable)"),
    mode: str | None = typer.Option(None, "--mode", help="Ask mode (summarize, reason, deep_research)"),
    fields: str | None = typer.Option(None, "--fields", help="Comma-separated fields to return"),
    # NOTE: --filter and --stream are not supported by the CLI
):
    """Ask a question over one or more datasets."""
    client = ctx.obj.client()
    kwargs: dict = {"query": query, "sources": sources}
    if mode:
        kwargs["mode"] = mode
    if fields:
        kwargs["select_fields"] = _parse_fields(fields)
    result = client.ask(**kwargs)
    ctx.obj.print(result)


# ── Search ─────────────────────────────────────────────────────────────────────

@app.command()
def search(
    ctx: typer.Context,
    query: str = typer.Argument(help="Search query"),
    sources: list[str] = typer.Option(..., "--source", "-s", help="Dataset name to search (repeatable)"),
    top_k: int = typer.Option(10, "--top-k", help="Number of results"),
    fields: str | None = typer.Option(None, "--fields", help="Comma-separated fields to return"),
    # NOTE: --filter and --stream are not supported by the CLI
):
    """Search over one or more datasets."""
    client = ctx.obj.client()
    kwargs: dict = {"query": query, "sources": sources, "top_k": top_k}
    if fields:
        kwargs["select_fields"] = _parse_fields(fields)
    result = client.search(**kwargs)
    ctx.obj.print(result)


# ── REPL ───────────────────────────────────────────────────────────────────────

@app.command(context_settings={"allow_extra_args": True, "ignore_unknown_options": True})
def repl(ctx: typer.Context):
    """Drop into an IPython shell with client pre-initialized."""
    try:
        import IPython
    except ImportError:
        typer.echo("IPython not found. Install it: pip install topk-cli[repl]", err=True)
        raise typer.Exit(1)

    import topk_sdk as t
    import topk_sdk.query as tq
    import topk_sdk.schema as ts

    client = ctx.obj.client()
    IPython.start_ipython(
        argv=ctx.args,
        user_ns={"client": client, "c": client, "t": t, "tq": tq, "ts": ts},
        display_banner=False,
    )


def main():
    try:
        app()
    except SystemExit:
        raise
    except Exception as e:
        typer.echo(f"Error: {e}", err=True)
        raise SystemExit(1)
