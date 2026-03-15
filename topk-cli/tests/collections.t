Setup:

  $ export TOPK_API_KEY=QZsw9LAxf47WZJXxoErASBptWqrBQZaFVaeNxYbhskVrY
  $ export TOPK_REGION=emulator TOPK_HOST=ddb TOPK_HTTPS=false
  $ export PATH="$TESTDIR/../.venv/bin:$PATH"
  $ P=t$$
  $ topk collections delete $P-col --yes 2>/dev/null || true
  $ topk collections delete $P-tmp --yes 2>/dev/null || true

List collections:

  $ topk collections list
  default-emulator

Create a collection:

  $ topk collections create $P-col --schema '{"title": "text:keyword", "year": "int"}' 2>&1
  Collection '*-col' created. (glob)

JSON output for create:

  $ topk --json collections create $P-tmp --schema '{"title": "text"}' | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['name'])"
  *-tmp (glob)

List includes new collections:

  $ topk collections list | sort | grep -v default
  *-col (glob)
  *-tmp (glob)

Get collection details as JSON:

  $ topk --json collections get $P-col | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['name'])"
  *-col (glob)

Piping auto-detects non-TTY and outputs names only:

  $ topk collections list | grep $P-col
  *-col (glob)

Delete:

  $ topk collections delete $P-col --yes 2>&1
  Collection '*-col' deleted. (glob)

Verify deleted:

  $ topk collections list | grep $P-col || echo "(none)"
  (none)

Non-interactive delete without --yes fails:

  $ echo | topk collections delete $P-tmp 2>&1
  Error: Use --yes to confirm deletion in non-interactive mode.
  [1]

Cleanup:

  $ topk collections delete $P-tmp --yes 2>/dev/null
