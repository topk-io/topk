Setup:

  $ export TOPK_API_KEY=QZsw9LAxf47WZJXxoErASBptWqrBQZaFVaeNxYbhskVrY
  $ export TOPK_REGION=emulator TOPK_HOST=ddb TOPK_HTTPS=false
  $ export PATH="$TESTDIR/../.venv/bin:$PATH"
  $ P=t$$
  $ topk datasets create $P-files 2>/dev/null
  $ cat > "$CRAMTMP/sample.md" << 'EOF'
  > # Sample Document
  > The quick brown fox jumps over the lazy dog.
  > EOF

Upload a file with --wait:

  $ topk dataset --name $P-files upsert-file doc-1 --file "$CRAMTMP/sample.md" --metadata '{"source": "test", "type": "markdown"}' --wait --timeout 60 2>&1
  Waiting for processing...
  Done in *s. (glob)
  ".+" (re)

List files:

  $ topk dataset --name $P-files list | grep doc-1
  doc-1* (glob)

Get metadata:

  $ topk --json dataset --name $P-files get-metadata doc-1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['source'])"
  test

Get metadata with --fields:

  $ topk --json dataset --name $P-files get-metadata doc-1 --fields source | python3 -c "import sys,json; d=json.load(sys.stdin); print(sorted(k for k in d if k != 'id'))"
  ['source']

Update metadata with --wait:

  $ topk dataset --name $P-files update-metadata doc-1 --metadata '{"source": "test", "reviewed": true}' --wait --timeout 60 2>&1
  Waiting for processing...
  Done in *s. (glob)
  ".+" (re)

Verify metadata updated:

  $ sleep 2
  $ topk --json dataset --name $P-files get-metadata doc-1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('reviewed'))"
  True

Delete file with --wait:

  $ topk dataset --name $P-files delete doc-1 --wait --timeout 60 2>&1
  Waiting for processing...
  Done in *s. (glob)
  ".+" (re)

Cleanup:

  $ topk datasets delete $P-files --yes 2>/dev/null
