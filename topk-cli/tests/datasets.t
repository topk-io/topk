Setup:

  $ export TOPK_API_KEY=QZsw9LAxf47WZJXxoErASBptWqrBQZaFVaeNxYbhskVrY
  $ export TOPK_REGION=emulator TOPK_HOST=ddb TOPK_HTTPS=false
  $ export PATH="$TESTDIR/../.venv/bin:$PATH"
  $ P=t$$
  $ topk datasets delete $P-ds --yes 2>/dev/null || true
  $ topk datasets delete $P-tmp --yes 2>/dev/null || true

Create a dataset:

  $ topk datasets create $P-ds 2>&1
  Dataset '*-ds' created. (glob)

JSON output for create:

  $ topk --json datasets create $P-tmp | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['name'])"
  *-tmp (glob)

List includes new datasets:

  $ topk datasets list | grep $P-ds
  *-ds (glob)

Get dataset details:

  $ topk --json datasets get $P-ds | python3 -c "import sys,json; print(json.load(sys.stdin)['name'])"
  *-ds (glob)

Non-interactive delete without --yes fails:

  $ echo | topk datasets delete $P-tmp 2>&1
  Error: Use --yes to confirm deletion in non-interactive mode.
  [1]

JSON output for delete:

  $ topk --json datasets delete $P-tmp --yes | python3 -c "import sys,json; json.load(sys.stdin); print('ok')"
  ok

Delete:

  $ topk datasets delete $P-ds --yes 2>&1
  Dataset '*-ds' deleted. (glob)

Verify deleted:

  $ topk datasets list | grep $P-ds || echo "(none)"
  (none)
